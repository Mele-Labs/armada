//! Pulse's two kills: one process a Job holds, and every one. `#1647`.
//!
//! **Bridge chose these paths first**, so they do not spell their keys in the
//! last segment the way the rest of the table does: a control shipped ahead of
//! its route, and `packages/protocol/src/pending.ts` named them until this
//! served them.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Commands;
use crate::reference::Resolved;
use crate::served::Served;

/// The segment before `kill` on `kill_process`. **A string, read here**, so a
/// segment that is not a pid is the same 400 every undecodable request earns
/// rather than axum's own plain-text rejection.
#[derive(Deserialize)]
pub(crate) struct Process {
    pid: String,
}

/// One process of the Job, and what it started. What comes back is the Job:
/// unchanged where the process was a child of the Drone, which is not the Drone
/// dying; as `kill_drone` leaves it where the pid was the Drone's own.
pub(crate) async fn kill_process<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    Path(Process { pid }): Path<Process>,
) -> Response {
    let pid: u32 = match pid.parse() {
        Ok(pid) => pid,
        Err(why) => return undecodable(&format!("a pid: {why}"), served.run_id()),
    };
    match served.shared().kill_process(job.id(), pid).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every process of the Job. The Drone is among them, so the step stops as
/// `kill_drone` stops it; the Job survives.
pub(crate) async fn kill_processes<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.shared().kill_processes(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
