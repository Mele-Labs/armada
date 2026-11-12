//! A workflow's lanes as the Workflows page's editor draws them, at the default level: `editorLanes` in
//! `LaneCanvas/build.ts`, read off the pieces as `workspaceOf` and `placesOf` in Bridge's
//! `lifecycle-editing/` do. Served by `get_workflow_lanes`.
//!
//! **Departures, each because the default level needs no more.** A lifecycle is its default piece alone:
//! Bridge's merges in the states and moves a narrower piece adds, then hides them on the default's tab, and
//! lays the columns out over all of them. A rule is drawn from the piece at its widest level, as Bridge's
//! is, with no mark yet of where it differs. The pieces are the one repository's that was asked about,
//! where Bridge also reads the other repositories of its Workspaces, and two Workspaces that keep one id
//! are taken in the order their pieces are read.

use std::collections::HashSet;

use ipc::{
    AppliedLifecycle, AppliedRule, JobLanesHeld, JobLanesView, LaneCopy, LaneEdge, LaneKept, LaneKind, LaneLifecycle,
    LaneMoment, LaneNodeKind, LaneNodeRole, LaneOutcome, LaneRef, LaneRule, LaneStep, LaneView, LifecycleDefinition,
    LifecycleRule, PieceKept, PieceScope, RuleAct, RuleCondition, RuleWhen,
};

use super::draw::{kind_of_state, lane_of_lifecycle, node, node_id, with_ends};
use super::read::{autonomy_of, lane_of_target, lifecycle};

/// The lifecycle a watch's `deploy` lands on.
const DEPLOY: &str = "deploy";

/// What a workflow's editor lanes are read from.
pub(crate) struct EditorInput<'a> {
    /// The workflow's id, which titles its lane.
    pub workflow: &'a str,
    /// Each step's id and label, in order.
    pub steps: &'a [(String, String)],
    /// Every lifecycle piece that applies to the repository, at every level: `list_lifecycles`' list.
    pub lifecycles: &'a [AppliedLifecycle],
    /// Every rule piece that applies to it: `list_lifecycle_rules`' list.
    pub rules: &'a [AppliedRule],
}

/// How wide a piece's level is: the default's, a Workspace's, or one repository's.
fn width(kept: &PieceKept) -> u8 {
    match kept.rank.reach.as_str() {
        "everywhere" => 0,
        "workspace" => 1,
        _ => 2,
    }
}

/// Where a piece of one id and level is preferred from: the person's own, then the repository's, then Armada's.
fn place(kept: &PieceKept) -> u8 {
    match kept.rank.source.as_str() {
        "kit" => 0,
        "repository" => 1,
        "armada" => 2,
        _ => 9,
    }
}

fn scope_id(scope: &PieceScope) -> &str {
    match scope {
        PieceScope::Everywhere => "",
        PieceScope::Within { id } => id,
    }
}

/// Each id's piece at the widest level that keeps one, in the order the ids are first read. Within a level
/// the preferred place wins, and the first read where two tie.
fn widest<'a, T>(pieces: &'a [T], of: impl Fn(&'a T) -> (&'a str, &'a PieceScope, &'a PieceKept)) -> Vec<&'a T> {
    let mut ids: Vec<&str> = Vec::new();
    let mut levels: Vec<(u8, &str)> = Vec::new();
    for one in pieces {
        let (id, scope, kept) = of(one);
        if !ids.contains(&id) {
            ids.push(id);
        }
        let level = (width(kept), scope_id(scope));
        if !levels.contains(&level) {
            levels.push(level);
        }
    }
    let level_of = |one: &'a T| {
        let (_, scope, kept) = of(one);
        let level = (width(kept), scope_id(scope));
        (level.0, levels.iter().position(|each| *each == level).unwrap_or(usize::MAX))
    };
    ids.into_iter()
        .filter_map(|id| {
            let mine: Vec<&'a T> = pieces.iter().filter(|one| of(one).0 == id).collect();
            let level = mine.iter().map(|one| level_of(one)).min()?;
            mine.into_iter().filter(|one| level_of(one) == level).reduce(|best, one| if place(of(one).2) < place(of(best).2) { one } else { best })
        })
        .collect()
}

/// The node a rule lands on: the state a ticket is moved to or closed in, the state a pull request is
/// brought to, the deploy a watch takes on, else the step an `at_step` condition names. None lands it nowhere.
fn landing(rule: &LifecycleRule, defs: &[&LifecycleDefinition]) -> Option<LaneRef> {
    let def = |id: &str| defs.iter().find(|one| one.id == id);
    let at = |lane: String, item: &str| Some(LaneRef { lane, item: item.to_string() });
    for one in &rule.does {
        let found = match &one.what {
            RuleAct::TrackerMove { to, target } => lane_of_target(target, rule).and_then(|lane| at(lane, to)),
            RuleAct::TrackerClose { target } => lane_of_target(target, rule).and_then(|lane| {
                // The first state, in the lifecycle's order, that ends it.
                let found = def(&lane)?;
                let end = found.states.iter().find(|state| found.ends.contains(&state.id))?;
                at(lane, &end.id)
            }),
            RuleAct::ForgeMarkReady { target } => lane_of_target(target, rule).and_then(|lane| at(lane, "ready")),
            RuleAct::ForgeMerge { target } => lane_of_target(target, rule).and_then(|lane| at(lane, "merged")),
            RuleAct::StartWatch { deploy: Some(deploy), .. } if !deploy.is_empty() => {
                def(DEPLOY).and_then(|one| one.states.first()).and_then(|first| at(DEPLOY.into(), &first.id))
            }
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    rule.conditions.iter().find_map(|one| match one {
        RuleCondition::AtStep { step } => at("workflow".into(), step),
        _ => None,
    })
}

/// A rule as the lanes draw it, and the one workflow it belongs to. None where a Job event or a gate sets
/// it off, or it lands on no node.
fn drawn_rule(applied: &AppliedRule, defs: &[&LifecycleDefinition]) -> Option<(LaneRule, Option<String>)> {
    let AppliedRule { rule, kept } = applied;
    let (from, at) = match &rule.when {
        RuleWhen::Job { .. } | RuleWhen::GateCloses { .. } => return None,
        RuleWhen::Enters { lifecycle, state } => (LaneRef { lane: lifecycle.clone(), item: state.clone() }, LaneMoment::Enters),
        RuleWhen::StepStarts { step } => (LaneRef { lane: "workflow".into(), item: step.clone() }, LaneMoment::Starts),
        RuleWhen::StepEnds { step } => (LaneRef { lane: "workflow".into(), item: step.clone() }, LaneMoment::Ends),
    };
    let to = landing(rule, defs)?;
    let workflow = rule.conditions.iter().find_map(|one| match one {
        RuleCondition::Workflow { workflow } => Some(workflow.clone()),
        _ => None,
    });
    let kept = match kept.rank.source.as_str() {
        "armada" => LaneKept::Armada,
        "kit" => LaneKept::Yours,
        _ => LaneKept::Repository,
    };
    Some((LaneRule { id: rule.id.clone(), label: rule.name.clone(), from, at, to, autonomy: autonomy_of(rule), kept }, workflow))
}

/// The editor's lanes: the workflow on top, then a lane for each lifecycle at the default level, its states
/// in the columns the moves between them give, and an edge for each rule whose ends are both drawn. **No
/// Job stands in them**, so `job` holds only the definitions they are drawn from, and an edge waits on nothing.
pub(crate) fn editor_lanes(input: &EditorInput<'_>) -> JobLanesView {
    let pieces = widest(input.lifecycles, |one| (one.lifecycle.id.as_str(), &one.lifecycle.scope, &one.kept));
    let defs: Vec<&LifecycleDefinition> = pieces.iter().map(|one| &one.lifecycle).collect();
    let lifecycles: Vec<LaneLifecycle> = pieces
        .iter()
        .filter(|one| width(&one.kept) == 0 && !one.lifecycle.states.is_empty())
        .map(|one| lifecycle(&one.lifecycle))
        .collect();
    let steps: Vec<LaneStep> = input
        .steps
        .iter()
        .enumerate()
        .map(|(col, (id, label))| LaneStep { id: id.clone(), label: label.clone(), col: col as u32 })
        .collect();
    let drawn: HashSet<String> = steps
        .iter()
        .map(|step| node_id("workflow", &step.id))
        .chain(lifecycles.iter().flat_map(|one| one.states.iter().map(|state| node_id(&one.id, &state.id))))
        .collect();
    let rules: Vec<LaneRule> = widest(input.rules, |one| (one.rule.id.as_str(), &one.rule.scope, &one.kept))
        .into_iter()
        .filter_map(|one| drawn_rule(one, &defs))
        .filter(|(rule, workflow)| {
            workflow.as_deref().is_none_or(|one| one == input.workflow)
                && drawn.contains(&node_id(&rule.from.lane, &rule.from.item))
                && drawn.contains(&node_id(&rule.to.lane, &rule.to.item))
        })
        .map(|(rule, _)| rule)
        .collect();
    let step_nodes = steps
        .iter()
        .map(|step| node(node_id("workflow", &step.id), &step.label, step.col, LaneNodeRole::Step, LaneNodeKind::Step))
        .collect();
    let pairs = steps.windows(2).map(|two| (node_id("workflow", &two[0].id), node_id("workflow", &two[1].id))).collect();
    let (nodes, moves) = with_ends(step_nodes, pairs, false, false);
    let flow = LaneView {
        id: "workflow".into(),
        kind: LaneKind::Workflow,
        title: input.workflow.to_string(),
        source: "Workflow".into(),
        what: None,
        note: None,
        ticket: false,
        ended: false,
        changed: None,
        nodes,
        moves,
    };
    let lanes = std::iter::once(flow)
        .chain(lifecycles.iter().map(|one| {
            let nodes = one
                .states
                .iter()
                .map(|state| {
                    let mut drawn = node(node_id(&one.id, &state.id), &state.label, state.col, LaneNodeRole::State, kind_of_state(one));
                    drawn.end = state.end;
                    drawn
                })
                .collect();
            lane_of_lifecycle(one, &one.id, &one.label, None, nodes)
        }))
        .collect();
    let edges = rules
        .iter()
        .map(|rule| LaneEdge {
            id: rule.id.clone(),
            rule: rule.id.clone(),
            from: node_id(&rule.from.lane, &rule.from.item),
            to: node_id(&rule.to.lane, &rule.to.item),
            label: rule.label.clone(),
            waits: None,
            fired_at: None,
            autonomy: rule.autonomy,
            status: LaneOutcome::Waiting,
            did: Vec::new(),
            changed: None,
            quiet: false,
        })
        .collect();
    JobLanesView {
        workflow: input.workflow.to_string(),
        at: -1,
        working: false,
        skipped: Vec::new(),
        job: JobLanesHeld {
            copy: LaneCopy { steps, lifecycles, rules },
            instances: Vec::new(),
            fired: Vec::new(),
            changes: Vec::new(),
            waiting: Vec::new(),
        },
        lanes,
        rules: edges,
    }
}
