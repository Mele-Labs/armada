//! Where a Session's plan meets the store: the agent's task list and the edits Fleet saw are heard
//! into it, and what the record carries is read back, under the lock the caller already holds.
//! `docs/concepts/session.md`, *The plan*. The rules are `plan_record`'s.

use ipc::{FileTouch, Instant, ReportedTask, TouchAct, TouchBy};
use store::{KeptTouch, Store, WriteError};

use crate::file_touching::WINDOW;
use crate::plan_record::{edit_seen, listed};

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

/// A file touched at `path`, from the repository's root: kept for the Field, and marks the groups of
/// the plan whose paths hold it as worked. `by` is the subagent's type and call, absent for the
/// Session's own. Whether the row would show it: a touch that is new, or a plan that moved.
pub(crate) fn touch_kept(
    store: &mut Store,
    id: &str,
    by: Option<(String, String)>,
    path: &str,
    act: &str,
    now: &str,
) -> Result<bool, WriteError> {
    let new = store.keep_session_touch(
        id,
        &KeptTouch { by, path: path.to_string(), act: act.to_string(), at: now.to_string() },
        WINDOW,
    )?;
    let Some(mut plan) = store.session_plan(id)? else { return Ok(new) };
    let seen = edit_seen(&mut plan, path, now);
    if !seen.groups.is_empty() {
        store.keep_session_plan_edit(id, &seen.groups, &seen.tasks, now)?;
    }
    Ok(new || seen.shown)
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
