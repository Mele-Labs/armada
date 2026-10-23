//! Watches: a command Fleet runs on a cadence for one Job, and the agent's weighing of a failed
//! reading. `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
//!
//! **Its own surface**, for [`Requests`](super::Requests)' reason: whole on its own.

use std::future::Future;

use ipc::{SessionId, StartWatch, StopWatch, WatchList, WatchSummary, WeighWatch};

use crate::daemon::Refusal;

pub trait Watches: Send + Sync + 'static {
    /// `start_watch` — check it against the recipe and the limits, and keep it for the Job
    /// `caller` places, else the one `start.session_id` names.
    fn start_watch(
        &self,
        caller: Option<crate::Caller>,
        start: StartWatch,
    ) -> impl Future<Output = Result<WatchSummary, Refusal>> + Send;

    /// `weigh_watch` — the agent's verdict on a failed reading. A placed Job weighs its own.
    fn weigh_watch(
        &self,
        caller: Option<crate::Caller>,
        weigh: WeighWatch,
    ) -> impl Future<Output = Result<WatchSummary, Refusal>> + Send;

    /// `stop_watch` — end it `stopped`. A placed Job stops its own; Bridge stops any.
    fn stop_watch(
        &self,
        caller: Option<crate::Caller>,
        stop: StopWatch,
    ) -> impl Future<Output = Result<WatchSummary, Refusal>> + Send;

    /// `list_watches` — the placed Job's watches, live and ended.
    fn list_watches(
        &self,
        caller: Option<crate::Caller>,
        session_id: Option<SessionId>,
    ) -> impl Future<Output = Result<WatchList, Refusal>> + Send;
}
