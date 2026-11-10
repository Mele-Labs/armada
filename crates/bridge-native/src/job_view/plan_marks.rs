//! The plan as it reads with the person's marks on it (Electron's `plan-marks.ts`). A mark is an edit or
//! a note he has made and not sent: an edited title shows what it was, a removed task is struck, an
//! added one is marked new, a note is pinned. **Nothing here changes the plan**; [`view_of`] lays the
//! marks over it, and the marks go to the agent as one `send_plan_changes`.

use std::sync::atomic::{AtomicU32, Ordering};

use ipc::{PlanMark, PlanTaskState, SessionPlan, SessionPlanTask};

/// What the agent changed on one group or task in the plan it recorded again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Since {
    Added,
    Renamed(String),
}

/// A note he pinned and has not sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Note {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ViewTask {
    pub id: String,
    pub text: String,
    pub state: PlanTaskState,
    /// The group the task is marked under: where the plan has it, or where he added it.
    pub home: String,
    /// What the text was before he renamed it.
    pub was: Option<String>,
    pub added: bool,
    pub removed: bool,
    /// The title of the group it came from.
    pub moved_from: Option<String>,
    pub notes: Vec<Note>,
    pub since: Option<Since>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ViewGroup {
    pub id: String,
    pub title: String,
    pub agent: String,
    pub was: Option<String>,
    pub agent_was: Option<String>,
    pub added: bool,
    pub removed: bool,
    pub notes: Vec<Note>,
    pub tasks: Vec<ViewTask>,
    /// Tasks the agent took out of the plan it recorded again.
    pub gone: Vec<String>,
    pub since: Option<Since>,
}

/// A group, or a task of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Target {
    pub group: String,
    pub task: Option<String>,
}

impl Target {
    pub fn group(group: &str) -> Target {
        Target { group: group.to_string(), task: None }
    }

    pub fn task(group: &str, task: &str) -> Target {
        Target { group: group.to_string(), task: Some(task.to_string()) }
    }

    fn is(&self, group: &str, task: &Option<String>) -> bool {
        self.group == group && &self.task == task
    }
}

static MADE: AtomicU32 = AtomicU32::new(0);

/// An id for what he adds, which the agent does not know yet.
pub(super) fn new_id(prefix: &str) -> String {
    format!("{prefix}-new-{}", MADE.fetch_add(1, Ordering::Relaxed) + 1)
}

fn notes_on(marks: &[PlanMark], target: &Target) -> Vec<Note> {
    marks
        .iter()
        .filter_map(|mark| match mark {
            PlanMark::Note { id, group, task, text } if target.is(group, task) => Some(Note { id: id.clone(), text: text.clone() }),
            _ => None,
        })
        .collect()
}

fn renamed<'a>(marks: &'a [PlanMark], target: &Target) -> Option<(&'a str, &'a str)> {
    marks.iter().find_map(|mark| match mark {
        PlanMark::Rename { group, task, from, to } if target.is(group, task) => Some((from.as_str(), to.as_str())),
        _ => None,
    })
}

fn is_removed(marks: &[PlanMark], target: &Target) -> bool {
    marks.iter().any(|mark| matches!(mark, PlanMark::Remove { group, task } if target.is(group, task)))
}

fn moved<'a>(marks: &'a [PlanMark], task: &str) -> Option<&'a str> {
    marks.iter().find_map(|mark| match mark {
        PlanMark::MoveTask { task: one, to, .. } if one == task => Some(to.as_str()),
        _ => None,
    })
}

fn since(plan: &SessionPlan, id: &str) -> Option<Since> {
    let changes = plan.changes.as_ref()?;
    if changes.added.iter().any(|one| one == id) {
        return Some(Since::Added);
    }
    changes.renamed.iter().find(|one| one.id == id).map(|one| Since::Renamed(one.from.clone()))
}

fn task_of(plan: &SessionPlan, marks: &[PlanMark], group: &str, task: &SessionPlanTask) -> ViewTask {
    let target = Target::task(group, &task.id);
    let rename = renamed(marks, &target);
    ViewTask {
        id: task.id.clone(),
        text: rename.map(|(_, to)| to.to_string()).unwrap_or_else(|| task.text.clone()),
        state: task.state,
        home: group.to_string(),
        was: rename.map(|(from, _)| from.to_string()),
        added: false,
        removed: is_removed(marks, &target),
        moved_from: None,
        notes: notes_on(marks, &target),
        since: since(plan, &task.id),
    }
}

/// The plan with his marks laid over it.
pub(super) fn view_of(plan: &SessionPlan, marks: &[PlanMark]) -> Vec<ViewGroup> {
    // The plan's groups, then the groups he added, which hand to a general-purpose agent until he says otherwise.
    let mut all: Vec<(String, String, String, Vec<&SessionPlanTask>, bool)> =
        plan.groups.iter().map(|group| (group.id.clone(), group.title.clone(), group.agent.clone(), group.tasks.iter().collect(), false)).collect();
    for mark in marks {
        if let PlanMark::AddGroup { id, text } = mark {
            all.push((id.clone(), text.clone(), "general-purpose".to_string(), Vec::new(), true));
        }
    }
    let title_of = |id: &str| all.iter().find(|one| one.0 == id).map(|one| one.1.clone()).unwrap_or_else(|| id.to_string());
    all.iter()
        .map(|(id, title, agent, tasks, added)| {
            let target = Target::group(id);
            let rename = renamed(marks, &target);
            let agent_mark = marks.iter().find_map(|mark| match mark {
                PlanMark::Subagent { group, from, to } if group == id => Some((from.clone(), to.clone())),
                _ => None,
            });
            let kept = tasks.iter().filter(|task| moved(marks, &task.id).is_none()).map(|task| task_of(plan, marks, id, task));
            let comes = plan.groups.iter().flat_map(|from| from.tasks.iter().map(move |task| (from, task))).filter_map(|(from, task)| {
                (moved(marks, &task.id) == Some(id.as_str())).then(|| ViewTask { moved_from: Some(title_of(&from.id)), ..task_of(plan, marks, &from.id, task) })
            });
            let adds = marks.iter().filter_map(|mark| match mark {
                PlanMark::AddTask { group, id: task, text } if group == id => Some(ViewTask {
                    id: task.clone(),
                    text: text.clone(),
                    state: PlanTaskState::Open,
                    home: id.clone(),
                    was: None,
                    added: true,
                    removed: false,
                    moved_from: None,
                    notes: notes_on(marks, &Target::task(id, task)),
                    since: None,
                }),
                _ => None,
            });
            ViewGroup {
                id: id.clone(),
                title: rename.map(|(_, to)| to.to_string()).unwrap_or_else(|| title.clone()),
                agent: agent_mark.as_ref().map(|(_, to)| to.clone()).unwrap_or_else(|| agent.clone()),
                was: rename.map(|(from, _)| from.to_string()),
                agent_was: agent_mark.map(|(from, _)| from),
                added: *added,
                removed: is_removed(marks, &target),
                notes: notes_on(marks, &target),
                tasks: kept.chain(comes).chain(adds).collect(),
                gone: plan.changes.iter().flat_map(|changes| &changes.removed).filter(|one| &one.group == id).map(|one| one.text.clone()).collect(),
                since: since(plan, id),
            }
        })
        .collect()
}

/// Whether `target` is something he added, which a remove takes back instead of striking.
fn is_added(marks: &[PlanMark], target: &Target) -> bool {
    marks.iter().any(|mark| match (mark, &target.task) {
        (PlanMark::AddGroup { id, .. }, None) => id == &target.group,
        (PlanMark::AddTask { id, .. }, Some(task)) => id == task,
        _ => false,
    })
}

/// The text the target had before any rename of his.
fn name_of(plan: &SessionPlan, target: &Target) -> String {
    let group = plan.groups.iter().find(|one| one.id == target.group);
    match &target.task {
        None => group.map(|group| group.title.clone()).unwrap_or_default(),
        Some(task) => group.and_then(|group| group.tasks.iter().find(|one| &one.id == task)).map(|task| task.text.clone()).unwrap_or_default(),
    }
}

/// Renames a target. A name put back, or an item he added, leaves no rename.
pub(super) fn rename(plan: &SessionPlan, marks: &[PlanMark], target: &Target, to: &str) -> Vec<PlanMark> {
    let text = to.trim();
    if text.is_empty() {
        return marks.to_vec();
    }
    if is_added(marks, target) {
        return marks
            .iter()
            .map(|mark| match (mark, &target.task) {
                (PlanMark::AddGroup { id, .. }, None) if id == &target.group => PlanMark::AddGroup { id: id.clone(), text: text.to_string() },
                (PlanMark::AddTask { group, id, .. }, Some(task)) if id == task => PlanMark::AddTask { group: group.clone(), id: id.clone(), text: text.to_string() },
                _ => mark.clone(),
            })
            .collect();
    }
    let from = name_of(plan, target);
    let mut rest: Vec<PlanMark> = marks.iter().filter(|mark| !matches!(mark, PlanMark::Rename { group, task, .. } if target.is(group, task))).cloned().collect();
    if text != from {
        rest.push(PlanMark::Rename { group: target.group.clone(), task: target.task.clone(), from, to: text.to_string() });
    }
    rest
}

/// Removes a target, or takes the removal back. Something he added is simply taken out, with what hung on it.
pub(super) fn remove(marks: &[PlanMark], target: &Target) -> Vec<PlanMark> {
    if is_added(marks, target) {
        let id = target.task.as_deref().unwrap_or(&target.group);
        return marks
            .iter()
            .filter(|mark| match mark {
                PlanMark::AddGroup { id: one, .. } | PlanMark::AddTask { id: one, .. } if one == id => false,
                PlanMark::Note { group, task, .. } | PlanMark::Rename { group, task, .. } if target.is(group, task) => false,
                PlanMark::AddTask { group, .. } if target.task.is_none() && group == id => false,
                _ => true,
            })
            .cloned()
            .collect();
    }
    if is_removed(marks, target) {
        marks.iter().filter(|mark| !matches!(mark, PlanMark::Remove { group, task } if target.is(group, task))).cloned().collect()
    } else {
        let mut next = marks.to_vec();
        next.push(PlanMark::Remove { group: target.group.clone(), task: target.task.clone() });
        next
    }
}

pub(super) fn add_task(marks: &[PlanMark], group: &str, text: &str) -> Vec<PlanMark> {
    let mut next = marks.to_vec();
    if !text.trim().is_empty() {
        next.push(PlanMark::AddTask { group: group.to_string(), id: new_id("t"), text: text.trim().to_string() });
    }
    next
}

pub(super) fn add_group(marks: &[PlanMark], title: &str) -> Vec<PlanMark> {
    let mut next = marks.to_vec();
    if !title.trim().is_empty() {
        next.push(PlanMark::AddGroup { id: new_id("g"), text: title.trim().to_string() });
    }
    next
}

/// Moves a task to another group; back where it was, it is not moved.
pub(super) fn move_task(plan: &SessionPlan, marks: &[PlanMark], task: &str, to: &str) -> Vec<PlanMark> {
    let Some(home) = plan.groups.iter().find(|group| group.tasks.iter().any(|one| one.id == task)) else { return marks.to_vec() };
    let mut rest: Vec<PlanMark> = marks.iter().filter(|mark| !matches!(mark, PlanMark::MoveTask { task: one, .. } if one == task)).cloned().collect();
    if to != home.id {
        rest.push(PlanMark::MoveTask { task: task.to_string(), from: home.id.clone(), to: to.to_string() });
    }
    rest
}

pub(super) fn set_agent(plan: &SessionPlan, marks: &[PlanMark], group: &str, to: &str) -> Vec<PlanMark> {
    let from = plan.groups.iter().find(|one| one.id == group).map(|one| one.agent.clone());
    let mut rest: Vec<PlanMark> = marks.iter().filter(|mark| !matches!(mark, PlanMark::Subagent { group: one, .. } if one == group)).cloned().collect();
    if let Some(from) = from.filter(|from| from != to) {
        rest.push(PlanMark::Subagent { group: group.to_string(), from, to: to.to_string() });
    }
    rest
}

pub(super) fn add_note(marks: &[PlanMark], target: &Target, text: &str) -> Vec<PlanMark> {
    let mut next = marks.to_vec();
    if !text.trim().is_empty() {
        next.push(PlanMark::Note { id: new_id("n"), group: target.group.clone(), task: target.task.clone(), text: text.trim().to_string() });
    }
    next
}

pub(super) fn drop_note(marks: &[PlanMark], id: &str) -> Vec<PlanMark> {
    marks.iter().filter(|mark| !matches!(mark, PlanMark::Note { id: one, .. } if one == id)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> SessionPlan {
        let json = r#"{
            "version": 2, "summary": "", "recorded_at": "2026-11-09T13:40:00Z", "approved": false, "scope": [],
            "groups": [
                {"id": "g1", "agent": "rust-engineer", "title": "Cap the delay", "paths": [], "tasks": [
                    {"id": "t1", "text": "Add the cap", "state": "open"},
                    {"id": "t2", "text": "Read the cap", "state": "open"}
                ]},
                {"id": "g2", "agent": "test-writer", "title": "Hold the curve", "paths": [], "tasks": [
                    {"id": "t3", "text": "Test each delay", "state": "open"}
                ]}
            ],
            "changes": {"added": ["t3"], "renamed": [{"id": "g2", "from": "Test the curve"}], "moved": [], "removed": [{"group": "g2", "text": "Test zero"}]}
        }"#;
        ipc::decode::<SessionPlan>("SessionPlan", json.as_bytes()).expect("the plan decodes")
    }

    #[test]
    fn a_rename_shows_what_it_was_and_putting_the_name_back_leaves_no_mark() {
        let plan = plan();
        let marks = rename(&plan, &[], &Target::task("g1", "t1"), "Add the thirty second cap");
        let task = &view_of(&plan, &marks)[0].tasks[0];
        assert_eq!((task.text.as_str(), task.was.as_deref()), ("Add the thirty second cap", Some("Add the cap")));
        assert!(rename(&plan, &marks, &Target::task("g1", "t1"), "Add the cap").is_empty());
    }

    #[test]
    fn remove_strikes_and_a_second_press_keeps_it() {
        let plan = plan();
        let marks = remove(&[], &Target::task("g1", "t2"));
        assert!(view_of(&plan, &marks)[0].tasks[1].removed);
        assert!(remove(&marks, &Target::task("g1", "t2")).is_empty());
    }

    #[test]
    fn an_added_group_is_taken_out_whole_with_its_tasks() {
        let plan = plan();
        let marks = add_group(&[], "Write it down");
        let added = view_of(&plan, &marks)[2].clone();
        assert!(added.added);
        assert_eq!(added.agent, "general-purpose");
        let marks = add_task(&marks, &added.id, "Say the cap in the docs");
        assert_eq!(view_of(&plan, &marks)[2].tasks.len(), 1);
        assert!(remove(&marks, &Target::group(&added.id)).is_empty());
    }

    #[test]
    fn a_moved_task_leaves_its_group_and_says_where_it_came_from() {
        let plan = plan();
        let marks = move_task(&plan, &[], "t1", "g2");
        let view = view_of(&plan, &marks);
        assert_eq!(view[0].tasks.len(), 1);
        assert_eq!(view[1].tasks[1].moved_from.as_deref(), Some("Cap the delay"));
        assert!(move_task(&plan, &marks, "t1", "g1").is_empty());
    }

    #[test]
    fn a_subagent_change_shows_the_old_type_and_back_is_no_change() {
        let plan = plan();
        let marks = set_agent(&plan, &[], "g1", "judge");
        let group = &view_of(&plan, &marks)[0];
        assert_eq!((group.agent.as_str(), group.agent_was.as_deref()), ("judge", Some("rust-engineer")));
        assert!(set_agent(&plan, &marks, "g1", "rust-engineer").is_empty());
    }

    #[test]
    fn notes_pin_to_their_target_and_drop_by_id() {
        let plan = plan();
        let marks = add_note(&[], &Target::task("g1", "t2"), "  Read it at start  ");
        let note = view_of(&plan, &marks)[0].tasks[1].notes[0].clone();
        assert_eq!(note.text, "Read it at start");
        assert!(drop_note(&marks, &note.id).is_empty());
    }

    #[test]
    fn what_the_agent_changed_is_read_from_the_plan() {
        let view = view_of(&plan(), &[]);
        assert_eq!(view[1].since, Some(Since::Renamed("Test the curve".into())));
        assert_eq!(view[1].tasks[0].since, Some(Since::Added));
        assert_eq!(view[1].gone, vec!["Test zero".to_string()]);
    }
}
