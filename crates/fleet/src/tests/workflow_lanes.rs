//! A workflow's lanes as its editor draws them, worked out by Fleet: `crate::job_lanes::editor_lanes` and
//! `get_workflow_lanes`. **The cases are `editorLanes`' in `LaneCanvas/build.ts` at the default level**, read
//! off the pieces as `workspaceOf` and `placesOf` in Bridge's `lifecycle-editing/` read them: the lanes
//! the stories draw, then each way a rule is drawn or left out, then which piece of an id is the default's.

use std::sync::Arc;

use axum::http::StatusCode;
use ipc::{
    AppliedLifecycle, AppliedRule, JobLanesView, LaneAutonomy, LaneKind, LaneNodeKind, LaneNodeRole, LaneOutcome, LanePending,
    LifecycleDefinition, LifecycleRule, LifecycleStateDefinition, LifecycleTransition, PieceKept, PieceRank, PieceScope, RuleAct,
    RuleAmong, RuleCondition, RuleDoing, RuleOnFailure, RuleTarget, RuleWhen, StateReadFrom,
};

use crate::job_lanes::{editor_lanes, Drafts, EditorInput, PiecesRead, RepositoryRead, WorkspaceRead};
use crate::tests::http::call;
use crate::tests::repositories::{a_fleet_reading, code, served, Planted};
use crate::tests::tmp::TempDir;
use crate::tests::workflow_page::{read, staged_in_repository};

fn kept(reach: &str, source: &str) -> PieceKept {
    PieceKept {
        rank: PieceRank { reach: reach.into(), source: source.into(), home: false },
        path: (source != "armada").then(|| format!("/{source}/lifecycles/piece.yml")),
        overridden_by: None,
        hides: None,
    }
}

fn armadas(lifecycle: LifecycleDefinition) -> AppliedLifecycle {
    AppliedLifecycle { lifecycle, kept: kept("everywhere", "armada") }
}

fn state(id: &str, label: &str, read: StateReadFrom) -> LifecycleStateDefinition {
    LifecycleStateDefinition { id: id.into(), label: label.into(), read }
}

fn tracker(id: &str, label: &str) -> LifecycleStateDefinition {
    state(id, label, StateReadFrom::Tracker { words: vec![label.into()] })
}

fn moves_of(pairs: &[(&str, &str)]) -> Vec<LifecycleTransition> {
    pairs.iter().map(|(from, to)| LifecycleTransition { from: (*from).into(), to: (*to).into() }).collect()
}

/// The stories' ticket, with a move back from review that leads nowhere new.
fn ticket() -> LifecycleDefinition {
    LifecycleDefinition {
        id: "ticket".into(),
        name: "Ticket".into(),
        scope: PieceScope::Everywhere,
        states: vec![tracker("todo", "To do"), tracker("ready", "Ready"), tracker("in_progress", "In progress"), tracker("done", "Done")],
        transitions: moves_of(&[("todo", "ready"), ("todo", "in_progress"), ("ready", "in_progress"), ("in_progress", "todo"), ("in_progress", "done")]),
        ends: vec!["done".into()],
    }
}

fn pull() -> LifecycleDefinition {
    let forge = |id: &str, label: &str| state(id, label, StateReadFrom::Forge { fact: id.into() });
    LifecycleDefinition {
        id: "pull_request".into(),
        name: "PR".into(),
        scope: PieceScope::Everywhere,
        states: vec![forge("draft", "Draft"), forge("ready", "Ready"), forge("merged", "Merged")],
        transitions: moves_of(&[("draft", "ready"), ("ready", "merged")]),
        ends: vec!["merged".into()],
    }
}

fn deploy() -> LifecycleDefinition {
    let watch = |id: &str, label: &str| state(id, label, StateReadFrom::Watch { state: id.into() });
    LifecycleDefinition {
        id: "deploy".into(),
        name: "Deploy".into(),
        scope: PieceScope::Everywhere,
        states: vec![watch("rolling_out", "Rolling out"), watch("healthy", "Healthy")],
        transitions: moves_of(&[("rolling_out", "healthy")]),
        ends: vec!["healthy".into()],
    }
}

fn job() -> LifecycleDefinition {
    let fleet = |id: &str, label: &str| state(id, label, StateReadFrom::Fleet { fact: id.into() });
    LifecycleDefinition {
        id: "job".into(),
        name: "Job".into(),
        scope: PieceScope::Everywhere,
        states: vec![fleet("started", "Started"), fleet("landed", "Landed")],
        transitions: moves_of(&[("started", "landed")]),
        ends: vec!["landed".into()],
    }
}

fn steps() -> Vec<(String, String)> {
    [("plan", "Plan"), ("build", "Build"), ("ship", "Ship")].iter().map(|(id, label)| ((*id).into(), (*label).into())).collect()
}

fn held(lifecycle: &str) -> RuleTarget {
    RuleTarget { among: RuleAmong::Held { lifecycle: lifecycle.into() }, in_state: None }
}

fn doing(what: RuleAct, autonomy: &str) -> RuleDoing {
    RuleDoing { what, kind: "tracker_write".into(), autonomy: autonomy.into() }
}

fn rule(id: &str, when: RuleWhen, does: Vec<RuleAct>) -> LifecycleRule {
    LifecycleRule {
        id: id.into(),
        name: format!("Rule {id}"),
        scope: PieceScope::Everywhere,
        when,
        conditions: Vec::new(),
        does: does.into_iter().map(|what| doing(what, "acts")).collect(),
        autonomy: None,
        on_failure: RuleOnFailure { retries: 0, then: "tell_agent".into() },
    }
}

fn kits(rule: LifecycleRule) -> AppliedRule {
    AppliedRule { rule, kept: kept("everywhere", "kit") }
}

fn starts(step: &str) -> RuleWhen {
    RuleWhen::StepStarts { step: step.into() }
}

fn moves_ticket_to(to: &str) -> RuleAct {
    RuleAct::TrackerMove { to: to.into(), target: held("ticket") }
}

/// The repository asked about, in the one Workspace `web`.
fn store_api(lifecycles: &[AppliedLifecycle], rules: &[AppliedRule]) -> RepositoryRead {
    RepositoryRead { id: "store-api".into(), name: "store-api".into(), lifecycles: lifecycles.to_vec(), rules: rules.to_vec() }
}

fn web(repositories: &[&str]) -> WorkspaceRead {
    WorkspaceRead { id: "web".into(), name: "Web".into(), repositories: repositories.iter().map(|one| (*one).into()).collect() }
}

fn drawn_from(read: &PiecesRead, drafts: &Drafts) -> JobLanesView {
    drawn_at(read, drafts, "default")
}

fn drawn_at(read: &PiecesRead, drafts: &Drafts, scope: &str) -> JobLanesView {
    editor_lanes(&EditorInput { workflow: "feature", steps: &steps(), read, drafts, scope })
}

fn lanes_of(lifecycles: &[AppliedLifecycle], rules: &[AppliedRule]) -> JobLanesView {
    let read = PiecesRead { workspaces: vec![web(&["store-api"])], repositories: vec![store_api(lifecycles, rules)] };
    drawn_from(&read, &Drafts::default())
}

fn every() -> Vec<AppliedLifecycle> {
    vec![armadas(ticket()), armadas(pull()), armadas(deploy()), armadas(job())]
}

fn ids(view: &JobLanesView) -> Vec<&str> {
    view.lanes.iter().map(|lane| lane.id.as_str()).collect()
}

/// Each edge as `from -> to`.
fn edges(view: &JobLanesView) -> Vec<String> {
    view.rules.iter().map(|edge| format!("{} -> {}", edge.from, edge.to)).collect()
}

// ---- The lanes the stories draw -----------------------------------------------------------------------------------------------

/// **The workflow is on top, with a small circle where it starts and where it ends** in the columns of its
/// first and last step, and each step in the column its order gives it.
#[test]
fn draws_the_workflow_lane_with_a_start_and_an_end() {
    let view = lanes_of(&[], &[]);

    let flow = &view.lanes[0];
    assert_eq!((flow.id.as_str(), flow.kind, flow.title.as_str(), flow.source.as_str()), ("workflow", LaneKind::Workflow, "feature", "Workflow"));
    let nodes: Vec<(&str, u32, LaneNodeKind)> = flow.nodes.iter().map(|one| (one.id.as_str(), one.col, one.kind)).collect();
    assert_eq!(
        nodes,
        [
            ("workflow/$start", 0, LaneNodeKind::Start),
            ("workflow/plan", 0, LaneNodeKind::Step),
            ("workflow/build", 1, LaneNodeKind::Step),
            ("workflow/ship", 2, LaneNodeKind::Step),
            ("workflow/$end", 2, LaneNodeKind::End),
        ]
    );
    assert!(flow.nodes.iter().all(|one| one.role == LaneNodeRole::Step && !one.lit && !one.here && !one.passed));
    let moves: Vec<(&str, &str)> = flow.moves.iter().map(|(from, to)| (from.as_str(), to.as_str())).collect();
    assert_eq!(
        moves,
        [
            ("workflow/$start", "workflow/plan"),
            ("workflow/plan", "workflow/build"),
            ("workflow/build", "workflow/ship"),
            ("workflow/ship", "workflow/$end"),
        ]
    );
}

/// **No Job stands in the lanes**: nothing is at a step, and the Job's part holds only the definitions the
/// lanes are drawn from, which the forms that add a step, a lifecycle or a rule pick from.
#[test]
fn holds_the_definitions_and_no_job() {
    let view = lanes_of(&every(), &[kits(rule("plan_moves", starts("plan"), vec![moves_ticket_to("in_progress")]))]);

    assert_eq!((view.workflow.as_str(), view.at, view.working), ("feature", -1, false));
    assert!(view.skipped.is_empty());
    assert!(view.job.instances.is_empty() && view.job.fired.is_empty() && view.job.changes.is_empty() && view.job.waiting.is_empty());
    let steps: Vec<(&str, &str, u32)> = view.job.copy.steps.iter().map(|one| (one.id.as_str(), one.label.as_str(), one.col)).collect();
    assert_eq!(steps, [("plan", "Plan", 0), ("build", "Build", 1), ("ship", "Ship", 2)]);
    let lifecycles: Vec<&str> = view.job.copy.lifecycles.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(lifecycles, ["ticket", "pull_request", "deploy", "job"]);
    let rules: Vec<&str> = view.job.copy.rules.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(rules, ["plan_moves"]);
}

/// **A workflow with no step still has its lane**, empty, as the editor draws it, so a first step can be added to it.
#[test]
fn draws_a_workflow_with_no_step_as_an_empty_lane() {
    let view = editor_lanes(&EditorInput { workflow: "empty", steps: &[], read: &PiecesRead::default(), drafts: &Drafts::default(), scope: "default" });

    assert_eq!(ids(&view), ["workflow"]);
    assert!(view.lanes[0].nodes.is_empty() && view.lanes[0].moves.is_empty());
}

/// **Each state sits after the states that lead to it**, and a move back leads nowhere new; the lane is
/// the lifecycle's, by its id, and the ticket's lane says it is a ticket.
#[test]
fn places_each_state_after_the_states_that_lead_to_it() {
    let view = lanes_of(&[armadas(ticket())], &[]);

    assert_eq!(ids(&view), ["workflow", "ticket"]);
    let lane = &view.lanes[1];
    assert_eq!((lane.kind, lane.title.as_str(), lane.source.as_str(), lane.ticket), (LaneKind::Lifecycle, "Ticket", "Tracker", true));
    let nodes: Vec<(&str, &str, u32, bool)> = lane.nodes.iter().map(|one| (one.id.as_str(), one.label.as_str(), one.col, one.end)).collect();
    assert_eq!(
        nodes,
        [
            ("ticket/todo", "To do", 0, false),
            ("ticket/ready", "Ready", 1, false),
            ("ticket/in_progress", "In progress", 2, false),
            ("ticket/done", "Done", 3, true),
        ]
    );
    assert!(lane.nodes.iter().all(|one| one.role == LaneNodeRole::State && one.kind == LaneNodeKind::Tracker));
    assert!(lane.moves.contains(&("ticket/in_progress".into(), "ticket/todo".into())), "a move back is drawn: {:?}", lane.moves);
}

/// **A lane's nodes take the shape of what reads its states**: the forge's, a watch's, Fleet's for a Job.
/// Only the tracker's lane is a ticket.
#[test]
fn draws_each_lifecycle_in_the_shape_its_source_reads() {
    let view = lanes_of(&every(), &[]);

    let shapes: Vec<(&str, &str, LaneNodeKind, bool)> =
        view.lanes[1..].iter().map(|lane| (lane.id.as_str(), lane.source.as_str(), lane.nodes[0].kind, lane.ticket)).collect();
    assert_eq!(
        shapes,
        [
            ("ticket", "Tracker", LaneNodeKind::Tracker, true),
            ("pull_request", "Forge", LaneNodeKind::Forge, false),
            ("deploy", "Watch", LaneNodeKind::Watch, false),
            ("job", "Fleet", LaneNodeKind::Job, false),
        ]
    );
}

/// **A lifecycle with no state has no lane**: there is nowhere on it to draw.
#[test]
fn draws_no_lane_for_a_lifecycle_with_no_state() {
    let empty = LifecycleDefinition { states: Vec::new(), transitions: Vec::new(), ends: Vec::new(), ..ticket() };

    assert_eq!(ids(&lanes_of(&[armadas(empty)], &[])), ["workflow"]);
}

// ---- Where a rule is drawn ----------------------------------------------------------------------------------------------------

/// **A rule lands where its act says**: the state a ticket is moved to, the first state that ends the
/// ticket it closes, the state a pull request is brought to, the deploy a watch takes on; and where no act
/// lands, the step an `at_step` condition names. Each edge is the rule's, and waits on nothing.
#[test]
fn lands_each_rule_where_its_act_says() {
    let mut at_step = rule("tell_at_build", RuleWhen::Enters { lifecycle: "ticket".into(), state: "ready".into() }, vec![RuleAct::Tell {
        message: "Start building".into(),
    }]);
    at_step.conditions = vec![RuleCondition::AtStep { step: "build".into() }];
    let rules = vec![
        kits(rule("plan_moves", starts("plan"), vec![moves_ticket_to("in_progress")])),
        kits(rule("ship_closes", RuleWhen::StepEnds { step: "ship".into() }, vec![RuleAct::TrackerClose { target: held("ticket") }])),
        kits(rule("build_readies", RuleWhen::StepEnds { step: "build".into() }, vec![RuleAct::ForgeMarkReady { target: held("pull_request") }])),
        kits(rule("ship_merges", starts("ship"), vec![RuleAct::ForgeMerge { target: held("pull_request") }])),
        kits(rule("merged_deploys", RuleWhen::Enters { lifecycle: "pull_request".into(), state: "merged".into() }, vec![RuleAct::StartWatch {
            recipe: "deploy".into(),
            values: Default::default(),
            deploy: Some("production".into()),
        }])),
        kits(at_step),
    ];

    let view = lanes_of(&every(), &rules);

    assert_eq!(
        edges(&view),
        [
            "workflow/plan -> ticket/in_progress",
            "workflow/ship -> ticket/done",
            "workflow/build -> pull_request/ready",
            "workflow/ship -> pull_request/merged",
            "pull_request/merged -> deploy/rolling_out",
            "ticket/ready -> workflow/build",
        ]
    );
    let first = &view.rules[0];
    assert_eq!((first.id.as_str(), first.rule.as_str(), first.label.as_str()), ("plan_moves", "plan_moves", "Rule plan_moves"));
    assert!(view.rules.iter().all(|edge| edge.status == LaneOutcome::Waiting && edge.waits.is_none() && edge.fired_at.is_none() && !edge.quiet));
}

/// **A rule a Job event or a gate sets off has no node to leave from, and one whose acts land nowhere has
/// none to reach**: neither is drawn. There is no falling back to the step a Job is on, since none is.
#[test]
fn draws_no_rule_that_leaves_or_lands_on_no_node() {
    let rules = vec![
        kits(rule("on_landed", RuleWhen::Job { event: "landed".into() }, vec![moves_ticket_to("done")])),
        kits(rule("on_merged_gate", RuleWhen::GateCloses { gate: "merged".into() }, vec![moves_ticket_to("done")])),
        kits(rule("tells", starts("build"), vec![RuleAct::Tell { message: "Build it".into() }])),
    ];

    let view = lanes_of(&every(), &rules);

    assert!(view.rules.is_empty(), "{:?}", edges(&view));
    assert!(view.job.copy.rules.is_empty());
}

/// **A rule for one workflow is drawn in that one alone**; a rule for none is drawn in every workflow that
/// has both its ends.
#[test]
fn draws_a_rule_only_in_its_own_workflow() {
    let only = |id: &str, workflow: &str| {
        let mut one = rule(id, starts("plan"), vec![moves_ticket_to("ready")]);
        one.conditions = vec![RuleCondition::Workflow { workflow: workflow.into() }];
        kits(one)
    };
    let rules = vec![only("mine", "feature"), only("theirs", "bugfix"), kits(rule("anyones", starts("build"), vec![moves_ticket_to("in_progress")]))];

    let drawn: Vec<String> = lanes_of(&every(), &rules).rules.into_iter().map(|edge| edge.rule).collect();

    assert_eq!(drawn, ["mine", "anyones"]);
}

/// **An edge is drawn only between two nodes that are drawn**: a step this workflow lacks, a state the
/// lifecycle lacks, or a lifecycle with no lane leaves the rule out.
#[test]
fn draws_no_rule_whose_end_is_not_drawn() {
    let rules = vec![
        kits(rule("no_step", starts("review"), vec![moves_ticket_to("ready")])),
        kits(rule("no_state", starts("plan"), vec![moves_ticket_to("blocked")])),
        kits(rule("no_lane", starts("plan"), vec![RuleAct::ForgeMerge { target: held("pull_request") }])),
    ];

    let view = lanes_of(&[armadas(ticket())], &rules);

    assert!(view.rules.is_empty(), "{:?}", edges(&view));
}

/// **A rule that is off is drawn, as off**, so it can be turned on from the lanes; a Job's lanes leave it out.
#[test]
fn draws_a_rule_that_is_off() {
    let mut off = rule("quiet", starts("plan"), vec![moves_ticket_to("ready")]);
    off.autonomy = Some("off".into());
    let mut asks = rule("asking", starts("build"), vec![moves_ticket_to("in_progress")]);
    asks.does[0].autonomy = "asks".into();

    let view = lanes_of(&every(), &[kits(off), kits(asks)]);

    let autonomy: Vec<(&str, LaneAutonomy)> = view.rules.iter().map(|edge| (edge.rule.as_str(), edge.autonomy)).collect();
    assert_eq!(autonomy, [("quiet", LaneAutonomy::Off), ("asking", LaneAutonomy::Asks)]);
}

// ---- Which piece is the default's ---------------------------------------------------------------------------------------------

/// **The default level reads the piece scoped everywhere, Kit's over the repository's over Armada's**,
/// whatever order they come in. A lifecycle only a repository or a Workspace keeps has no lane on the
/// default's tab, and a narrower piece of one the default keeps does not change it.
#[test]
fn takes_the_default_levels_piece_kit_over_repository_over_armada() {
    let named = |name: &str| LifecycleDefinition { name: name.into(), ..ticket() };
    let within = |lifecycle: LifecycleDefinition, reach: &str| AppliedLifecycle {
        lifecycle: LifecycleDefinition { scope: PieceScope::Within { id: "store-api".into() }, ..lifecycle },
        kept: kept(reach, "repository"),
    };
    let lifecycles = vec![
        within(named("Repository's own ticket"), "repository"),
        AppliedLifecycle { lifecycle: named("Repository's ticket"), kept: kept("everywhere", "repository") },
        armadas(named("Armada's ticket")),
        AppliedLifecycle { lifecycle: named("Kit's ticket"), kept: kept("everywhere", "kit") },
        AppliedLifecycle {
            lifecycle: LifecycleDefinition { id: "incident".into(), scope: PieceScope::Within { id: "web".into() }, ..ticket() },
            kept: kept("workspace", "repository"),
        },
    ];

    let view = lanes_of(&lifecycles, &[]);

    assert_eq!(ids(&view), ["workflow", "ticket"]);
    assert_eq!(view.lanes[1].title, "Kit's ticket");
}

/// **Of two pieces at one level and place, the first read is taken.**
#[test]
fn takes_the_first_of_two_pieces_kept_alike() {
    let named = |name: &str| armadas(LifecycleDefinition { name: name.into(), ..ticket() });

    let view = lanes_of(&[named("First"), named("Second")], &[]);

    assert_eq!(view.lanes[1].title, "First");
}

/// **A rule is drawn from its piece at the widest level that keeps one**: the default's where it has one,
/// else the narrower piece, as the editor's default tab shows every rule.
#[test]
fn draws_a_rule_from_its_widest_piece() {
    let within = |rule: LifecycleRule, reach: &str| AppliedRule {
        rule: LifecycleRule { scope: PieceScope::Within { id: "store-api".into() }, ..rule },
        kept: kept(reach, "repository"),
    };
    let renamed = |rule: LifecycleRule, name: &str| LifecycleRule { name: name.into(), ..rule };
    let both = rule("both", starts("plan"), vec![moves_ticket_to("ready")]);
    let narrow = rule("narrow", starts("build"), vec![moves_ticket_to("in_progress")]);
    let rules = vec![
        within(renamed(both.clone(), "The repository's"), "repository"),
        kits(renamed(both, "The default's")),
        within(renamed(narrow.clone(), "The repository's own"), "repository"),
        AppliedRule { rule: LifecycleRule { scope: PieceScope::Within { id: "web".into() }, ..renamed(narrow, "The Workspace's") }, kept: kept("workspace", "repository") },
    ];

    let view = lanes_of(&every(), &rules);

    let labels: Vec<(&str, &str)> = view.rules.iter().map(|edge| (edge.rule.as_str(), edge.label.as_str())).collect();
    assert_eq!(labels, [("both", "The default's"), ("narrow", "The Workspace's")]);
}

// ---- Where a level below differs ----------------------------------------------------------------------------------------------

/// A lifecycle piece kept at a Workspace's or a repository's level.
fn kept_within(lifecycle: LifecycleDefinition, level: &str, reach: &str) -> AppliedLifecycle {
    AppliedLifecycle { lifecycle: LifecycleDefinition { scope: PieceScope::Within { id: level.into() }, ..lifecycle }, kept: kept(reach, "repository") }
}

fn rule_within(rule: LifecycleRule, level: &str, reach: &str) -> AppliedRule {
    AppliedRule { rule: LifecycleRule { scope: PieceScope::Within { id: level.into() }, ..rule }, kept: kept(reach, "repository") }
}

/// The ticket with one state's label and tracker words changed.
fn ticket_with(id: &str, label: &str, words: &[&str]) -> LifecycleDefinition {
    let mut one = ticket();
    let state = one.states.iter_mut().find(|state| state.id == id).expect("the ticket has it");
    state.label = label.into();
    state.read = StateReadFrom::Tracker { words: words.iter().map(|word| (*word).into()).collect() };
    one
}

/// Each state of the ticket's lane with the levels its mark names.
fn ticket_marks(view: &JobLanesView) -> Vec<(&str, Vec<&str>)> {
    let lane = view.lanes.iter().find(|lane| lane.id == "ticket").expect("the ticket's lane");
    lane.nodes.iter().map(|node| (node.id.as_str(), node.differs.iter().map(String::as_str).collect())).collect()
}

/// **A state names each level below the default that reads it otherwise**: a Workspace that calls it something
/// else, a repository whose tracker words for it are its own, and one whose piece leaves it out. Each is named as
/// the person knows it, and a state that no level changes has no mark.
#[test]
fn marks_each_state_a_level_below_names_reads_or_leaves_out() {
    let mut own = ticket_with("ready", "Queued", &["Ready"]);
    own.states.retain(|state| state.id != "todo");
    own.states.iter_mut().filter(|state| state.id == "in_progress").for_each(|state| state.read = StateReadFrom::Tracker { words: vec!["Doing".into()] });
    let lifecycles = vec![
        armadas(ticket()),
        kept_within(ticket_with("ready", "Queued", &["Ready"]), "web", "workspace"),
        kept_within(own, "store-api", "repository"),
    ];

    let view = lanes_of(&lifecycles, &[]);

    assert_eq!(
        ticket_marks(&view),
        [
            ("ticket/todo", vec!["store-api"]),
            ("ticket/ready", vec!["Web"]),
            ("ticket/in_progress", vec!["store-api"]),
            ("ticket/done", vec![]),
        ]
    );
    let labels: Vec<&str> = view.lanes[1].nodes.iter().map(|node| node.label.as_str()).collect();
    assert_eq!(labels, ["To do", "Ready", "In progress", "Done"], "the default's tab names each state as the default does");
}

/// **The other repositories of its Workspaces are read too**, so a state one of them changes names it; a piece
/// scoped to a repository nobody read has nowhere to be drawn and names nothing.
#[test]
fn marks_a_state_another_repository_of_the_workspace_changes() {
    let read = PiecesRead {
        workspaces: vec![web(&["store-api", "storefront"])],
        repositories: vec![
            store_api(&[armadas(ticket()), kept_within(ticket_with("ready", "Picked", &["Ready"]), "elsewhere", "repository")], &[]),
            RepositoryRead {
                id: "storefront".into(),
                name: "Storefront".into(),
                lifecycles: vec![armadas(ticket()), kept_within(ticket_with("done", "Shipped", &["Done"]), "storefront", "repository")],
                rules: Vec::new(),
            },
        ],
    };

    let view = drawn_from(&read, &Drafts::default());

    assert_eq!(
        ticket_marks(&view),
        [("ticket/todo", vec![]), ("ticket/ready", vec![]), ("ticket/in_progress", vec![]), ("ticket/done", vec!["Storefront"])]
    );
}

/// **Two Workspaces that set one piece alike tie for a repository in both, which then reads the default's**: the
/// default's tab names both where they differ, and marks no clash, since only a repository's own tab sets one aside.
#[test]
fn names_both_workspaces_that_tie_and_marks_no_clash_on_the_default_tab() {
    let mobile = WorkspaceRead { id: "mobile".into(), name: "Mobile".into(), repositories: vec!["store-api".into()] };
    let lifecycles = vec![
        armadas(ticket()),
        kept_within(ticket_with("ready", "Queued", &["Ready"]), "web", "workspace"),
        kept_within(ticket_with("ready", "Waiting", &["Ready"]), "mobile", "workspace"),
    ];
    let rules = vec![kits(rule("plan_readies", starts("plan"), vec![moves_ticket_to("ready")]))];
    let read = PiecesRead { workspaces: vec![web(&["store-api"]), mobile], repositories: vec![store_api(&lifecycles, &rules)] };

    let view = drawn_from(&read, &Drafts::default());

    assert_eq!(ticket_marks(&view)[1], ("ticket/ready", vec!["Web", "Mobile"]));
    assert!(view.lanes.iter().flat_map(|lane| &lane.nodes).all(|node| node.clash.is_none()));
    assert!(view.rules.iter().all(|edge| edge.clash.is_none()));
}

/// **A rule names each level that keeps it alone or keeps it otherwise**: a rule only a Workspace keeps names that
/// Workspace, a repository's own piece of a rule names the repository, and a piece that says what the one above
/// says, scope aside, changes nothing and names nothing.
#[test]
fn marks_each_rule_a_level_below_keeps_otherwise() {
    let renamed = |rule: LifecycleRule, name: &str| LifecycleRule { name: name.into(), ..rule };
    let plan = rule("plan_moves", starts("plan"), vec![moves_ticket_to("ready")]);
    let build = rule("build_moves", starts("build"), vec![moves_ticket_to("in_progress")]);
    let ship = rule("ship_moves", starts("ship"), vec![moves_ticket_to("done")]);
    let rules = vec![
        kits(plan.clone()),
        rule_within(renamed(plan, "Ours"), "store-api", "repository"),
        rule_within(build, "web", "workspace"),
        kits(ship.clone()),
        rule_within(ship, "store-api", "repository"),
        kits(rule("plain", RuleWhen::StepEnds { step: "ship".into() }, vec![moves_ticket_to("done")])),
    ];

    let view = lanes_of(&every(), &rules);

    let marks: Vec<(&str, Vec<&str>)> = view.rules.iter().map(|edge| (edge.rule.as_str(), edge.differs.iter().map(String::as_str).collect())).collect();
    assert_eq!(marks, [("plan_moves", vec!["store-api"]), ("build_moves", vec!["Web"]), ("ship_moves", vec![]), ("plain", vec![])]);
    assert_eq!(view.rules[0].label, "Rule plan_moves", "the default's tab draws the default's piece");
}

// ---- What a keep draft waits on -----------------------------------------------------------------------------------------------

/// **A rule only a keep draft holds is drawn as proposed**, and every lifecycle or rule a draft would keep says it
/// waits on the owner's Approve; one the draft changes is still drawn as the repository keeps it. A lifecycle only
/// a draft holds has a lane of its own.
#[test]
fn draws_what_a_keep_draft_would_add_as_proposed_and_waiting_on_approve() {
    let incident = LifecycleDefinition { id: "incident".into(), name: "Incident".into(), ..ticket() };
    let drafted = rule("drafted", starts("build"), vec![moves_ticket_to("in_progress")]);
    let drafts = Drafts {
        lifecycles: vec![incident, ticket_with("ready", "Not this", &["Ready"])],
        rules: vec![drafted, rule("held", starts("plan"), vec![moves_ticket_to("todo")])],
        keeps: ["lifecycle:incident", "lifecycle:ticket", "rule:drafted", "rule:held", "rule:unread"].iter().map(|one| (*one).into()).collect(),
    };
    let read = PiecesRead {
        workspaces: vec![web(&["store-api"])],
        repositories: vec![store_api(&every(), &[kits(rule("held", starts("plan"), vec![moves_ticket_to("ready")])), kits(rule("plain", starts("ship"), vec![moves_ticket_to("done")]))])],
    };

    let view = drawn_from(&read, &drafts);

    let edges: Vec<(&str, &str, bool, Option<LanePending>)> =
        view.rules.iter().map(|edge| (edge.rule.as_str(), edge.to.as_str(), edge.proposed, edge.pending)).collect();
    assert_eq!(
        edges,
        [
            ("held", "ticket/ready", false, Some(LanePending::Keep)),
            ("plain", "ticket/done", false, None),
            ("drafted", "ticket/in_progress", true, Some(LanePending::Keep)),
        ]
    );
    let lanes: Vec<(&str, Option<LanePending>)> = view.lanes.iter().map(|lane| (lane.id.as_str(), lane.pending)).collect();
    assert_eq!(
        lanes,
        [
            ("workflow", None),
            ("ticket", Some(LanePending::Keep)),
            ("pull_request", None),
            ("deploy", None),
            ("job", None),
            ("incident", Some(LanePending::Keep)),
        ]
    );
    assert_eq!(view.lanes[1].nodes[1].label, "Ready", "a draft of a lifecycle the repository keeps changes nothing drawn");
    let copied: Vec<&str> = view.job.copy.rules.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(copied, ["held", "plain"], "a proposed rule is no definition yet");
}

// ---- Each tab at its own level ------------------------------------------------------------------------------------------------

/// Each state of the ticket's lane with its name and the levels its mark names.
fn ticket_read(view: &JobLanesView) -> Vec<(&str, &str, Vec<&str>)> {
    let lane = view.lanes.iter().find(|lane| lane.id == "ticket").expect("the ticket's lane");
    lane.nodes.iter().map(|node| (node.id.as_str(), node.label.as_str(), node.differs.iter().map(String::as_str).collect())).collect()
}

/// **A Workspace's tab names each state as the Workspace does, and marks the repositories it holds that differ**;
/// the Workspace itself is no mark on its own tab.
#[test]
fn a_workspaces_tab_names_states_as_it_does_and_marks_the_repositories_it_holds() {
    let mut own = ticket_with("ready", "Queued", &["Ready"]);
    own.states.iter_mut().filter(|state| state.id == "in_progress").for_each(|state| state.read = StateReadFrom::Tracker { words: vec!["Doing".into()] });
    let lifecycles = vec![armadas(ticket()), kept_within(ticket_with("ready", "Queued", &["Ready"]), "web", "workspace"), kept_within(own, "store-api", "repository")];
    let read = PiecesRead { workspaces: vec![web(&["store-api"])], repositories: vec![store_api(&lifecycles, &[])] };

    let view = drawn_at(&read, &Drafts::default(), "workspace:web");

    assert_eq!(
        ticket_read(&view),
        [
            ("ticket/todo", "To do", vec![]),
            ("ticket/ready", "Queued", vec![]),
            ("ticket/in_progress", "In progress", vec!["store-api"]),
            ("ticket/done", "Done", vec![]),
        ]
    );
}

/// **A repository's tab runs its own piece whole**: a state its piece leaves out is not drawn, one it adds is, each
/// is named as the repository names it, and a rule reads as the repository keeps it. Only the repository is named
/// on its own tab.
#[test]
fn a_repositorys_tab_runs_its_own_piece_whole() {
    let mut own = ticket_with("ready", "Picked", &["Ready"]);
    own.states.retain(|state| state.id != "todo");
    own.states.insert(2, tracker("review", "Review"));
    own.transitions = moves_of(&[("ready", "in_progress"), ("in_progress", "review"), ("review", "done")]);
    let plan = rule("plan_moves", starts("plan"), vec![moves_ticket_to("ready")]);
    let rules = vec![kits(plan.clone()), rule_within(LifecycleRule { name: "Ours".into(), ..plan }, "store-api", "repository")];
    let read = PiecesRead { workspaces: vec![web(&["store-api"])], repositories: vec![store_api(&[armadas(ticket()), kept_within(own, "store-api", "repository")], &rules)] };

    let view = drawn_at(&read, &Drafts::default(), "store-api");

    assert_eq!(
        ticket_read(&view),
        [
            ("ticket/ready", "Picked", vec!["store-api"]),
            ("ticket/in_progress", "In progress", vec![]),
            ("ticket/done", "Done", vec![]),
            ("ticket/review", "Review", vec!["store-api"]),
        ]
    );
    let edges: Vec<(&str, &str, Vec<&str>)> =
        view.rules.iter().map(|edge| (edge.rule.as_str(), edge.label.as_str(), edge.differs.iter().map(String::as_str).collect())).collect();
    assert_eq!(edges, [("plan_moves", "Ours", vec!["store-api"])]);
    assert_eq!(view.job.copy.rules[0].label, "Ours", "the copy the forms pick from reads as the tab does");
}

/// **A repository in two Workspaces that set one piece alike reads the default's, and its tab says which tie**;
/// each Workspace's own tab reads its own piece and marks no clash.
#[test]
fn a_repository_in_two_workspaces_that_tie_reads_the_defaults_and_says_so() {
    let mobile = WorkspaceRead { id: "mobile".into(), name: "Mobile".into(), repositories: vec!["store-api".into()] };
    let lifecycles = vec![
        armadas(ticket()),
        kept_within(ticket_with("ready", "Queued", &["Ready"]), "web", "workspace"),
        kept_within(ticket_with("ready", "Waiting", &["Ready"]), "mobile", "workspace"),
    ];
    let read = PiecesRead { workspaces: vec![web(&["store-api"]), mobile], repositories: vec![store_api(&lifecycles, &[])] };
    let ready = |view: &JobLanesView| {
        let node = view.lanes.iter().flat_map(|lane| &lane.nodes).find(|node| node.id == "ticket/ready").expect("drawn").clone();
        (node.label, node.clash)
    };

    assert_eq!(ready(&drawn_at(&read, &Drafts::default(), "store-api")), ("Ready".to_string(), Some("Web and Mobile differ".to_string())));
    assert_eq!(ready(&drawn_at(&read, &Drafts::default(), "workspace:web")), ("Queued".to_string(), None));
    assert_eq!(ready(&drawn_at(&read, &Drafts::default(), "workspace:mobile")), ("Waiting".to_string(), None));
}

/// **A rule is drawn on the tabs that read it**: a Workspace's tab draws the default's, its own and those of the
/// repositories it holds; a repository's tab draws those of the levels it reads, and no other repository's.
#[test]
fn a_rule_is_drawn_on_the_tabs_that_read_it() {
    let mobile = WorkspaceRead { id: "mobile".into(), name: "Mobile".into(), repositories: vec!["store-api".into()] };
    let at = |id: &str, level: Option<&str>| {
        let one = rule(id, starts("plan"), vec![moves_ticket_to("ready")]);
        match level {
            None => kits(one),
            Some(level) => rule_within(one, level, "repository"),
        }
    };
    let rules = vec![at("everyone", None), at("webs", Some("web")), at("mobiles", Some("mobile")), at("storefronts", Some("storefront"))];
    let read = PiecesRead {
        workspaces: vec![web(&["store-api", "storefront"]), mobile],
        repositories: vec![
            store_api(&[armadas(ticket())], &rules),
            RepositoryRead { id: "storefront".into(), name: "Storefront".into(), lifecycles: vec![armadas(ticket())], rules: Vec::new() },
        ],
    };
    let drawn = |scope: &str| -> Vec<String> { drawn_at(&read, &Drafts::default(), scope).rules.into_iter().map(|edge| edge.rule).collect() };

    assert_eq!(drawn("default"), ["everyone", "webs", "mobiles", "storefronts"]);
    assert_eq!(drawn("workspace:web"), ["everyone", "webs", "storefronts"]);
    assert_eq!(drawn("store-api"), ["everyone", "webs", "mobiles"]);
    assert_eq!(drawn("storefront"), ["everyone", "webs", "storefronts"]);
}

// ---- The route ----------------------------------------------------------------------------------------------------------------

async fn holding() -> (TempDir, axum::Router) {
    let home = TempDir::new();
    let planted = Arc::new(Planted::nothing());
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);
    planted.will_read(read(staged_in_repository()));
    fleet.workflows_changed(&home.path().to_string_lossy());
    (home, app)
}

/// **`get_workflow_lanes` serves a workflow the repository runs**, its steps on top and a lane for each
/// lifecycle that applies, with no Job in them.
#[tokio::test]
async fn a_workflow_the_repository_runs_is_served_as_its_lanes() {
    let (_home, app) = holding().await;

    let (status, body) = call(&app, "GET", "/workflows/lanes?workflow=staged", "").await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let view: JobLanesView = ipc::decode("lanes", &body).expect("a workflow's lanes");
    assert_eq!((view.workflow.as_str(), view.at), ("staged", -1));
    let flow: Vec<&str> = view.lanes[0].nodes.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(flow, ["workflow/$start", "workflow/propose", "workflow/plan", "workflow/walk", "workflow/deliver", "workflow/$end"]);
    assert!(view.lanes.len() > 1, "Armada's lifecycles apply everywhere");
    assert!(view.lanes[1..].iter().all(|lane| lane.kind == LaneKind::Lifecycle));
}

/// **A mark is left off the wire where there is none**, on a lane, a node and an edge, so lanes whose pieces
/// nobody changes below the default read as they did before marks.
#[test]
fn the_lanes_carry_no_mark_where_nothing_differs() {
    let view = lanes_of(&every(), &[kits(rule("plan_moves", starts("plan"), vec![moves_ticket_to("ready")]))]);
    assert!(!view.rules.is_empty(), "an edge to say nothing on");

    let said = ipc::encode(&view).expect("encoded");

    for field in ["\"differs\":", "\"clash\":", "\"proposed\":", "\"pending\":"] {
        assert!(!said.contains(field), "{field} in {said}");
    }
}

/// **A level the repository's lanes do not have is refused**, as is a workflow the repository does not run; each
/// is 422 with a code of its own.
#[tokio::test]
async fn another_level_or_a_workflow_not_run_is_refused() {
    let (_home, app) = holding().await;

    let (status, body) = call(&app, "GET", "/workflows/lanes?workflow=staged&scope=default", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (status, body) = call(&app, "GET", "/workflows/lanes?workflow=staged&scope=store-api", "").await;
    assert_eq!((status, code(&body)), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.no_such_lanes_level".to_string()));
    let (status, body) = call(&app, "GET", "/workflows/lanes?workflow=nowhere", "").await;
    assert_eq!((status, code(&body)), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.no_such_workflow_definition".to_string()));
}
