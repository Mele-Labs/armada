//! A workflow's lanes as the Workflows page's editor draws them: `editorLanes` in `LaneCanvas/build.ts`, over the
//! Workspace `workspace.rs` reads off the pieces as `workspaceOf` and `placesOf` in Bridge's
//! `lifecycle-editing/` do. Served by `get_workflow_lanes`.
//!
//! **The default level's tab only.** A node and an edge say which levels below differ, and a lifecycle or a rule
//! a keep draft waits on says so; a rule only a draft keeps is drawn as proposed.
//!
//! **Departures.** A proposed rule is drawn only where it belongs to this workflow and both its ends are drawn,
//! as a held rule is; the TypeScript drew one whatever its ends. A step names no level that differs, since the
//! pieces carry no step layers. Two Workspaces that keep one id are taken in the order their pieces are read.

use std::collections::HashSet;

use ipc::{
    JobLanesHeld, JobLanesView, LaneCopy, LaneEdge, LaneKind, LaneLifecycle, LaneLifecycleState, LaneNodeKind, LaneNodeRole,
    LaneOutcome, LanePending, LaneRule, LaneStep, LaneView,
};

use super::draw::{kind_of_state, lane_of_lifecycle, node, node_id, with_ends};
use super::levels::{clash_at, differ_said, exists_at, is_workspace_level, level_name, levels_of, PiecesRead, DEFAULT};
use super::read::columns;
use super::workspace::{workspace_of, Drafts, LifecycleDef, RuleDef, StateDef, Workspace};

/// What a workflow's editor lanes are read from.
pub(crate) struct EditorInput<'a> {
    /// The workflow's id, which titles its lane.
    pub workflow: &'a str,
    /// Each step's id and label, in order.
    pub steps: &'a [(String, String)],
    /// The pieces of the repository asked about and of the other repositories of its Workspaces.
    pub read: &'a PiecesRead,
    /// What the repository's keep drafts would add.
    pub drafts: &'a Drafts,
}

/// The level whose tab is drawn.
const SCOPE: &str = DEFAULT;

/// Whether a state is there on the tab for `scope`: added at that level or above, and not left out of the piece
/// that tab runs.
fn state_exists_at(state: &StateDef, scope: &str, read: &PiecesRead) -> bool {
    exists_at(&state.scope, scope, read, &state.clash) && !state.absent.iter().any(|one| one == scope)
}

/// A lifecycle as the tab draws it, and the states it draws in the order the lane lists them.
struct Placed<'a> {
    lane: LaneLifecycle,
    states: Vec<&'a StateDef>,
}

/// The lifecycles the tab draws, each with its states in the columns every level's states and moves give.
/// **A state left out is stepped over**: whatever led into it now leads to whatever it led to.
fn lifecycles_for<'a>(workspace: &'a Workspace, scope: &str, read: &PiecesRead) -> Vec<(&'a LifecycleDef, Placed<'a>)> {
    workspace
        .lifecycles
        .iter()
        .filter(|one| exists_at(&one.scope, scope, read, &Vec::new()))
        .filter_map(|one| {
            let ids: Vec<&str> = one.states.iter().map(|state| state.id.as_str()).collect();
            let moves: Vec<(&str, &str)> = one.next.iter().map(|(from, to)| (from.as_str(), to.as_str())).collect();
            let cols = columns(&ids, &moves);
            let states: Vec<&StateDef> = one.states.iter().filter(|state| state_exists_at(state, scope, read)).collect();
            if states.is_empty() {
                return None;
            }
            let drawn = |id: &str| states.iter().any(|state| state.id == id);
            let mut next = one.next.clone();
            for gone in one.states.iter().filter(|state| !drawn(&state.id)) {
                let into: Vec<String> = next.iter().filter(|(_, to)| *to == gone.id).map(|(from, _)| from.clone()).collect();
                let out: Vec<String> = next.iter().filter(|(from, _)| *from == gone.id).map(|(_, to)| to.clone()).collect();
                next.retain(|(from, to)| *from != gone.id && *to != gone.id);
                for pair in into.iter().flat_map(|from| out.iter().map(move |to| (from.clone(), to.clone()))) {
                    if !next.contains(&pair) {
                        next.push(pair);
                    }
                }
            }
            next.retain(|(from, to)| drawn(from) && drawn(to));
            let lane = LaneLifecycle {
                id: one.id.clone(),
                label: one.label.clone(),
                from: one.from.to_string(),
                kind: one.kind,
                states: states
                    .iter()
                    .map(|state| LaneLifecycleState {
                        id: state.id.clone(),
                        label: state.label.clone(),
                        col: cols.get(state.id.as_str()).copied().unwrap_or_default() as u32,
                        end: state.end,
                    })
                    .collect(),
                next,
            };
            Some((one, Placed { lane, states }))
        })
        .collect()
}

/// The levels a mark names, by name: on the default's tab every one under it that differs; on a Workspace's, the
/// repositories it holds that differ; on a repository's own tab only that one.
fn marked(ids: &[String], scope: &str, read: &PiecesRead) -> Vec<String> {
    ids.iter()
        .filter(|id| {
            scope == DEFAULT
                || if is_workspace_level(scope) {
                    !is_workspace_level(id) && levels_of(id, read, &Vec::new()).iter().any(|level| level == scope)
                } else {
                    *id == scope
                }
        })
        .map(|id| level_name(read, id))
        .collect()
}

/// What a mark says of the Workspaces whose pieces tie on this tab: only a repository's tab sets any aside.
fn clash_said(clash: &super::levels::Clash, own: &[String], scope: &str, read: &PiecesRead) -> Option<String> {
    let names: Vec<String> = clash_at(clash, scope, read, own).iter().map(|level| level_name(read, level)).collect();
    (!names.is_empty()).then(|| differ_said(&names))
}

fn pending_of(drafts: &Drafts, key: String) -> Option<LanePending> {
    drafts.keeps.contains(&key).then_some(LanePending::Keep)
}

fn lane_rule(rule: &RuleDef) -> LaneRule {
    let drawn = &rule.drawn;
    LaneRule {
        id: rule.id.clone(),
        label: drawn.label.clone(),
        from: drawn.from.clone(),
        at: drawn.at,
        to: drawn.to.clone(),
        autonomy: drawn.autonomy,
        kept: rule.kept,
    }
}

/// The editor's lanes: the workflow on top, then a lane for each lifecycle the tab draws, its states in the
/// columns the moves between them give, and an edge for each rule whose ends are both drawn, each marked where a
/// level below differs or a draft waits. **No Job stands in them**, so `job` holds only the definitions they are
/// drawn from, and an edge waits on nothing.
pub(crate) fn editor_lanes(input: &EditorInput<'_>) -> JobLanesView {
    let (read, scope) = (input.read, SCOPE);
    let workspace = workspace_of(read, input.drafts);
    let placed = lifecycles_for(&workspace, scope, read);
    let steps: Vec<LaneStep> = input
        .steps
        .iter()
        .enumerate()
        .map(|(col, (id, label))| LaneStep { id: id.clone(), label: label.clone(), col: col as u32 })
        .collect();
    let drawn: HashSet<String> = steps
        .iter()
        .map(|step| node_id("workflow", &step.id))
        .chain(placed.iter().flat_map(|(_, one)| one.lane.states.iter().map(|state| node_id(&one.lane.id, &state.id))))
        .collect();
    let applies = |rule: &&RuleDef| {
        rule.drawn.workflow.as_deref().is_none_or(|one| one == input.workflow)
            && drawn.contains(&node_id(&rule.drawn.from.lane, &rule.drawn.from.item))
            && drawn.contains(&node_id(&rule.drawn.to.lane, &rule.drawn.to.item))
    };
    let held: Vec<&RuleDef> = workspace.rules.iter().filter(applies).collect();
    let proposed: Vec<&RuleDef> = workspace.drafted.iter().filter(applies).collect();
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
        pending: None,
        nodes,
        moves,
    };
    let lanes = std::iter::once(flow)
        .chain(placed.iter().map(|(_, one)| {
            let lane = &one.lane;
            let nodes = lane
                .states
                .iter()
                .zip(&one.states)
                .map(|(state, def)| {
                    let mut drawn = node(node_id(&lane.id, &state.id), &state.label, state.col, LaneNodeRole::State, kind_of_state(lane));
                    drawn.end = state.end;
                    drawn.differs = marked(&def.differs(), scope, read);
                    drawn.clash = clash_said(&def.clash, &def.owners(), scope, read);
                    drawn
                })
                .collect();
            let mut drawn = lane_of_lifecycle(lane, &lane.id, &lane.label, None, nodes);
            drawn.pending = pending_of(input.drafts, format!("lifecycle:{}", lane.id));
            drawn
        }))
        .collect();
    let edge = |rule: &RuleDef, proposed: bool| LaneEdge {
        id: rule.id.clone(),
        rule: rule.id.clone(),
        from: node_id(&rule.drawn.from.lane, &rule.drawn.from.item),
        to: node_id(&rule.drawn.to.lane, &rule.drawn.to.item),
        label: rule.drawn.label.clone(),
        waits: None,
        fired_at: None,
        autonomy: rule.drawn.autonomy,
        status: LaneOutcome::Waiting,
        did: Vec::new(),
        changed: None,
        quiet: false,
        differs: marked(&rule.differs(), scope, read),
        clash: clash_said(&rule.clash, &rule.owners(), scope, read),
        proposed,
        pending: pending_of(input.drafts, format!("rule:{}", rule.id)),
    };
    let edges = held.iter().map(|rule| edge(rule, false)).chain(proposed.iter().map(|rule| edge(rule, true))).collect();
    JobLanesView {
        workflow: input.workflow.to_string(),
        at: -1,
        working: false,
        skipped: Vec::new(),
        job: JobLanesHeld {
            copy: LaneCopy {
                steps,
                lifecycles: placed.into_iter().map(|(_, one)| one.lane).collect(),
                rules: held.iter().map(|rule| lane_rule(rule)).collect(),
            },
            instances: Vec::new(),
            fired: Vec::new(),
            changes: Vec::new(),
            waiting: Vec::new(),
        },
        lanes,
        rules: edges,
    }
}
