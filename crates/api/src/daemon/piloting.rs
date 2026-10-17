//! The three ways out of a pilot. Since 23.50.
//! `docs/concepts/pilot.md`.
//!
//! **Its own surface**, for [`Needs`](super::Needs)' reason: whole on its own,
//! and a handler that takes only this cannot reach for a query it has no use
//! for. Every refusal is a 409 whose code says which, and
//! `docs/practices/protocol.md` lists them.

use std::future::Future;

use ipc::{JobId, JobSummary, PilotNote};

use crate::daemon::Refusal;

pub trait Piloting: Send + Sync + 'static {
    /// `submit_for_verification` — the step's gates run on the worktree. The Job
    /// answers `running` with no Drone, or `piloted` still where they failed.
    /// Held for the whole run, like `rerun_checks`.
    fn submit_for_verification(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `attest_complete` — `completed_success`, recorded as attested and never
    /// as verified. Refused while a step has not advanced.
    fn attest_complete(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        note: PilotNote,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `close_as_superseded` — the terminal state meaning superseded by human
    /// work. A Job waiting on it is released with a warning.
    fn close_as_superseded(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        note: PilotNote,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;
}
