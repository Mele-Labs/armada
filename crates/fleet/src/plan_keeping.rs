//! Where a Session's plan meets the store: the agent's task list and the edits Fleet saw are heard
//! into it, and what the record carries is read back, under the lock the caller already holds.
//! `docs/concepts/session.md`, *The plan*. The rules are `plan_record`'s.

use std::collections::BTreeMap;

use ipc::{FileTouch, Instant, PlanLandsInState, ReportedTask, TouchAct, TouchBy};
use store::{AttachmentState, Holder, KeptAttachment, KeptTouch, Store, WriteError};

use crate::file_touching::WINDOW;
use crate::plan_record::{commit_named, edit_seen, group_named, listed, PlanPull};
use crate::session_host::linking::GROUP;

/// The agent's task list, heard. Whether a task of the plan moved, so the row is worth publishing.
/// A Session with no plan moves no task here: its list is kept beside, as the build's pieces, and
/// Progress shows it until a plan is recorded.
pub(crate) fn tasks_heard(store: &mut Store, id: &str, list: &[ReportedTask], now: &str) -> Result<bool, WriteError> {
    let Some(mut plan) = store.session_plan(id)? else { return Ok(false) };
    let moved = listed(&mut plan, list, now);
    if moved {
        store.keep_session_plan_progress(id, &plan)?;
    }
    Ok(moved)
}

/// A file touched at `path`, from the repository's root: kept for the Field, and marks the group of the
/// plan its subagent named as worked, else the groups whose paths hold it. `by` is the subagent's type
/// and call, absent for the Session's own. Whether the row would show it: a touch that is new, or a plan
/// that moved.
pub(crate) fn touch_kept(
    store: &mut Store,
    id: &str,
    by: Option<(String, String)>,
    path: &str,
    act: &str,
    now: &str,
) -> Result<bool, WriteError> {
    let call = by.as_ref().map(|(_, call)| call.clone());
    let new = store.keep_session_touch(
        id,
        &KeptTouch { by, path: path.to_string(), act: act.to_string(), at: now.to_string() },
        WINDOW,
    )?;
    let Some(mut plan) = store.session_plan(id)? else { return Ok(new) };
    let group = match call {
        Some(call) => group_of_subagent(store, id, &call)?,
        None => None,
    };
    let seen = edit_seen(&mut plan, path, group.as_deref(), now);
    if seen.moved {
        store.keep_session_plan_progress(id, &plan)?;
    }
    Ok(new || seen.shown)
}

/// A `subagent` row's detail with the plan group its brief named made the plan's group id, or taken
/// off where the Session has no plan or no group of it is named so.
pub(crate) fn group_resolved(store: &Store, id: &str, mut detail: BTreeMap<String, String>) -> Result<BTreeMap<String, String>, WriteError> {
    let Some(named) = detail.remove(GROUP) else { return Ok(detail) };
    if let Some(group) = store.session_plan(id)?.and_then(|plan| group_named(&plan, &named)) {
        detail.insert(GROUP.to_string(), group);
    }
    Ok(detail)
}

/// The plan group the subagent started by `call` named, as its `subagent` row keeps it.
fn group_of_subagent(store: &Store, id: &str, call: &str) -> Result<Option<String>, WriteError> {
    let rows = store.attachments_of(&Holder::session(id))?;
    let row = rows.into_iter().find(|one| {
        one.kind == "subagent" && (one.target == call || one.detail.get("call").is_some_and(|started| started == call))
    });
    Ok(row.and_then(|mut one| one.detail.remove(GROUP)))
}

/// A commit heard, its short hash and first line: a task it names is completed on it. Whether the plan
/// moved. A Session with no plan, or a mod too old to send the line, moves nothing.
pub(crate) fn commit_heard(store: &mut Store, id: &str, commit: &str, subject: &str, now: &str) -> Result<bool, WriteError> {
    if subject.trim().is_empty() {
        return Ok(false);
    }
    let Some(mut plan) = store.session_plan(id)? else { return Ok(false) };
    let moved = commit_named(&mut plan, commit, subject, now);
    if moved {
        store.keep_session_plan_progress(id, &plan)?;
    }
    Ok(moved)
}

/// The pull requests a plan may land in, oldest first: the Session's own `pr` rows, each with the task in
/// progress when its first version was kept, then those of every Job a group was handed to, each with that
/// group. A row given back is none. Read beside the Session's ledger, under the lock the caller holds.
pub(crate) fn pulls_read(store: &Store, id: &str, held: &[KeptAttachment]) -> Result<Vec<PlanPull>, WriteError> {
    let own = pull_rows(held);
    let versions = if own.is_empty() { Vec::new() } else { store.attachment_versions(&Holder::session(id))? };
    let mut read: Vec<PlanPull> = own
        .into_iter()
        .filter_map(|row| {
            let from_task = versions.iter().find(|one| one.kind == "pr" && one.target == row.target).and_then(|one| one.from_task.clone());
            pull_of(row, from_task, None)
        })
        .collect();
    for link in held.iter().filter(|one| one.kind == "started_to") {
        let Some(group) = link.detail.get(GROUP) else { continue };
        let theirs = store.attachments_of(&Holder::session(&link.target))?;
        read.extend(pull_rows(&theirs).into_iter().filter_map(|row| pull_of(row, None, Some(group.clone()))));
    }
    Ok(read)
}

/// The `pr` rows not given back, oldest first.
fn pull_rows(rows: &[KeptAttachment]) -> Vec<&KeptAttachment> {
    let mut rows: Vec<&KeptAttachment> =
        rows.iter().filter(|one| one.kind == "pr" && one.state != AttachmentState::GivenBack).collect();
    rows.sort_by(|a, b| a.since.cmp(&b.since));
    rows
}

/// A `pr` row as a plan reads it: its number, and the title and standing the forge last gave it. A row
/// the forge has not described is open, or merged where it is spent.
fn pull_of(row: &KeptAttachment, from_task: Option<String>, handed: Option<String>) -> Option<PlanPull> {
    let state = match row.detail.get("state").map(String::as_str) {
        Some("open") => PlanLandsInState::Open,
        Some("draft") => PlanLandsInState::Draft,
        Some("merged") => PlanLandsInState::Merged,
        Some("closed") => PlanLandsInState::Closed,
        _ if row.state == AttachmentState::Spent => PlanLandsInState::Merged,
        _ => PlanLandsInState::Open,
    };
    Some(PlanPull {
        number: row.target.parse().ok()?,
        title: row.detail.get("title").cloned().unwrap_or_default(),
        state,
        from_task,
        handed,
    })
}

/// The touches as the record carries them, oldest first.
pub(crate) fn touches_wire(kept: Vec<KeptTouch>) -> Vec<FileTouch> {
    kept.into_iter()
        .map(|one| FileTouch {
            by: one.by.map(|(agent, id)| TouchBy { agent, id }),
            path: one.path,
            act: match one.act.as_str() {
                "write" => TouchAct::Write,
                "remove" => TouchAct::Remove,
                _ => TouchAct::Edit,
            },
            at: Instant::carried(one.at),
        })
        .collect()
}
