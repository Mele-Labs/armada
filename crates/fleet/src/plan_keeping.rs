//! Where a Session's plan meets the store: the agent's task list and the edits Fleet saw are heard
//! into it, and what the record carries is read back, under the lock the caller already holds.
//! `docs/concepts/session.md`, *The plan*. The rules are `plan_record`'s.

use ipc::{FileTouch, Instant, ReportedTask, TouchAct, TouchBy};
use store::{Holder, KeptTouch, Store, WriteError};

use crate::file_touching::WINDOW;
use crate::plan_record::{commit_named, edit_seen, listed, merge_marked, untold};

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
    if seen.moved {
        store.keep_session_plan_progress(id, &plan)?;
    }
    Ok(new || seen.shown)
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

/// Pull request `number` of the Session merged: the tasks of the group whose task was in progress when
/// it was first kept are completed on it, and told now. What the merge's message adds for them, empty
/// where it moved none.
pub(crate) fn merge_heard(store: &mut Store, id: &str, number: u64, now: &str) -> Result<String, WriteError> {
    let target = number.to_string();
    let from = store
        .attachment_versions(&Holder::session(id))?
        .into_iter()
        .find(|one| one.kind == "pr" && one.target == target)
        .and_then(|one| one.from_task);
    let (Some(task), Some(mut plan)) = (from, store.session_plan(id)?) else { return Ok(String::new()) };
    let moved = merge_marked(&mut plan, &task, number, now);
    if moved.is_empty() {
        return Ok(String::new());
    }
    for group in &mut plan.groups {
        for task in group.tasks.iter_mut().filter(|task| moved.contains(&task.text)) {
            task.told = true;
        }
    }
    store.keep_session_plan_progress(id, &plan)?;
    let named: Vec<String> = moved.iter().map(|text| format!("\"{text}\"")).collect();
    Ok(format!("\nFleet marked these tasks of your plan completed, as #{number} merged: {}.", named.join(", ")))
}

/// `calls` more of the agent's own tool calls. Whether the Session has a plan, which is all that counts them.
pub(crate) fn calls_counted(store: &mut Store, id: &str, calls: u32) -> Result<bool, WriteError> {
    Ok(store.count_session_calls(id, calls)?.is_some())
}

/// What the agent is told of the tasks Fleet marked or saw go stale since it was last told, kept as told.
/// Absent where there is none.
pub(crate) fn marks_untold(store: &mut Store, id: &str) -> Result<Option<String>, WriteError> {
    let Some(mut plan) = store.session_plan(id)? else { return Ok(None) };
    let touches = store.session_touches(id)?;
    let said = untold(&mut plan, &touches);
    if said.is_some() {
        store.keep_session_plan_progress(id, &plan)?;
    }
    Ok(said)
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
