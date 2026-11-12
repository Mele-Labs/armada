//! The lanes and edges a Job draws, from what it holds: `jobLanes` in `LaneCanvas/build.ts`.

use std::collections::HashSet;

use ipc::{
    JobLanesHeld, LaneEdge, LaneInstance, LaneKind, LaneLifecycle, LaneLifecycleKind, LaneMoment, LaneNode, LaneNodeKind,
    LaneNodeRole, LaneOutcome, LaneView,
};

use super::read::CURRENT_STEP;

/// Where a Job stands in its workflow. `working` is the agent being on that step now: only then is the step lit.
pub(super) struct Standing<'a> {
    pub workflow: &'a str,
    pub at: i32,
    pub skipped: &'a [u32],
    pub working: bool,
}

/// The id a node is drawn under.
fn node_id(lane: &str, item: &str) -> String {
    format!("{lane}/{item}")
}

/// Where the workflow starts and ends, drawn as small circles at the ends of its lane: not steps, so nothing picks them.
const START_ITEM: &str = "$start";
const END_ITEM: &str = "$end";

fn kind_of_state(lifecycle: &LaneLifecycle) -> LaneNodeKind {
    match lifecycle.kind {
        LaneLifecycleKind::Tracker => LaneNodeKind::Tracker,
        LaneLifecycleKind::Forge => LaneNodeKind::Forge,
        LaneLifecycleKind::Watch => LaneNodeKind::Watch,
        LaneLifecycleKind::Job => LaneNodeKind::Job,
    }
}

fn node(id: String, label: &str, col: u32, role: LaneNodeRole, kind: LaneNodeKind) -> LaneNode {
    LaneNode {
        id,
        label: label.to_string(),
        col,
        role,
        kind,
        lit: false,
        here: false,
        passed: false,
        waiting: Vec::new(),
        end: false,
        skipped: false,
        ended: false,
        ticket: None,
        changed: None,
    }
}

/// The workflow's lane with a small circle at each end: where it starts and where it ends. They sit in the
/// columns of the first and the last step, and are no steps, so nothing picks them. `began` and `ended` are a
/// Job's: it has started its first step, and every step is over.
fn with_ends(steps: Vec<LaneNode>, pairs: Vec<(String, String)>, began: bool, ended: bool) -> (Vec<LaneNode>, Vec<(String, String)>) {
    let (Some(first), Some(last)) = (steps.first(), steps.last()) else { return (steps, pairs) };
    let mut start = node(node_id("workflow", START_ITEM), "Start", first.col, LaneNodeRole::Step, LaneNodeKind::Start);
    start.passed = began;
    let mut end = node(node_id("workflow", END_ITEM), "End", last.col, LaneNodeRole::Step, LaneNodeKind::End);
    end.here = ended;
    let moves = std::iter::once((start.id.clone(), first.id.clone()))
        .chain(pairs)
        .chain(std::iter::once((last.id.clone(), end.id.clone())))
        .collect();
    let nodes = std::iter::once(start).chain(steps).chain(std::iter::once(end)).collect();
    (nodes, moves)
}

fn lane_of_lifecycle(lifecycle: &LaneLifecycle, id: &str, title: &str, note: Option<&str>, nodes: Vec<LaneNode>) -> LaneView {
    LaneView {
        id: id.to_string(),
        kind: LaneKind::Lifecycle,
        title: title.to_string(),
        source: lifecycle.from.clone(),
        what: None,
        note: note.map(str::to_string),
        ticket: lifecycle.kind == LaneLifecycleKind::Tracker,
        ended: false,
        changed: None,
        nodes,
        moves: lifecycle.next.iter().map(|(from, to)| (node_id(id, from), node_id(id, to))).collect(),
    }
}

/// A Job's lanes: its workflow on top with the step it is at (lit while the agent works on it), then a lane for
/// each instance it holds, the state it stands in marked as where it is, and the ones it went through marked. A
/// rule that fired is an edge with its outcome and when; a rule still waiting is an edge naming what it waits
/// for. A rule turned off is not drawn.
pub(super) fn lanes(job: &JobLanesHeld, standing: &Standing<'_>) -> (Vec<LaneView>, Vec<LaneEdge>) {
    let copy = &job.copy;
    let change = |lane: &str, item: &str| job.changes.iter().find(|one| one.lane == lane && one.item == item).map(|one| one.text.clone());
    let label = |lane: &str, item: &str| {
        copy.lifecycles
            .iter()
            .find(|one| one.id == lane)
            .and_then(|one| one.states.iter().find(|state| state.id == item))
            .map_or_else(|| item.to_string(), |state| state.label.clone())
    };
    let step_nodes: Vec<LaneNode> = copy
        .steps
        .iter()
        .enumerate()
        .map(|(at, step)| {
            let at = at as i32;
            let mut one = node(node_id("workflow", &step.id), &step.label, step.col, LaneNodeRole::Step, LaneNodeKind::Step);
            if at == standing.at {
                if standing.working {
                    one.lit = true;
                } else {
                    one.here = true;
                }
                // What the Job's step waits on is the step it is at.
                one.waiting = job.waiting.clone();
            }
            one.passed = at < standing.at;
            one.skipped = standing.skipped.contains(&(at as u32));
            one.changed = change("workflow", &step.id);
            one
        })
        .collect();
    let pairs = copy.steps.windows(2).map(|two| (node_id("workflow", &two[0].id), node_id("workflow", &two[1].id))).collect();
    let (nodes, moves) = with_ends(step_nodes, pairs, standing.at >= 0, standing.at >= copy.steps.len() as i32);
    let flow = LaneView {
        id: "workflow".into(),
        kind: LaneKind::Workflow,
        title: standing.workflow.to_string(),
        source: "Workflow".into(),
        what: None,
        note: None,
        ticket: false,
        ended: false,
        changed: None,
        nodes,
        moves,
    };
    let mut lanes: Vec<LaneView> = Vec::new();
    // A Job with no workflow has no top lane.
    if !copy.steps.is_empty() {
        lanes.push(flow);
    }
    for instance in &job.instances {
        let Some(lifecycle) = copy.lifecycles.iter().find(|one| one.id == instance.lifecycle) else { continue };
        let nodes = lifecycle
            .states
            .iter()
            .map(|state| {
                let mut one = node(node_id(&instance.id, &state.id), &state.label, state.col, LaneNodeRole::State, kind_of_state(lifecycle));
                let here = state.id == instance.state;
                if here {
                    if instance.ended {
                        one.ended = true;
                    } else {
                        one.here = true;
                    }
                    one.ticket = instance.ticket.clone();
                }
                one.passed = !here && instance.visited.contains(&state.id);
                one.end = state.end;
                one.changed = change(&instance.id, &state.id);
                one
            })
            .collect();
        let mut lane = lane_of_lifecycle(lifecycle, &instance.id, &instance.label, instance.title.as_deref(), nodes);
        lane.what = Some(lifecycle.label.clone());
        lane.ended = instance.ended;
        lane.changed = change(&instance.lifecycle, "");
        lanes.push(lane);
    }
    let step_at = |id: &str| copy.steps.iter().position(|one| one.id == id).map_or(-1, |at| at as i64);
    // An edge is drawn only between two nodes that are drawn: a rule naming a state its lifecycle lacks, or a step the Job has none of, has nowhere to land.
    let drawn: HashSet<&str> = lanes.iter().flat_map(|lane| lane.nodes.iter().map(|one| one.id.as_str())).collect();
    let of = |lane: &str| -> Vec<Option<&LaneInstance>> {
        if lane == "workflow" {
            vec![None]
        } else {
            job.instances.iter().filter(|one| one.lifecycle == lane).map(Some).collect()
        }
    };
    let mut edges: Vec<LaneEdge> = Vec::new();
    for rule in &copy.rules {
        if rule.autonomy == ipc::LaneAutonomy::Off {
            continue;
        }
        let changed = change("rule", &rule.id);
        let mut first = true;
        for source in of(&rule.from.lane) {
            for target in of(&rule.to.lane) {
                let owner = target.or(source).map_or("", |one| one.id.as_str());
                let fired = job.fired.iter().find(|one| one.rule == rule.id && one.instance == owner);
                let reached = match source {
                    None => {
                        let at = step_at(&rule.from.item);
                        at < standing.at as i64 || (rule.at != LaneMoment::Ends && at == standing.at as i64)
                    }
                    Some(source) => source.visited.contains(&rule.from.item),
                };
                // A rule waits for its source only while it has not been reached: one reached and unfired has nothing left to show.
                if fired.is_none() && reached {
                    continue;
                }
                let status = fired.map_or(LaneOutcome::Waiting, |one| one.outcome);
                // A rule that hands to the step the Job is on lands on the one it was on when it fired, else the one it is on now.
                let landing = if rule.to.item == CURRENT_STEP {
                    fired
                        .and_then(|one| one.step.clone())
                        .or_else(|| {
                            let last = copy.steps.len().checked_sub(1)?;
                            copy.steps.get((standing.at.max(0) as usize).min(last)).map(|step| step.id.clone())
                        })
                        .unwrap_or_else(|| rule.to.item.clone())
                } else {
                    rule.to.item.clone()
                };
                let from = node_id(source.map_or("workflow", |one| one.id.as_str()), &rule.from.item);
                let to = node_id(target.map_or("workflow", |one| one.id.as_str()), &landing);
                // A rule that leaves a node and lands on the same one moves nothing on the lanes.
                if from == to || !drawn.contains(from.as_str()) || !drawn.contains(to.as_str()) {
                    continue;
                }
                let name = match source {
                    None => copy.steps.iter().find(|one| one.id == rule.from.item).map_or_else(|| rule.from.item.clone(), |one| one.label.clone()),
                    Some(_) => label(&rule.from.lane, &rule.from.item),
                };
                let waits = match source {
                    None => format!("{name} {}", if rule.at == LaneMoment::Ends { "ends" } else { "starts" }),
                    Some(source) => format!("{}: {name}", source.label),
                };
                edges.push(LaneEdge {
                    id: format!(
                        "{}:{}{}",
                        rule.id,
                        source.or(target).map_or("workflow", |one| one.id.as_str()),
                        match (source, target) {
                            (Some(_), Some(target)) => format!(">{}", target.id),
                            _ => String::new(),
                        }
                    ),
                    rule: rule.id.clone(),
                    from,
                    to,
                    label: rule.label.clone(),
                    waits: fired.is_none().then_some(waits),
                    fired_at: fired.map(|one| one.at.clone()),
                    autonomy: rule.autonomy,
                    status,
                    did: fired.map(|one| one.did.clone()).unwrap_or_default(),
                    changed: changed.clone(),
                    quiet: !first,
                });
                first = false;
            }
        }
    }
    (lanes, edges)
}
