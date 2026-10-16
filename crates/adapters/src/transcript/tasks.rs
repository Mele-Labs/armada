//! The harness's account of a task it started for a call: a shell command run beside the turn, or
//! a watch. Recorded 16 Oct 2026 on claude 2.1.300; `tests/background_task.rs` holds the lines.
//!
//! **Three lines say it and each is read for what only it has.** `task_started` has the task, the
//! call that asked for it and whether it runs beside the turn. `task_updated` has the status the
//! moment it changes. `task_notification` follows it with where the output went and, for a
//! command, the exit code.
//!
//! **The exit code is the one fact read out of prose.** No key carries it: the notice says
//! `completed (exit code 0)` or `failed with exit code 3` in its `summary`, which also quotes the
//! agent's own description. So only the end of the summary is read, only whole, and a summary that
//! does not end that way has no exit — a miss is an absent figure and never a wrong one.

use adapter_traits::{DroneEvent, TaskStatus};

use super::SystemLine;

/// A background task of a call, by the tool that made it: the form the ledger keeps it under.
/// `None` for every other tool, a subagent's included — those are other rows.
pub fn background_form(tool: &str) -> Option<&'static str> {
    match tool {
        "Bash" => Some("command"),
        "Monitor" => Some("monitor"),
        _ => None,
    }
}

pub(super) fn started(system: SystemLine) -> DroneEvent {
    match (system.task_id, system.tool_use_id) {
        (Some(task), Some(call)) => DroneEvent::TaskStarted {
            task,
            call,
            description: system.description.unwrap_or_default(),
            backgrounded: system.is_backgrounded.unwrap_or(false),
        },
        _ => unrecognised("task_started"),
    }
}

/// The status change. A patch that moves anything but a final status is not an end.
pub(super) fn updated(system: SystemLine) -> DroneEvent {
    let status = system.patch.and_then(|patch| patch.status).as_deref().and_then(status_of);
    match (system.task_id, status) {
        (Some(task), Some(status)) => DroneEvent::TaskEnded { task, status, output: None, exit: None },
        _ => unrecognised("task_updated"),
    }
}

pub(super) fn notified(system: SystemLine) -> DroneEvent {
    let status = system.status.as_deref().and_then(status_of);
    match (system.task_id, status) {
        (Some(task), Some(status)) => DroneEvent::TaskEnded {
            task,
            status,
            output: system.output_file.filter(|file| !file.is_empty()),
            exit: system.summary.as_deref().and_then(exit_of),
        },
        _ => unrecognised("task_notification"),
    }
}

fn unrecognised(subtype: &str) -> DroneEvent {
    DroneEvent::Unrecognised { kind: format!("system/{subtype}") }
}

/// A task stopped on purpose is `killed` in one line and `stopped` in the other.
fn status_of(word: &str) -> Option<TaskStatus> {
    match word {
        "completed" => Some(TaskStatus::Completed),
        "failed" => Some(TaskStatus::Failed),
        "killed" | "stopped" => Some(TaskStatus::Killed),
        _ => None,
    }
}

/// The two endings the harness writes: `… completed (exit code 0)` and `… " failed with exit code 3`.
fn exit_of(summary: &str) -> Option<i32> {
    let code = match summary.strip_suffix(')') {
        Some(rest) => rest.rsplit_once(" (exit code ")?.1,
        None => summary.rsplit_once("\" failed with exit code ")?.1,
    };
    code.parse().ok()
}
