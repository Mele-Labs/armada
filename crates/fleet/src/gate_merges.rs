//! Which group's `merged` gate a merged pull request closes. `docs/concepts/session/the-plan.md`, *Gates*.
//!
//! **A merge closes only a group Fleet can tie it to from what it keeps**: a group handed to a Job of
//! its own is tied to that Job's pull requests, by the `group` its `started_to` row carries; the Job's
//! own pull requests are tied to its one group with a `merged` gate that is not handed on. Where the plan
//! has several, Fleet keeps no file list for a pull request to tell them apart, so a merge is tied by
//! when it came: to a group whose tasks are all done, merged after the first of them was completed.

use store::{AttachmentState, Holder, KeptAttachment, KeptPlan, KeptPlanGroup, Store};

use crate::gate_keeping::Seen;
use crate::session_host::linking::GROUP;
use crate::session_host::replacing::REPLACED_BY;
use crate::started_jobs::STARTED_TO;
use ipc::PlanGate;

/// A pull request of a Job started for a plan group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HandedPull {
    pub group: String,
    pub row: KeptAttachment,
}

/// The pull requests of every Job `held` names as started for a group, followed to the Job it was
/// handed on to for context, as `jobs_ended` follows it.
pub(crate) fn handed_pulls(store: &Store, held: &[KeptAttachment]) -> Vec<HandedPull> {
    let mut pulls = Vec::new();
    for link in held.iter().filter(|one| one.kind == STARTED_TO && one.state != AttachmentState::GivenBack) {
        let Some(group) = link.detail.get(GROUP) else { continue };
        let mut visited = vec![link.target.clone()];
        let mut id = link.target.clone();
        loop {
            let attached = store.attachments_of(&Holder::session(&id)).unwrap_or_default();
            pulls.extend(attached.iter().filter(|one| one.kind == "pr").map(|row| HandedPull { group: group.clone(), row: row.clone() }));
            match attached.iter().find(|one| one.kind == REPLACED_BY).map(|one| one.target.clone()) {
                Some(next) if !visited.contains(&next) => {
                    visited.push(next.clone());
                    id = next;
                }
                _ => break,
            }
        }
    }
    pulls
}

/// Each group's `merged` fact: the newest merge tied to it since it was recorded, once nothing tied to
/// it is still on the way.
pub(crate) fn merges_seen(plan: &KeptPlan, since: &str, held: &[KeptAttachment], handed: &[HandedPull]) -> Vec<Seen> {
    let merged_gate = |group: &&KeptPlanGroup| group.gate.as_deref() == Some(PlanGate::Merged.word());
    let mut own = plan.groups.iter().filter(merged_gate).filter(|group| !group.own_job);
    let only = match (own.next(), own.next()) {
        (Some(only), None) => Some(only.id.as_str()),
        _ => None,
    };
    let mut seen = Vec::new();
    for group in plan.groups.iter().filter(merged_gate) {
        let tied: Vec<&KeptAttachment> = if group.own_job {
            handed.iter().filter(|one| one.group == group.id).map(|one| &one.row).collect()
        } else {
            held.iter().filter(|one| one.kind == "pr").collect()
        };
        // A group that lands in several pull requests is not done at the first: another still open, or
        // a task of its own still open, is work on the way. A handed group's tasks are its Job's.
        let tasks_open = !group.own_job
            && group.tasks.iter().any(|task| matches!(crate::plan_record::status(task), Some("pending" | "in_progress")));
        if tasks_open || tied.iter().any(|one| open(one)) {
            continue;
        }
        let recorded = recorded_at(plan, &group.id).unwrap_or(since);
        // One of several groups of its own: only a merge after its work was first completed is its.
        let first_done = group.tasks.iter().filter_map(|task| task.completed_at.as_deref()).min();
        let floor = match (only == Some(group.id.as_str()) || group.own_job, first_done) {
            (true, _) => recorded,
            (false, Some(done)) => done.max(recorded),
            (false, None) => continue,
        };
        let newest = tied
            .into_iter()
            .filter(|one| crate::build_record::merged(one) && one.changed_at.as_str() >= floor)
            .max_by(|a, b| a.changed_at.cmp(&b.changed_at));
        if let Some(one) = newest {
            seen.push(Seen {
                gate: PlanGate::Merged,
                source: format!("pr:{}", one.target),
                group: Some(group.id.clone()),
                at: one.changed_at.clone(),
                fact: format!("Pull request #{} merged", one.target),
                commit: None,
            });
        }
    }
    seen
}

/// When the group was recorded, where this version added it. One kept from an earlier version reads
/// the plan's first recording, the most Fleet keeps of it.
fn recorded_at<'a>(plan: &'a KeptPlan, group: &str) -> Option<&'a str> {
    let added = plan.changes.as_ref()?.iter().any(|one| one.kind == "added" && one.id == group);
    added.then_some(plan.recorded_at.as_str())
}

/// A pull request not merged, closed or given back.
fn open(pull: &KeptAttachment) -> bool {
    pull.state != AttachmentState::GivenBack
        && !crate::build_record::merged(pull)
        && pull.detail.get("state").map(String::as_str) != Some("closed")
}
