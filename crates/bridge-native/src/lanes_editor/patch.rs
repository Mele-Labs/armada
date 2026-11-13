//! The lanes as the draft would leave them. Fleet reads the lanes of what is saved; the editor lays the draft's
//! steps over them: a step's new name, a step added, a step taken out. What differs from the base is marked as the
//! canvas marks a changed node, so the draft shows on the canvas before anything is published.

use ipc::{JobLanesView, LaneKind, LaneNode, LaneNodeKind, LaneNodeRole};

use crate::settings::definition::steps_of;

/// The workflow lane's node for a step.
pub(super) fn node_id(step: &str) -> String {
    format!("workflow/{step}")
}

pub(super) fn step_node(step: &str, label: &str, col: u32) -> LaneNode {
    LaneNode {
        id: node_id(step),
        label: label.to_string(),
        col,
        role: LaneNodeRole::Step,
        kind: LaneNodeKind::Step,
        lit: false,
        here: false,
        passed: false,
        waiting: Vec::new(),
        end: false,
        skipped: false,
        ended: false,
        ticket: None,
        changed: Some("Added".into()),
        differs: Vec::new(),
        clash: None,
    }
}

/// `lanes` read for `base`, with `drafted`'s steps laid over it. The same view back where the two read alike.
pub fn patched(lanes: &JobLanesView, base: &str, drafted: &str) -> JobLanesView {
    let mut view = lanes.clone();
    if base == drafted {
        return view;
    }
    let (was, now) = (steps_of(base), steps_of(drafted));
    let Some(at) = view.lanes.iter().position(|lane| lane.kind == LaneKind::Workflow) else { return view };

    // A step gone: its node and the rules drawn from or to it. Its column stays empty, so the rest do not move.
    for (id, _) in was.iter().filter(|(id, _)| !now.iter().any(|(each, _)| each == id)) {
        let gone = node_id(id);
        view.lanes[at].nodes.retain(|node| node.id != gone);
        view.lanes[at].moves.retain(|(from, to)| *from != gone && *to != gone);
        view.rules.retain(|edge| edge.from != gone && edge.to != gone);
    }

    for (n, (id, label)) in now.iter().enumerate() {
        let node = node_id(id);
        match was.iter().find(|(each, _)| each == id) {
            Some((_, before)) if before != label => {
                if let Some(one) = view.lanes[at].nodes.iter_mut().find(|one| one.id == node) {
                    one.changed = Some(format!("Renamed from {before}"));
                    one.label = label.clone();
                }
            }
            Some(_) => {}
            None => {
                if view.lanes[at].nodes.iter().any(|one| one.id == node) {
                    continue;
                }
                // Where it stands: after the nearest step before it that is drawn, else before the nearest after
                // it, else at the end of the steps. The last step of the workflow is at the end of the steps.
                let drawn = |step: &str| view.lanes[at].nodes.iter().find(|one| one.id == node_id(step) && one.role == LaneNodeRole::Step).map(|one| one.col);
                let end = view.lanes[at].nodes.iter().filter(|one| one.kind == LaneNodeKind::Step).map(|one| one.col + 1).max().unwrap_or(0);
                let last = n + 1 == now.len();
                let col = if last { end } else { now[..n].iter().rev().find_map(|(each, _)| drawn(each)).map(|col| col + 1).or_else(|| now[n + 1..].iter().find_map(|(each, _)| drawn(each))).unwrap_or(end) };
                // A column is every lane's: the ones from there on make room, but where the workflow starts.
                for lane in &mut view.lanes {
                    for one in lane.nodes.iter_mut().filter(|one| one.col >= col && one.kind != LaneNodeKind::Start) {
                        one.col += 1;
                    }
                }
                // Where the workflow ends follows its last step.
                if last {
                    for one in view.lanes[at].nodes.iter_mut().filter(|one| one.kind == LaneNodeKind::End) {
                        one.col = col;
                    }
                }
                view.lanes[at].nodes.push(step_node(id, label, col));
            }
        }
    }
    view
}

/// The lanes with each state the draft renames, `(lifecycle, state, name)`, under its new name and marked as a renamed
/// step is.
pub fn with_states_renamed(mut view: JobLanesView, labels: &[(String, String, String)]) -> JobLanesView {
    for (lifecycle, state, label) in labels {
        let id = format!("{lifecycle}/{state}");
        let found = view.lanes.iter_mut().filter(|lane| lane.kind == LaneKind::Lifecycle).flat_map(|lane| lane.nodes.iter_mut()).find(|one| one.id == id);
        if let Some(one) = found.filter(|one| one.label != *label) {
            one.changed = Some(format!("Renamed from {}", one.label));
            one.label = label.clone();
        }
    }
    view
}

/// The lanes with each lifecycle the draft adds drawn under the others: its states in columns from the first, one node
/// each, and the moves it expects between them. A lane the lanes already draw (one a publish has kept) is left as read.
pub fn with_lifecycles(mut view: JobLanesView, added: &[ipc::LifecycleDefinition]) -> JobLanesView {
    for one in added {
        if view.lanes.iter().any(|lane| lane.id == one.id) {
            continue;
        }
        let nodes = one
            .states
            .iter()
            .enumerate()
            .map(|(col, state)| LaneNode {
                id: format!("{}/{}", one.id, state.id),
                label: state.label.clone(),
                col: col as u32,
                role: LaneNodeRole::State,
                kind: super::lane::node_kind(&state.read),
                lit: false,
                here: false,
                passed: false,
                waiting: Vec::new(),
                end: one.ends.contains(&state.id),
                skipped: false,
                ended: false,
                ticket: None,
                changed: None,
                differs: Vec::new(),
                clash: None,
            })
            .collect();
        let moves = one.transitions.iter().map(|one_move| (format!("{}/{}", one.id, one_move.from), format!("{}/{}", one.id, one_move.to))).collect();
        view.lanes.push(ipc::LaneView {
            id: one.id.clone(),
            kind: LaneKind::Lifecycle,
            title: one.name.clone(),
            source: super::lane::source_of(one).to_string(),
            what: None,
            note: None,
            ticket: false,
            ended: false,
            changed: None,
            pending: None,
            nodes,
            moves,
        });
    }
    view
}

/// A rule the draft sets, and the rule that runs where there is one.
pub struct RuleChange {
    pub was: Option<ipc::LifecycleRule>,
    pub now: ipc::LifecycleRule,
}

/// The lanes with each rule the draft sets laid over them: a rule that runs has its chip's words, autonomy and ends changed
/// and is marked as a step is; a rule the draft adds gets a chip where its moment and its act name two nodes the lanes
/// draw. A rule the lanes do not draw (a gate or the Job sets it off) is not drawn here either.
pub fn with_rules(mut view: JobLanesView, changes: &[RuleChange], vocab: &super::rule::Vocab, marked: bool) -> JobLanesView {
    use super::rule::{autonomy_of, ends_of, when_phrase};
    for change in changes {
        let rule = &change.now;
        let drawn = |id: &str| view.lanes.iter().flat_map(|lane| lane.nodes.iter()).any(|node| node.id == id);
        let ends = ends_of(rule).filter(|(from, to)| drawn(from) && drawn(to));
        let autonomy = match autonomy_of(vocab, rule) {
            "asks" => ipc::LaneAutonomy::Asks,
            "off" => ipc::LaneAutonomy::Off,
            _ => ipc::LaneAutonomy::Acts,
        };
        let mark = marked.then(|| if change.was.is_some() { "Changed" } else { "Added" }.to_string());
        let waits = Some(when_phrase(vocab, &rule.when));
        match view.rules.iter().position(|edge| edge.rule == rule.id && !edge.quiet) {
            Some(at) => {
                let edge = &mut view.rules[at];
                edge.label = rule.name.clone();
                edge.autonomy = autonomy;
                edge.waits = edge.waits.as_ref().and(waits);
                edge.changed = mark;
                if let Some((from, to)) = ends {
                    edge.id = format!("{}:{from}>{to}", rule.id);
                    (edge.from, edge.to) = (from, to);
                }
            }
            None => {
                let Some((from, to)) = ends else { continue };
                view.rules.push(ipc::LaneEdge {
                    id: format!("{}:{from}>{to}", rule.id),
                    rule: rule.id.clone(),
                    from,
                    to,
                    label: rule.name.clone(),
                    waits,
                    fired_at: None,
                    autonomy,
                    status: ipc::LaneOutcome::Waiting,
                    did: Vec::new(),
                    changed: mark,
                    quiet: false,
                    differs: Vec::new(),
                    clash: None,
                    proposed: false,
                    pending: None,
                });
            }
        }
    }
    view
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::definition::{with_label, with_words};

    const BASE: &str = r#"{"version":1,"workflow_id":"prototype","name":"prototype","steps":[{"id":"plan","label":"Plan"},{"id":"build","label":"Build"},{"id":"land","label":"Land"}]}"#;

    fn lanes() -> JobLanesView {
        ipc::decode("JobLanesView", include_bytes!("../settings/mock-lanes.json")).expect("the mock lanes decode")
    }

    fn node<'a>(view: &'a JobLanesView, id: &str) -> Option<&'a LaneNode> {
        view.lanes.iter().flat_map(|lane| lane.nodes.iter()).find(|one| one.id == id)
    }

    #[test]
    fn a_draft_that_reads_as_the_base_leaves_the_lanes_as_they_were() {
        let read = lanes();
        assert_eq!(patched(&read, BASE, BASE), read);
        // Words are not drawn on a node: the lanes stand.
        let said = with_words(BASE, "build", "Build it.").expect("a step");
        assert_eq!(patched(&read, BASE, &said), read);
    }

    #[test]
    fn a_step_renamed_is_drawn_with_its_new_name_and_marked() {
        let read = lanes();
        let said = with_label(BASE, "build", "Make").expect("a step");
        let view = patched(&read, BASE, &said);
        let build = node(&view, "workflow/build").expect("drawn");
        assert_eq!((build.label.as_str(), build.changed.as_deref()), ("Make", Some("Renamed from Build")));
        assert_eq!(node(&read, "workflow/build").map(|one| one.changed.clone()), Some(None));
    }

    #[test]
    fn a_step_added_after_build_takes_its_column_and_every_lane_makes_room() {
        let read = lanes();
        let said = BASE.replace(r#"{"id":"land""#, r#"{"id":"review","label":"Review"},{"id":"land""#);
        let view = patched(&read, BASE, &said);
        let (build, review) = (node(&view, "workflow/build").expect("build"), node(&view, "workflow/review").expect("review"));
        assert_eq!((review.col, review.changed.as_deref(), review.role), (build.col + 1, Some("Added"), LaneNodeRole::Step));
        // What stood in that column and after it moved one on, in every lane, and the start did not.
        let was = |id: &str| node(&read, id).map(|one| one.col);
        assert_eq!(node(&view, "workflow/walk").map(|one| one.col), was("workflow/walk").map(|col| col + 1));
        assert_eq!(node(&view, "pull_request/open").map(|one| one.col), was("pull_request/open").map(|col| col + 1));
        assert_eq!(node(&view, "ticket/in_progress").map(|one| one.col), was("ticket/in_progress"));
        assert_eq!(node(&view, "workflow/:start").map(|one| one.col), was("workflow/:start"));
    }

    #[test]
    fn a_step_added_at_the_end_of_the_workflow_is_drawn_after_its_last_step_and_the_end_follows_it() {
        let read = lanes();
        let said = BASE.replace(r#"{"id":"land","label":"Land"}"#, r#"{"id":"land","label":"Land"},{"id":"retro","label":"Retro"}"#);
        let view = patched(&read, BASE, &said);
        let (land, retro, end) = (node(&view, "workflow/land").expect("land"), node(&view, "workflow/retro").expect("retro"), node(&view, "workflow/:end").expect("end"));
        assert_eq!((retro.col, end.col), (land.col + 1, land.col + 1));
        assert_eq!(land.col, node(&read, "workflow/land").map_or(0, |one| one.col));
    }

    #[test]
    fn a_step_taken_out_leaves_the_lanes_without_its_node_or_its_rules() {
        let read = lanes();
        let said = BASE.replace(r#"{"id":"build","label":"Build"},"#, "");
        let view = patched(&read, BASE, &said);
        assert!(node(&view, "workflow/build").is_none());
        assert!(view.rules.iter().all(|edge| edge.from != "workflow/build" && edge.to != "workflow/build"));
        assert!(read.rules.iter().any(|edge| edge.from == "workflow/build"));
        assert_eq!(node(&view, "workflow/land").map(|one| one.col), node(&read, "workflow/land").map(|one| one.col));
    }
}
