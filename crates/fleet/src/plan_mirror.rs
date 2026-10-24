//! What it takes to make a Job's beads issue say what its plan says: the writes, in order, from
//! the plan as kept and the beads already filed for it. `docs/concepts/session/the-plan.md`.
//!
//! **Pure, and Fleet's plan is the source.** Nothing is read back from the backlog. A bead is
//! written where the plan differs from what was last written to it (the row's `status`, `title` and
//! `blocked_by`), so a write that failed is simply still different at the next look. **A task's
//! status is `state_of`'s**: Fleet's reading of edits it saw and the agent's task list, never
//! a word the agent said about progress. `plan_mirroring` runs the steps.

use adapter_traits::BeadState;
use ipc::PlanTaskState;
use store::{KeptBead, KeptPlan, KeptPlanTask};

use crate::plan_record::state_of;

/// What a close says for a task or group the plan finished.
pub(crate) const FINISHED: &str = "done in the plan";

/// What a close says for one the plan no longer holds.
pub(crate) const REMOVED: &str = "removed from the plan";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// File `id` as a child of the group `parent`, or of the issue where there is none.
    Create { id: String, parent: Option<String>, title: String },
    Retitle { id: String, title: String },
    /// Order the group `id` after the group `after`, or after none, undoing the order it had.
    Order { id: String, after: Option<String> },
    Set { id: String, state: BeadState },
    /// The plan no longer holds `id`: close its bead and forget the row.
    Retire { id: String },
}

fn wanted(task: &KeptPlanTask) -> BeadState {
    match state_of(task).0 {
        PlanTaskState::Working => BeadState::InProgress,
        PlanTaskState::Done | PlanTaskState::HandedIn => BeadState::Closed,
        PlanTaskState::Open | PlanTaskState::Failed => BeadState::Open,
    }
}

/// The writes `plan` asks of `held`, in the order to make them: a group before its tasks, every
/// bead before the order between groups, those before the states, and what went last.
///
/// **Structure follows the approved version only**: groups, tasks and their order are filed once
/// the head of the plan is approved. A version recorded since and not yet approved changes nothing
/// but the states of beads already filed, since a task's state is the plan's whichever version holds it.
pub(crate) fn steps(plan: &KeptPlan, held: &[KeptBead]) -> Vec<Step> {
    let have = |id: &str| held.iter().find(|one| one.plan_id == id);
    let approved = plan.approved_at.is_some();
    let mut out = Vec::new();

    if approved {
        for group in &plan.groups {
            out.extend(filed(&group.id, None, &group.title, have(&group.id)));
            for task in &group.tasks {
                out.extend(filed(&task.id, Some(&group.id), &task.text, have(&task.id)));
            }
        }
        let mut before: Option<&str> = None;
        for group in &plan.groups {
            let was = have(&group.id).map_or("", |row| row.blocked_by.as_str());
            if was != before.unwrap_or_default() {
                out.push(Step::Order { id: group.id.clone(), after: before.map(str::to_string) });
            }
            before = Some(&group.id);
        }
    }

    for group in &plan.groups {
        let tasks: Vec<(&str, BeadState)> = group.tasks.iter().map(|task| (task.id.as_str(), wanted(task))).collect();
        let finished = !tasks.is_empty() && tasks.iter().all(|(_, state)| *state == BeadState::Closed);
        let group_state = if finished { BeadState::Closed } else { BeadState::Open };
        for (id, state) in tasks.into_iter().chain(std::iter::once((group.id.as_str(), group_state))) {
            // A bead not filed yet is filed open.
            let was = have(id).map_or(approved.then_some("open"), |row| Some(row.status.as_str()));
            if was.is_some_and(|was| was != state.word()) {
                out.push(Step::Set { id: id.to_string(), state });
            }
        }
    }

    if approved {
        let in_plan = |id: &str| plan.groups.iter().any(|group| group.id == id || group.tasks.iter().any(|task| task.id == id));
        out.extend(held.iter().filter(|row| !in_plan(&row.plan_id)).map(|row| Step::Retire { id: row.plan_id.clone() }));
    }
    out
}

fn filed(id: &str, parent: Option<&str>, title: &str, row: Option<&KeptBead>) -> Option<Step> {
    match row {
        None => Some(Step::Create { id: id.to_string(), parent: parent.map(str::to_string), title: title.to_string() }),
        Some(row) if row.title != title => Some(Step::Retitle { id: id.to_string(), title: title.to_string() }),
        Some(_) => None,
    }
}
