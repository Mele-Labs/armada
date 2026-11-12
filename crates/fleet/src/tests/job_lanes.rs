//! A Job's lanes, worked out by Fleet: `crate::job_lanes`. **Every case is a case of
//! `packages/screens/src/session-lanes.test.ts`, on the same inputs**, so the Rust and the TypeScript
//! provably say the same of one Job; where a case there reads a field of `jobLanes`' output, this reads the
//! same field of `JobLanesView::lanes` or `rules`. The fixtures are that file's, name for name.
//!
//! Two cases are not here. `instanceKey` and `ruleOfWire` are TypeScript exports with no Rust counterpart: the
//! key a lane id was made of is read back by a client, and a rule is read through the view's copy. The
//! in-flight list is the record's `in_flight`, covered by `lifecycle_in_flight`.

use std::collections::HashMap;

use ipc::{
    AppliedRule, HeldInstance, Instant, InstanceLink, InstanceMove, InstanceStanding, JobCopyChange, JobLanesView, JobLifecycles, LaneAutonomy,
    LaneKept, LaneLifecycleKind, LaneMoment, LaneNodeKind, LaneNodeRole, LaneOutcome, LaneRef, LaneView, LifecycleDefinition,
    LifecycleRule, LifecycleStateDefinition, LifecycleTransition, PieceKept, PieceRank, PieceScope, RuleAct, RuleActReading, RuleAmong,
    RuleCondition, RuleDoing, RuleOnFailure, RuleTarget, RuleWhen, SessionId, StateReadFrom, StepVisit,
};

use crate::job_lanes::{lanes_of_job, LaneInput, LaneWorkflow};

const AT: &str = "2026-10-29T12:00:00.000Z";

fn at(text: &str) -> Instant {
    Instant::carried(text)
}

fn tracker_state(id: &str, label: &str, word: &str) -> LifecycleStateDefinition {
    LifecycleStateDefinition { id: id.into(), label: label.into(), read: StateReadFrom::Tracker { words: vec![word.into()] } }
}

fn moves_of(pairs: &[(&str, &str)]) -> Vec<LifecycleTransition> {
    pairs.iter().map(|(from, to)| LifecycleTransition { from: (*from).into(), to: (*to).into() }).collect()
}

fn ticket() -> LifecycleDefinition {
    LifecycleDefinition {
        id: "ticket".into(),
        name: "Ticket".into(),
        scope: PieceScope::Everywhere,
        states: vec![
            tracker_state("todo", "To do", "To Do"),
            tracker_state("ready", "Ready", "Ready"),
            tracker_state("in_progress", "In progress", "In Progress"),
            tracker_state("done", "Done", "Done"),
        ],
        transitions: moves_of(&[("todo", "ready"), ("todo", "in_progress"), ("ready", "in_progress"), ("in_progress", "done")]),
        ends: vec!["done".into()],
    }
}

fn pull() -> LifecycleDefinition {
    LifecycleDefinition {
        id: "pull_request".into(),
        name: "PR".into(),
        scope: PieceScope::Everywhere,
        states: vec![
            LifecycleStateDefinition { id: "draft".into(), label: "Draft".into(), read: StateReadFrom::Forge { fact: "draft".into() } },
            LifecycleStateDefinition { id: "approved".into(), label: "Approved".into(), read: StateReadFrom::Forge { fact: "approved".into() } },
        ],
        transitions: moves_of(&[("draft", "approved")]),
        ends: Vec::new(),
    }
}

fn deploy() -> LifecycleDefinition {
    LifecycleDefinition {
        id: "deploy".into(),
        name: "Deploy".into(),
        scope: PieceScope::Everywhere,
        states: vec![
            LifecycleStateDefinition { id: "rolling_out".into(), label: "Rolling out".into(), read: StateReadFrom::Watch { state: "rolling_out".into() } },
            LifecycleStateDefinition { id: "healthy".into(), label: "Healthy".into(), read: StateReadFrom::Watch { state: "healthy".into() } },
        ],
        transitions: moves_of(&[("rolling_out", "healthy")]),
        ends: vec!["healthy".into()],
    }
}

fn job_definition() -> LifecycleDefinition {
    let fleet = |id: &str, label: &str| LifecycleStateDefinition { id: id.into(), label: label.into(), read: StateReadFrom::Fleet { fact: id.into() } };
    LifecycleDefinition {
        id: "job".into(),
        name: "Job".into(),
        scope: PieceScope::Everywhere,
        states: vec![fleet("started", "Started"), fleet("landed", "Landed")],
        transitions: moves_of(&[("started", "landed")]),
        ends: vec!["landed".into()],
    }
}

fn held(lifecycle: &str, key: &str, label: &str, state: &str) -> HeldInstance {
    HeldInstance {
        lifecycle: lifecycle.into(),
        key: key.into(),
        label: label.into(),
        standing: InstanceStanding::In { state: state.into() },
        links: Vec::new(),
        ended_at: None,
    }
}

fn stood(instance: HeldInstance, standing: InstanceStanding) -> HeldInstance {
    HeldInstance { standing, ..instance }
}

fn ended(instance: HeldInstance) -> HeldInstance {
    HeldInstance { ended_at: Some(at(AT)), ..instance }
}

fn moved(lifecycle: &str, key: &str, from: InstanceStanding, state: &str) -> InstanceMove {
    InstanceMove {
        lifecycle: lifecycle.into(),
        key: key.into(),
        from,
        to: InstanceStanding::In { state: state.into() },
        source: "tracker".into(),
        at: at(AT),
    }
}

fn nothing() -> JobLifecycles {
    JobLifecycles {
        session_id: SessionId::carried("job"),
        lifecycles: Vec::new(),
        instances: Vec::new(),
        moves: Vec::new(),
        acts: Vec::new(),
        rules: Vec::new(),
        changes: Vec::new(),
    }
}

fn with_ticket() -> JobLifecycles {
    JobLifecycles {
        lifecycles: vec![ticket()],
        instances: vec![held("ticket", "ARM-41", "ARM-41", "in_progress")],
        moves: vec![
            moved("ticket", "ARM-41", InstanceStanding::Unread, "todo"),
            moved("ticket", "ARM-41", InstanceStanding::In { state: "todo".into() }, "in_progress"),
        ],
        ..nothing()
    }
}

fn with_pull() -> JobLifecycles {
    let mut one = with_ticket();
    one.lifecycles = vec![ticket(), pull()];
    one.instances.push(held("pull_request", "812", "PR #812", "draft"));
    one
}

fn steps() -> Vec<(String, String)> {
    vec![("plan".into(), "Plan".into()), ("land".into(), "Land".into())]
}

fn workflow(at: Option<u32>, began: bool) -> LaneWorkflow {
    LaneWorkflow { name: "feature".into(), steps: steps(), at, began, skipped: Vec::new(), visits: Vec::new() }
}

fn visits() -> Vec<StepVisit> {
    vec![
        StepVisit { step: 0, since: at("2026-10-29T11:00:00.000Z") },
        StepVisit { step: 1, since: at("2026-10-29T11:30:00.000Z") },
    ]
}

/// What `lanesOfJob` is handed, as the TypeScript test builds it: only what a case names.
#[derive(Default)]
struct Case {
    held: Option<JobLifecycles>,
    workflow: Option<LaneWorkflow>,
    rules: Vec<AppliedRule>,
    started: HashMap<String, JobLifecycles>,
    titles: HashMap<String, String>,
    working: bool,
    waiting: Vec<String>,
}

impl Case {
    fn of(held: JobLifecycles) -> Self {
        Case { held: Some(held), ..Case::default() }
    }
    fn rules(mut self, rules: Vec<AppliedRule>) -> Self {
        self.rules = rules;
        self
    }
    fn workflow(mut self, workflow: LaneWorkflow) -> Self {
        self.workflow = Some(workflow);
        self
    }
    fn working(mut self) -> Self {
        self.working = true;
        self
    }
    fn waiting(mut self, waiting: &[&str]) -> Self {
        self.waiting = waiting.iter().map(|one| (*one).to_string()).collect();
        self
    }
    fn started(mut self, id: &str, child: JobLifecycles) -> Self {
        self.started.insert(id.into(), child);
        self
    }
    fn titled(mut self, key: &str, title: &str) -> Self {
        self.titles.insert(key.into(), title.into());
        self
    }
    fn draw(&self) -> Option<JobLanesView> {
        let held = self.held.clone().unwrap_or_else(nothing);
        lanes_of_job(&LaneInput {
            held: &held,
            workflow: self.workflow.as_ref(),
            rules: &self.rules,
            started: &self.started,
            titles: &self.titles,
            working: self.working,
            waiting: &self.waiting,
        })
    }
    fn lanes(&self) -> JobLanesView {
        self.draw().expect("a Job with lanes")
    }
}

fn id(lifecycle: &str, key: &str) -> String {
    format!("{lifecycle}:{key}")
}

fn node(lane: &str, item: &str) -> String {
    format!("{lane}/{item}")
}

fn steps_drawn(lane: &LaneView) -> Vec<&ipc::LaneNode> {
    lane.nodes.iter().filter(|one| one.role == LaneNodeRole::Step && one.kind != LaneNodeKind::Start && one.kind != LaneNodeKind::End).collect()
}

// ---- The lanes a Job draws -----------------------------------------------------------------------------------------------------

#[test]
fn draws_no_lanes_for_a_job_that_holds_no_instance() {
    assert_eq!(Case::of(nothing()).draw(), None);
}

#[test]
fn draws_the_steps_of_a_workflow_for_a_job_that_holds_no_instance_yet_and_no_lane_under_them() {
    let lanes = Case::of(nothing()).working().workflow(workflow(Some(0), true)).lanes();
    assert_eq!(lanes.at, 0);
    assert_eq!(lanes.lanes.iter().map(|one| one.title.as_str()).collect::<Vec<_>>(), ["feature"]);
    assert_eq!(steps_drawn(&lanes.lanes[0]).iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Plan", "Land"]);
}

#[test]
fn draws_no_lanes_for_a_workflow_with_no_steps_and_no_instance() {
    let empty = LaneWorkflow { steps: Vec::new(), ..workflow(None, false) };
    assert_eq!(Case::of(nothing()).workflow(empty).draw(), None);
}

#[test]
fn places_each_state_after_the_ones_that_lead_to_it_and_never_beside_another_in_its_lane() {
    let cols = |lifecycle: LifecycleDefinition| -> Vec<(String, u32)> {
        let held = JobLifecycles { lifecycles: vec![lifecycle], instances: vec![held("ticket", "ARM-41", "ARM-41", "todo")], ..nothing() };
        Case::of(held).lanes().job.copy.lifecycles[0].states.iter().map(|one| (one.id.clone(), one.col)).collect()
    };
    assert_eq!(
        cols(ticket()),
        [("todo".to_string(), 0), ("ready".to_string(), 1), ("in_progress".to_string(), 2), ("done".to_string(), 3)]
    );
    let branching = cols(LifecycleDefinition { transitions: moves_of(&[("todo", "ready"), ("todo", "in_progress"), ("in_progress", "done")]), ..ticket() });
    let distinct: std::collections::HashSet<u32> = branching.iter().map(|(_, col)| *col).collect();
    assert_eq!(distinct.len(), 4);
    let col = |state: &str| branching.iter().find(|(id, _)| id == state).map(|(_, col)| *col).expect("a state");
    assert!(col("done") > col("in_progress"));
}

#[test]
fn reads_where_an_instance_stands_the_states_it_went_through_and_the_lifecycles_ends() {
    let lanes = Case::of(with_ticket()).lanes();
    let one = &lanes.job.instances[0];
    assert_eq!(one.id, id("ticket", "ARM-41"));
    assert_eq!((one.lifecycle.as_str(), one.label.as_str(), one.state.as_str()), ("ticket", "ARM-41", "in_progress"));
    assert_eq!(one.visited, ["todo", "in_progress"]);
    let lifecycle = &lanes.job.copy.lifecycles[0];
    assert_eq!((lifecycle.id.as_str(), lifecycle.label.as_str(), lifecycle.kind), ("ticket", "Ticket", LaneLifecycleKind::Tracker));
    assert!(lifecycle.states.iter().find(|state| state.id == "done").expect("done").end);
    assert!(!lifecycle.states.iter().find(|state| state.id == "todo").expect("todo").end);
}

#[test]
fn names_where_each_lanes_states_are_read_from_and_fleet_for_what_fleets_own_events_say() {
    let mut all = with_pull();
    all.lifecycles = vec![ticket(), pull(), deploy(), job_definition()];
    all.instances.push(held("deploy", "web", "Deploy", "rolling_out"));
    all.instances.push(held("job", "j1", "Fix it", "started"));
    let lanes = Case::of(all).lanes();
    assert_eq!(lanes.job.copy.lifecycles.iter().map(|one| one.from.as_str()).collect::<Vec<_>>(), ["Tracker", "Forge", "Watch", "Fleet"]);
}

#[test]
fn keeps_a_key_with_a_slash_in_one_lane_id_and_escapes_it_as_a_browser_does() {
    let mut slashed = with_ticket();
    slashed.instances = vec![held("ticket", "web/ARM-41", "ARM-41", "todo")];
    slashed.moves = Vec::new();
    let lanes = Case::of(slashed).lanes();
    assert_eq!(lanes.job.instances[0].id, "ticket:web%2FARM-41");
    assert_eq!(lanes.lanes[0].id, "ticket:web%2FARM-41");
    let mut odd = with_ticket();
    odd.instances = vec![held("ticket", "a b~c(d)!é", "x", "todo")];
    assert_eq!(Case::of(odd).lanes().job.instances[0].id, "ticket:a%20b~c(d)!%C3%A9");
}

#[test]
fn marks_the_lane_of_a_trackers_lifecycle_as_a_tickets_and_no_other() {
    let lanes = Case::of(with_pull()).working().lanes();
    assert_eq!(lanes.lanes.len(), 2);
    assert_eq!(lanes.lanes.iter().filter(|lane| lane.ticket).map(|lane| lane.id.as_str()).collect::<Vec<_>>(), ["ticket:ARM-41"]);
}

#[test]
fn leaves_no_state_lit_for_an_instance_whose_source_said_something_no_state_claims_and_says_so_on_the_lane() {
    let unmapped = JobLifecycles {
        instances: vec![stood(held("ticket", "ARM-41", "ARM-41", "todo"), InstanceStanding::Unmapped { said: "Blocked".into() })],
        moves: Vec::new(),
        ..with_ticket()
    };
    let lanes = Case::of(unmapped).lanes();
    let one = &lanes.job.instances[0];
    assert_eq!(one.state, "");
    assert!(one.visited.is_empty());
    assert_eq!(one.title.as_deref(), Some("Unmapped: Blocked"));
    assert_eq!(one.unmapped.as_deref(), Some("Blocked"));
    assert_eq!(lanes.lanes[0].note.as_deref(), Some("Unmapped: Blocked"));
    assert!(lanes.lanes[0].nodes.iter().all(|node| !node.here));
}

#[test]
fn draws_an_instance_that_ending_the_job_ended_where_it_stood_as_ended_and_not_as_now() {
    let stopped = JobLifecycles { instances: vec![ended(held("ticket", "ARM-41", "ARM-41", "in_progress"))], ..with_ticket() };
    let lanes = Case::of(stopped).lanes();
    let lane = &lanes.lanes[0];
    assert_eq!(lane.title, "ARM-41");
    assert!(lane.ended);
    let stood = lane.nodes.iter().find(|node| node.label == "In progress").expect("a node");
    assert!(stood.ended);
    assert!(!stood.here);
    assert!(!Case::of(with_ticket()).lanes().lanes[0].ended);
}

#[test]
fn draws_the_workflow_on_top_with_the_step_the_job_is_at_and_one_line_for_each_sentence_the_step_waits_with() {
    let lanes = Case::of(with_pull())
        .working()
        .waiting(&["Waiting on PR #812: Approved", "Waiting on ARM-41: Done"])
        .workflow(workflow(Some(1), true))
        .lanes();
    assert_eq!(lanes.lanes.iter().map(|lane| lane.title.as_str()).collect::<Vec<_>>(), ["feature", "ARM-41", "PR #812"]);
    let drawn = steps_drawn(&lanes.lanes[0]);
    let plan = drawn[0];
    let land = drawn[1];
    assert_eq!((plan.label.as_str(), plan.passed, plan.lit, plan.waiting.is_empty()), ("Plan", true, false, true));
    assert_eq!((land.label.as_str(), land.passed, land.lit), ("Land", false, true));
    assert_eq!(land.waiting, ["Waiting on PR #812: Approved", "Waiting on ARM-41: Done"]);
    assert_eq!(lanes.job.waiting, ["Waiting on PR #812: Approved", "Waiting on ARM-41: Done"]);
}

#[test]
fn lights_a_step_only_while_the_agent_works_on_it_and_marks_where_it_stands_otherwise() {
    let rested = Case::of(with_ticket()).workflow(workflow(Some(0), true)).lanes();
    let plan = steps_drawn(&rested.lanes[0])[0].clone();
    assert!(plan.here && !plan.lit);
    assert!(!rested.working);
    // Working with no step Fleet names is not working on a step.
    assert!(!Case::of(with_ticket()).working().workflow(workflow(None, false)).lanes().working);
    assert!(Case::of(with_ticket()).working().workflow(workflow(Some(0), true)).lanes().working);
}

#[test]
fn holds_the_step_at_nothing_before_one_starts_and_past_the_last_once_every_one_has_ended_where_fleet_names_no_step() {
    let before = Case::of(with_ticket()).workflow(workflow(None, false)).lanes();
    let after = Case::of(with_ticket()).workflow(workflow(None, true)).lanes();
    assert_eq!([before.at, after.at], [-1, steps().len() as i32]);
    assert!(!before.working);
    // The circles at either end of the workflow stand for where it began and where it ended.
    let start = before.lanes[0].nodes.first().expect("a start");
    assert_eq!((start.kind, start.passed), (LaneNodeKind::Start, false));
    let begun = after.lanes[0].nodes.first().expect("a start");
    assert!(begun.passed);
    assert!(after.lanes[0].nodes.last().expect("an end").here);
}

#[test]
fn draws_no_top_lane_for_a_job_with_no_workflow_and_says_nothing_of_waiting_when_the_list_is_empty() {
    let lanes = Case::of(with_ticket()).working().waiting(&[]).lanes();
    assert!(lanes.job.waiting.is_empty());
    assert_eq!(lanes.lanes.iter().map(|lane| lane.id.as_str()).collect::<Vec<_>>(), ["ticket:ARM-41"]);
    assert_eq!((lanes.workflow.as_str(), lanes.at, lanes.working), ("", -1, false));
}

#[test]
fn passes_the_steps_a_person_skipped_on() {
    let skipped = LaneWorkflow { skipped: vec![1], ..workflow(Some(0), true) };
    let lanes = Case::of(with_ticket()).workflow(skipped).lanes();
    assert_eq!(lanes.skipped, [1]);
    let drawn = steps_drawn(&lanes.lanes[0]);
    assert_eq!([drawn[0].skipped, drawn[1].skipped], [false, true]);
    assert!(Case::of(with_ticket()).workflow(workflow(Some(0), true)).lanes().skipped.is_empty());
}

// ---- Rules and what they did ---------------------------------------------------------------------------------------------------

fn kept(source: &str) -> PieceKept {
    PieceKept { rank: PieceRank { reach: "everywhere".into(), source: source.into(), home: false }, path: None, overridden_by: None, hides: None }
}

fn applied(id: &str, name: &str, when: RuleWhen, does: Vec<RuleDoing>, conditions: Vec<RuleCondition>) -> AppliedRule {
    AppliedRule {
        rule: LifecycleRule {
            id: id.into(),
            name: name.into(),
            scope: PieceScope::Everywhere,
            when,
            conditions,
            does,
            autonomy: None,
            on_failure: RuleOnFailure { retries: 0, then: "ask_owner".into() },
        },
        kept: kept("armada"),
    }
}

fn doing(kind: &str, autonomy: &str, what: RuleAct) -> RuleDoing {
    RuleDoing { what, kind: kind.into(), autonomy: autonomy.into() }
}

fn enters(lifecycle: &str, state: &str) -> RuleWhen {
    RuleWhen::Enters { lifecycle: lifecycle.into(), state: state.into() }
}

/// When the PR enters Draft, the ticket moves to In progress.
fn moves_ticket() -> AppliedRule {
    applied(
        "move-ticket",
        "Move to In progress",
        enters("pull_request", "draft"),
        vec![doing(
            "tracker_write",
            "acts",
            RuleAct::TrackerMove { to: "in_progress".into(), target: RuleTarget { among: RuleAmong::Held { lifecycle: "ticket".into() }, in_state: None } },
        )],
        Vec::new(),
    )
}

/// When the PR is approved, ask for review: it hands to the step.
fn asks() -> AppliedRule {
    applied(
        "ask-review",
        "Ask for review",
        enters("pull_request", "approved"),
        vec![doing(
            "draft",
            "asks",
            RuleAct::Draft { template: "review".into(), target: RuleTarget { among: RuleAmong::Entered, in_state: None } },
        )],
        Vec::new(),
    )
}

/// When the PR is approved and the Job is at Land, ask for review: the condition names the step it hands to.
fn asks_at_land() -> AppliedRule {
    AppliedRule {
        rule: LifecycleRule { id: "ask-at-land".into(), conditions: vec![RuleCondition::AtStep { step: "land".into() }], ..asks().rule },
        kept: kept("armada"),
    }
}

/// When Plan ends, tell the agent: nothing names a place for it to land.
fn tells() -> AppliedRule {
    applied(
        "tell-on-plan",
        "Tell on plan",
        RuleWhen::StepEnds { step: "plan".into() },
        vec![doing("tell_agent", "acts", RuleAct::Tell { message: "x".into() })],
        Vec::new(),
    )
}

/// When the PR enters Draft, start a watch on the deploy `web`.
fn watches() -> AppliedRule {
    applied(
        "watch-deploy",
        "Watch the deploy",
        enters("pull_request", "draft"),
        vec![doing(
            "start_watch",
            "acts",
            RuleAct::StartWatch { recipe: "rollout".into(), values: Default::default(), deploy: Some("web".into()) },
        )],
        Vec::new(),
    )
}

fn by_job() -> AppliedRule {
    applied("on-landed", "On landed", RuleWhen::Job { event: "landed".into() }, vec![doing("tell_agent", "acts", RuleAct::Wake)], Vec::new())
}

fn act(rule: &str, on: Option<InstanceLink>, standing: &str) -> RuleActReading {
    RuleActReading {
        rule: rule.into(),
        text: "Move ARM-41 to In progress".into(),
        course: "acts".into(),
        standing: standing.into(),
        said: None,
        at: at(AT),
        settled_at: None,
        on,
        entered: None,
    }
}

fn link(lifecycle: &str, key: &str) -> InstanceLink {
    InstanceLink { lifecycle: lifecycle.into(), key: key.into() }
}

fn on_ticket() -> Option<InstanceLink> {
    Some(link("ticket", "ARM-41"))
}

fn entered() -> Option<InstanceLink> {
    Some(link("pull_request", "812"))
}

fn with_acts(mut base: JobLifecycles, acts: Vec<RuleActReading>) -> JobLifecycles {
    base.acts = acts;
    base
}

/// The first rule of the Job's copy, as the lanes draw it.
fn drawn_rule(rule: AppliedRule, lifecycles: Vec<LifecycleDefinition>) -> Option<ipc::LaneRule> {
    let mut base = with_pull();
    base.lifecycles = lifecycles;
    Case::of(base).rules(vec![rule]).lanes().job.copy.rules.first().cloned()
}

fn lane_ref(lane: &str, item: &str) -> LaneRef {
    LaneRef { lane: lane.into(), item: item.into() }
}

#[test]
fn lands_a_rule_where_it_already_says_the_state_it_moves_the_first_state_of_the_deploy_it_takes_on_the_step_its_condition_names_and_else_the_step_the_job_is_on() {
    let moves = drawn_rule(moves_ticket(), vec![ticket(), pull()]).expect("a rule");
    assert_eq!(moves.id, "move-ticket");
    assert_eq!(moves.label, "Move to In progress");
    assert_eq!(moves.from, lane_ref("pull_request", "draft"));
    assert_eq!(moves.to, lane_ref("ticket", "in_progress"));
    assert_eq!((moves.at, moves.autonomy, moves.kept), (LaneMoment::Enters, LaneAutonomy::Acts, LaneKept::Armada));

    let at_land = drawn_rule(asks_at_land(), vec![ticket(), pull()]).expect("a rule");
    assert_eq!(at_land.from, lane_ref("pull_request", "approved"));
    assert_eq!(at_land.to, lane_ref("workflow", "land"));
    assert_eq!(at_land.autonomy, LaneAutonomy::Asks);

    assert_eq!(drawn_rule(asks(), vec![ticket(), pull()]).expect("a rule").to, lane_ref("workflow", "$step"));
    assert_eq!(drawn_rule(watches(), vec![ticket(), pull(), deploy()]).expect("a rule").to, lane_ref("deploy", "rolling_out"));
    assert_eq!(drawn_rule(watches(), vec![ticket(), pull()]).expect("a rule").to.lane, "deploy");
}

#[test]
fn leaves_from_the_step_a_step_sets_a_rule_off_for_and_does_not_draw_one_a_job_event_sets_off_which_has_no_node_to_leave_from() {
    let tell = drawn_rule(tells(), vec![ticket(), pull()]).expect("a rule");
    assert_eq!(tell.from, lane_ref("workflow", "plan"));
    assert_eq!(tell.at, LaneMoment::Ends);
    assert_eq!(tell.to, lane_ref("workflow", "$step"));
    assert_eq!(drawn_rule(by_job(), vec![ticket(), pull()]), None);
    let gate = applied("on-gate", "On gate", RuleWhen::GateCloses { gate: "checks_green".into() }, vec![doing("tell_agent", "acts", RuleAct::Wake)], Vec::new());
    assert_eq!(drawn_rule(gate, vec![ticket(), pull()]), None);
}

#[test]
fn moves_a_state_over_a_step_its_condition_names_and_over_a_deploy_it_takes_on() {
    let mut does = moves_ticket().rule.does;
    does.extend(watches().rule.does);
    let both = AppliedRule {
        rule: LifecycleRule { id: "both".into(), conditions: vec![RuleCondition::AtStep { step: "land".into() }], does, ..moves_ticket().rule },
        kept: kept("armada"),
    };
    assert_eq!(drawn_rule(both, vec![ticket(), pull(), deploy()]).expect("a rule").to, lane_ref("ticket", "in_progress"));
}

#[test]
fn names_a_rules_kept_place_as_the_persons_the_repositorys_or_armadas() {
    let kept_as = |source: &str| drawn_rule(AppliedRule { kept: kept(source), ..moves_ticket() }, vec![ticket(), pull()]).expect("a rule").kept;
    assert_eq!([kept_as("armada"), kept_as("kit"), kept_as("repository")], [LaneKept::Armada, LaneKept::Yours, LaneKept::Repository]);
}

#[test]
fn takes_the_strongest_word_among_a_rules_acts_unless_the_rule_names_its_own() {
    let acts = |words: &[&str], own: Option<&str>| {
        let does = words.iter().map(|word| doing("tell_agent", word, RuleAct::Wake)).collect();
        let mut one = applied("r", "R", enters("pull_request", "draft"), does, Vec::new());
        one.rule.autonomy = own.map(str::to_string);
        drawn_rule(one, vec![ticket(), pull()]).expect("a rule").autonomy
    };
    assert_eq!(acts(&["acts", "asks"], None), LaneAutonomy::Asks);
    assert_eq!(acts(&["off", "off"], None), LaneAutonomy::Off);
    assert_eq!(acts(&["off", "acts"], None), LaneAutonomy::Acts);
    assert_eq!(acts(&[], None), LaneAutonomy::Acts);
    assert_eq!(acts(&["asks"], Some("off")), LaneAutonomy::Off);
}

#[test]
fn draws_a_rule_that_moved_a_state_as_an_edge_between_the_two_lanes_with_what_it_did() {
    let settled = RuleActReading { entered: entered(), settled_at: Some(at("2026-10-29T12:05:00.000Z")), ..act("move-ticket", on_ticket(), "done") };
    let lanes = Case::of(with_acts(with_pull(), vec![settled])).rules(vec![moves_ticket(), by_job()]).lanes();
    assert_eq!(lanes.rules.len(), 1);
    let edge = &lanes.rules[0];
    assert_eq!(edge.rule, "move-ticket");
    assert_eq!(edge.from, node(&id("pull_request", "812"), "draft"));
    assert_eq!(edge.to, node(&id("ticket", "ARM-41"), "in_progress"));
    assert_eq!(edge.status, LaneOutcome::Acted);
    assert_eq!(edge.did, ["Move ARM-41 to In progress"]);
    assert_eq!(edge.fired_at, Some(at("2026-10-29T12:05:00.000Z")));
    assert_eq!(edge.waits, None);
    assert!(!edge.quiet);
}

#[test]
fn keeps_what_each_act_did_and_where_it_stands_and_the_rule_asks_while_any_act_waits_on_the_person() {
    let acts = vec![
        act("move-ticket", on_ticket(), "done"),
        RuleActReading { text: "Assign ARM-41 to you".into(), said: Some("Not allowed".into()), ..act("move-ticket", on_ticket(), "stuck") },
        RuleActReading { text: "Close ARM-41".into(), course: "asks".into(), ..act("move-ticket", on_ticket(), "planned") },
    ];
    let lanes = Case::of(with_acts(with_pull(), acts)).rules(vec![moves_ticket()]).lanes();
    assert_eq!(lanes.job.fired.len(), 1);
    let one = &lanes.job.fired[0];
    assert_eq!((one.rule.as_str(), one.instance.as_str(), one.outcome), ("move-ticket", "ticket:ARM-41", LaneOutcome::Asked));
    assert_eq!(
        one.acts.iter().map(|each| (each.text.as_str(), each.standing.as_str(), each.course, each.said.as_deref())).collect::<Vec<_>>(),
        [
            ("Move ARM-41 to In progress", "done", LaneAutonomy::Acts, None),
            ("Assign ARM-41 to you", "stuck", LaneAutonomy::Acts, Some("Not allowed")),
            ("Close ARM-41", "planned", LaneAutonomy::Asks, None),
        ]
    );
    assert_eq!(one.did, ["Move ARM-41 to In progress", "Assign ARM-41 to you · Stuck: Not allowed", "Close ARM-41 · Planned"]);
}

#[test]
fn reads_the_outcome_of_a_rule_from_where_its_acts_stand() {
    let outcome = |standing: &[&str]| {
        let acts = standing.iter().map(|one| act("move-ticket", on_ticket(), one)).collect();
        Case::of(with_acts(with_pull(), acts)).rules(vec![moves_ticket()]).lanes().job.fired[0].outcome
    };
    assert_eq!(
        [
            outcome(&["done"]),
            outcome(&["done", "told"]),
            outcome(&["planned"]),
            outcome(&["not_carried"]),
            outcome(&["asked"]),
            outcome(&["skipped", "skipped"]),
            outcome(&["skipped", "done"]),
        ],
        [
            LaneOutcome::Acted,
            LaneOutcome::Acted,
            LaneOutcome::Waiting,
            LaneOutcome::Waiting,
            LaneOutcome::Asked,
            LaneOutcome::Skipped,
            LaneOutcome::Acted,
        ]
    );
}

#[test]
fn keeps_one_line_for_each_instance_a_rule_was_done_to_and_none_for_a_rule_that_is_not_drawn() {
    let mut two = with_pull();
    two.instances.push(held("ticket", "ARM-42", "ARM-42", "todo"));
    two.acts = vec![
        act("move-ticket", on_ticket(), "done"),
        act("move-ticket", Some(link("ticket", "ARM-42")), "planned"),
        act("on-landed", None, "done"),
    ];
    let fired = Case::of(two).rules(vec![moves_ticket(), by_job()]).lanes().job.fired;
    assert_eq!(
        fired.iter().map(|one| (one.rule.as_str(), one.instance.as_str(), one.outcome)).collect::<Vec<_>>(),
        [("move-ticket", "ticket:ARM-41", LaneOutcome::Acted), ("move-ticket", "ticket:ARM-42", LaneOutcome::Waiting)]
    );
}

#[test]
fn hands_a_rule_that_names_no_place_to_the_step_the_job_was_on_when_it_was_planned_and_keys_it_by_the_instance_that_set_it_off() {
    let planned = RuleActReading {
        entered: entered(),
        text: "Ask Maya and Jun for review".into(),
        course: "asks".into(),
        at: at("2026-10-29T11:10:00.000Z"),
        ..act("ask-review", None, "asked")
    };
    let flow = LaneWorkflow { visits: visits(), ..workflow(Some(1), true) };
    let lanes = Case::of(with_acts(with_pull(), vec![planned])).rules(vec![asks()]).workflow(flow).lanes();
    let fired = &lanes.job.fired[0];
    assert_eq!((fired.rule.as_str(), fired.instance.as_str(), fired.step.as_deref(), fired.outcome), ("ask-review", "pull_request:812", Some("plan"), LaneOutcome::Asked));
    let edge = &lanes.rules[0];
    assert_eq!(edge.rule, "ask-review");
    assert_eq!(edge.from, node(&id("pull_request", "812"), "approved"));
    assert_eq!(edge.to, node("workflow", "plan"));
    assert_eq!(edge.status, LaneOutcome::Asked);
}

#[test]
fn lands_a_rule_on_the_step_its_condition_names_before_it_fires_and_after_wherever_the_job_is() {
    let draw = |acts: Vec<RuleActReading>| {
        Case::of(with_acts(with_pull(), acts)).rules(vec![asks_at_land()]).workflow(workflow(Some(0), true)).lanes().rules
    };
    let to = node("workflow", "land");
    let before = draw(Vec::new());
    assert_eq!((before[0].to.as_str(), before[0].status), (to.as_str(), LaneOutcome::Waiting));
    let after = draw(vec![RuleActReading {
        entered: entered(),
        text: "Ask Maya and Jun for review".into(),
        course: "asks".into(),
        ..act("ask-at-land", None, "done")
    }]);
    assert_eq!((after[0].to.as_str(), after[0].status), (to.as_str(), LaneOutcome::Acted));
}

#[test]
fn lands_a_rule_that_takes_a_deploy_on_in_that_deploys_lane_keyed_by_the_deploy_though_the_act_is_done_to_none() {
    let mut with_deploy = with_pull();
    with_deploy.lifecycles = vec![ticket(), pull(), deploy()];
    with_deploy.instances.push(held("deploy", "web", "Deploy", "rolling_out"));
    with_deploy.acts = vec![RuleActReading { entered: entered(), text: "Start the rollout watch".into(), ..act("watch-deploy", None, "done") }];
    let lanes = Case::of(with_deploy).rules(vec![watches()]).lanes();
    let fired = &lanes.job.fired[0];
    assert_eq!((fired.rule.as_str(), fired.instance.as_str(), fired.outcome), ("watch-deploy", "deploy:web", LaneOutcome::Acted));
    let edge = &lanes.rules[0];
    assert_eq!(edge.from, node(&id("pull_request", "812"), "draft"));
    assert_eq!(edge.to, node(&id("deploy", "web"), "rolling_out"));
    assert_eq!(edge.status, LaneOutcome::Acted);
}

#[test]
fn draws_nothing_for_a_rule_that_would_leave_a_step_and_land_on_the_same_one() {
    let told = RuleActReading { text: "Tell the agent".into(), at: at("2026-10-29T11:10:00.000Z"), ..act("tell-on-plan", None, "done") };
    let flow = LaneWorkflow { visits: visits(), ..workflow(Some(1), true) };
    let lanes = Case::of(with_acts(with_pull(), vec![told])).rules(vec![tells()]).workflow(flow).lanes();
    let fired = &lanes.job.fired[0];
    assert_eq!((fired.rule.as_str(), fired.instance.as_str(), fired.step.as_deref()), ("tell-on-plan", "", Some("plan")));
    assert!(lanes.rules.is_empty());
}

#[test]
fn draws_no_edge_for_a_rule_whose_node_is_not_drawn() {
    let lanes = Case::of(with_acts(with_ticket(), vec![act("move-ticket", on_ticket(), "done")])).rules(vec![moves_ticket(), asks()]).lanes();
    assert!(lanes.rules.is_empty());
}

#[test]
fn draws_no_act_where_fleet_sends_none_and_keeps_the_lanes_as_they_were() {
    let lanes = Case::of(with_ticket()).rules(vec![moves_ticket()]).lanes();
    assert!(lanes.job.fired.is_empty());
}

#[test]
fn draws_a_rule_still_waiting_as_an_edge_naming_what_it_waits_for_and_a_second_instances_edge_quiet() {
    let on_approved = AppliedRule {
        rule: LifecycleRule { id: "move-on-approved".into(), when: enters("pull_request", "approved"), ..moves_ticket().rule },
        kept: kept("armada"),
    };
    let mut two = with_pull();
    two.instances.push(held("ticket", "ARM-42", "ARM-42", "todo"));
    let lanes = Case::of(two).rules(vec![on_approved]).lanes();
    assert_eq!(lanes.rules.len(), 2);
    let (first, second) = (&lanes.rules[0], &lanes.rules[1]);
    assert_eq!(first.id, "move-on-approved:pull_request:812>ticket:ARM-41");
    assert_eq!(second.id, "move-on-approved:pull_request:812>ticket:ARM-42");
    assert_eq!((first.status, first.waits.as_deref(), first.fired_at.clone()), (LaneOutcome::Waiting, Some("PR #812: Approved"), None));
    assert_eq!((first.quiet, second.quiet), (false, true));
    assert!(first.did.is_empty());
    // A rule whose source was reached and that never fired has nothing left to show.
    assert!(Case::of(with_pull()).rules(vec![moves_ticket()]).lanes().rules.is_empty());
}

#[test]
fn waits_on_a_step_to_end_until_the_job_is_past_it_and_lands_on_the_current_step_when_nothing_names_one() {
    // When Plan ends, hand to Land: the condition names where it lands.
    let hands = AppliedRule {
        rule: LifecycleRule { id: "hand-to-land".into(), conditions: vec![RuleCondition::AtStep { step: "land".into() }], ..tells().rule },
        kept: kept("armada"),
    };
    let edges = |at: Option<u32>, began: bool| {
        Case::of(with_pull()).rules(vec![hands.clone()]).workflow(workflow(at, began)).lanes().rules
    };
    let before = edges(None, false);
    assert_eq!(before.len(), 1);
    assert_eq!((before[0].from.as_str(), before[0].to.as_str(), before[0].waits.as_deref()), ("workflow/plan", "workflow/land", Some("Plan ends")));
    assert_eq!(edges(Some(0), true)[0].waits.as_deref(), Some("Plan ends"));
    assert!(edges(Some(1), true).is_empty(), "Plan is behind the Job, and the rule never fired");
    // A rule that names no place lands on the step the Job is on, which is where it leaves from.
    assert!(Case::of(with_pull()).rules(vec![tells()]).workflow(workflow(Some(0), true)).lanes().rules.is_empty());
}

// ---- The Job's own copy --------------------------------------------------------------------------------------------------------

fn change(lane: &str, item: &str, text: &str, dropped: bool) -> JobCopyChange {
    JobCopyChange { lane: lane.into(), item: item.into(), text: text.into(), by: ipc::ChangedBy::You, at: at(AT), dropped }
}

#[test]
fn draws_the_rules_the_jobs_copy_holds_in_place_of_the_repositorys_and_marks_what_it_changed() {
    let own = JobLifecycles { rules: vec![asks()], changes: vec![change("rule", "ask-review", "Added by you", false)], ..with_pull() };
    let lanes = Case::of(own).rules(vec![moves_ticket()]).lanes();
    assert_eq!(lanes.job.copy.rules.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["ask-review"]);
    let lone = Case::of(with_pull()).rules(vec![moves_ticket()]).lanes();
    assert_eq!(lone.job.copy.rules.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["move-ticket"]);
    assert_eq!(lanes.job.changes.len(), 1);
    let one = &lanes.job.changes[0];
    assert_eq!(one.id, format!("rule:ask-review:{AT}"));
    assert_eq!((one.lane.as_str(), one.item.as_str(), one.text.as_str(), one.at.clone()), ("rule", "ask-review", "Added by you", at(AT)));
    assert_eq!((one.dropped, one.name.as_deref()), (false, Some("Ask for review")));
}

#[test]
fn names_a_changed_rule_the_lanes_do_not_draw_one_a_job_event_sets_off_and_one_the_copy_dropped_that_the_repository_still_holds() {
    let own = JobLifecycles {
        rules: vec![by_job()],
        changes: vec![
            change("rule", "on-landed", "Added by you", false),
            change("rule", "ask-review", "Dropped by you", true),
            change("rule", "gone", "Dropped by you", true),
        ],
        ..with_pull()
    };
    let lanes = Case::of(own).rules(vec![asks()]).lanes();
    assert!(lanes.job.copy.rules.is_empty());
    assert_eq!(
        lanes.job.changes.iter().map(|one| (one.item.as_str(), one.name.as_deref())).collect::<Vec<_>>(),
        [("on-landed", Some("On landed")), ("ask-review", Some("Ask for review")), ("gone", None)]
    );
}

#[test]
fn carries_which_change_took_a_rule_out_of_the_copy_so_the_lanes_offer_nothing_to_keep_for_it() {
    let own = JobLifecycles {
        changes: vec![change("rule", "ask-at-land", "Dropped by you", true), change("rule", "ask-review", "Added by you", false)],
        ..with_pull()
    };
    let lanes = Case::of(own).workflow(workflow(Some(0), true)).lanes();
    assert_eq!(lanes.job.changes.iter().map(|one| (one.item.as_str(), one.dropped)).collect::<Vec<_>>(), [("ask-at-land", true), ("ask-review", false)]);
}

#[test]
fn marks_a_step_a_rule_and_a_lifecycle_lane_that_the_jobs_copy_changed_each_with_who_made_the_change() {
    let own = JobLifecycles {
        rules: vec![asks_at_land()],
        changes: vec![
            change("workflow", "plan", "Added after Propose by the Job", false),
            change("rule", "ask-at-land", "Added by you", false),
            change("ticket", "", "Changed by the Job", false),
        ],
        ..with_pull()
    };
    let lanes = Case::of(own).workflow(workflow(Some(0), true)).lanes();
    let flow = &lanes.lanes[0];
    assert_eq!(flow.nodes.iter().find(|node| node.label == "Plan").and_then(|node| node.changed.as_deref()), Some("Added after Propose by the Job"));
    assert_eq!(flow.nodes.iter().find(|node| node.label == "Land").and_then(|node| node.changed.clone()), None);
    assert_eq!(lanes.lanes.iter().find(|lane| lane.title == "ARM-41").and_then(|lane| lane.changed.as_deref()), Some("Changed by the Job"));
    assert_eq!(lanes.lanes.iter().find(|lane| lane.title.starts_with("PR")).and_then(|lane| lane.changed.clone()), None);
    assert_eq!(lanes.rules.iter().find(|edge| edge.rule == "ask-at-land").and_then(|edge| edge.changed.as_deref()), Some("Added by you"));
}

// ---- A ticket handed to the Job started for it --------------------------------------------------------------------------------

/// The Job that started `child` for ARM-52: the ticket ended on it and linked from its instance of the Job, as Fleet writes a hand-off.
fn handing_with(child: HeldInstance) -> JobLifecycles {
    JobLifecycles {
        session_id: SessionId::carried("parent"),
        lifecycles: vec![job_definition(), ticket()],
        instances: vec![
            child,
            ended(held("ticket", "ARM-52", "ARM-52", "todo")),
            held("ticket", "ARM-58", "ARM-58", "todo"),
        ],
        ..nothing()
    }
}

fn handing() -> JobLifecycles {
    handing_with(HeldInstance { links: vec![link("ticket", "ARM-52")], ..held("job", "child", "Fix cart rounding", "started") })
}

/// What the started Job holds of the same ticket, where it reads it.
fn reading(standing: InstanceStanding) -> JobLifecycles {
    JobLifecycles {
        session_id: SessionId::carried("child"),
        lifecycles: vec![ticket()],
        instances: vec![stood(held("ticket", "ARM-52", "ARM-52", "todo"), standing)],
        ..nothing()
    }
}

fn in_state(state: &str) -> InstanceStanding {
    InstanceStanding::In { state: state.into() }
}

fn card(parent: JobLifecycles, child: Option<JobLifecycles>) -> Option<ipc::LaneCarriedTicket> {
    let case = match child {
        Some(child) => Case::of(parent).started("child", child),
        None => Case::of(parent),
    };
    case.lanes().job.instances.into_iter().find(|one| one.id == id("job", "child")).expect("the Job's instance").ticket
}

fn carried(label: &str, title: Option<&str>, state: &str, state_id: &str) -> Option<ipc::LaneCarriedTicket> {
    Some(ipc::LaneCarriedTicket { label: label.into(), title: title.map(str::to_string), state: state.into(), state_id: state_id.into() })
}

#[test]
fn carries_a_handed_ticket_on_the_job_instance_that_links_to_it_in_the_state_the_started_job_reads_it_in_named_as_that_jobs_lifecycle_names_it() {
    assert_eq!(card(handing(), Some(reading(in_state("in_progress")))), carried("ARM-52", None, "In progress", "in_progress"));
}

#[test]
fn draws_no_lane_for_a_handed_ticket_and_keeps_the_lane_of_a_ticket_the_job_kept() {
    let lanes = Case::of(handing()).started("child", reading(in_state("in_progress"))).lanes();
    assert_eq!(lanes.job.instances.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["job:child", "ticket:ARM-58"]);
}

#[test]
fn keeps_the_lane_of_a_ticket_a_job_links_to_while_the_ticket_is_still_live_it_is_the_jobs_own() {
    let mut live = handing();
    live.instances[1].ended_at = None;
    let lanes = Case::of(live).started("child", reading(in_state("in_progress"))).lanes();
    assert!(lanes.job.instances.iter().any(|one| one.id == "ticket:ARM-52"));
    assert_eq!(lanes.job.instances.iter().find(|one| one.id == "job:child").expect("the Job's instance").ticket, None);
}

#[test]
fn keeps_the_lane_of_an_ended_ticket_that_no_job_links_to() {
    let lanes = Case::of(handing_with(held("job", "child", "Fix cart rounding", "started"))).lanes();
    assert!(lanes.job.instances.iter().any(|one| one.id == "ticket:ARM-52"));
    assert_eq!(lanes.job.instances.iter().find(|one| one.id == "job:child").expect("the Job's instance").ticket, None);
}

#[test]
fn follows_the_started_job_the_state_is_the_one_it_reads_now() {
    let states: Vec<_> = ["todo", "in_progress", "done"].iter().map(|state| card(handing(), Some(reading(in_state(state)))).expect("a ticket")).map(|one| (one.state_id, one.state)).collect();
    assert_eq!(
        states,
        [
            ("todo".to_string(), "To do".to_string()),
            ("in_progress".to_string(), "In progress".to_string()),
            ("done".to_string(), "Done".to_string()),
        ]
    );
}

#[test]
fn keeps_the_word_a_source_said_for_a_status_no_state_lists_and_gives_it_no_state_id() {
    assert_eq!(card(handing(), Some(reading(InstanceStanding::Unmapped { said: "Awaiting QA".into() }))), carried("ARM-52", None, "Awaiting QA", ""));
}

#[test]
fn draws_the_key_with_no_state_while_the_started_job_has_not_read_it_or_its_lifecycles_are_not_served() {
    assert_eq!(card(handing(), Some(reading(InstanceStanding::Unread))), carried("ARM-52", None, "", ""));
    assert_eq!(card(handing(), None), carried("ARM-52", None, "", ""));
}

#[test]
fn hands_the_ticket_to_the_lane_of_the_job_drawn_on_the_node_where_that_job_stands() {
    let lanes = Case::of(handing()).started("child", reading(in_state("in_progress"))).lanes();
    let lane = lanes.lanes.iter().find(|one| one.id == "job:child").expect("the Job's lane");
    assert_eq!(lane.nodes.iter().find(|node| node.here).and_then(|node| node.ticket.clone()), carried("ARM-52", None, "In progress", "in_progress"));
    assert!(!lanes.lanes.iter().any(|one| one.ticket && one.title == "ARM-52"));
}

#[test]
fn titles_a_handed_ticket_from_the_issues_by_its_key_and_leaves_the_title_out_where_none_is_known() {
    let titled = |key: &str, title: &str| {
        let case = Case::of(handing()).started("child", reading(in_state("in_progress"))).titled(key, title);
        case.lanes().job.instances.into_iter().find(|one| one.id == "job:child").expect("the Job's instance").ticket
    };
    assert_eq!(titled("ARM-52", "Cart totals are a cent off"), carried("ARM-52", Some("Cart totals are a cent off"), "In progress", "in_progress"));
    assert_eq!(titled("ARM-58", "Empty wishlist link 404s"), carried("ARM-52", None, "In progress", "in_progress"));
    assert_eq!(titled("ARM-52", ""), carried("ARM-52", None, "In progress", "in_progress"));
}
