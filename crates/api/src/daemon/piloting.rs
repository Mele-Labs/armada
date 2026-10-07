//! Taking a Job over and the three ways back. Since 23.50.
//! `docs/concepts/pilot.md`.
//!
//! **Its own surface**, for [`Needs`](super::Needs)' reason: whole on its own,
//! and a handler that takes only this cannot reach for a query it has no use
//! for. Every refusal is a 409 whose code says which, and
//! `docs/practices/protocol.md` lists them.

use std::future::Future;

use ipc::{HandoffBundle, JobId, JobSummary, PilotNote, TakeOver};

use crate::daemon::Refusal;

pub trait Piloting: Send + Sync + 'static {
    /// `take_over` — the Job's Drone ends where one is live and the worktree is
    /// the person's. Answers the Job, `piloted`.
    fn take_over(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        request: TakeOver,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `get_handoff` — what a person, or a Session on their behalf, is handed.
    /// 409 `fleet.not_piloted` on a Job never taken over.
    fn get_handoff(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<HandoffBundle, Refusal>> + Send;

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
