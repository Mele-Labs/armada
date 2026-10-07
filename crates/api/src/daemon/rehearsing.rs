//! Running one Check or Command by hand, in a Job's worktree or the main
//! checkout, as a rehearsal.
//!
//! **Its own surface, and not a method of [`Commands`](super::Commands)**, which
//! was past the line the gate refuses a file over. The reason that stands without
//! the line count is [`Retros`](super::Retros)': these nine are whole on their
//! own, one family with one lifecycle (start, stop, undo), and a handler that
//! takes only this cannot reach for anything else a command does.

use std::future::Future;

use ipc::{
    CheckoutRunRecord, CheckoutRunUnderway, JobId, NamedRun, RunRecord, RunUnderway,
    StartCheckoutRun, StartRun,
};

use crate::daemon::Refusal;

pub trait Rehearsing: Send + Sync + 'static {
    /// `start_run` — run one Check or Command in this Job's worktree, as a
    /// rehearsal: no Evidence, no Check row, nothing on the Job moves.
    ///
    /// **By `Arc`, for [`Commands::show_again`](super::Commands::show_again)'s reason**: the run is a task of
    /// its own and outlives the request. It answers once the run is underway;
    /// the output and the end arrive as events.
    ///
    /// [`Refusal::IllegalMove`] where the worktree is gone or a run is already
    /// out on this Job; [`Refusal::Unacceptable`] where nothing declares the
    /// name, or a narrowed run has nothing to narrow to.
    fn start_run(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        run: StartRun,
    ) -> impl Future<Output = Result<RunUnderway, Refusal>> + Send;

    /// `stop_run` — end the run's process group, and answer with its record
    /// once it is written. [`Refusal::IllegalMove`] on a run not in flight.
    fn stop_run(
        &self,
        job_id: JobId,
        run: NamedRun,
    ) -> impl Future<Output = Result<RunRecord, Refusal>> + Send;

    /// `undo_run` — put back what one run changed, from the snapshot taken
    /// before it. **Never a discard**: a Drone's work is uncommitted until
    /// delivery. [`Refusal::IllegalMove`] while a Drone works in the tree, while
    /// a run is in flight, or where a path the run changed has moved since.
    fn undo_run(
        &self,
        job_id: JobId,
        run: NamedRun,
    ) -> impl Future<Output = Result<RunRecord, Refusal>> + Send;

    /// `start_checkout_run` — run one Check or Command in the main checkout,
    /// as a rehearsal: no Evidence, no Check row, and no Job to move.
    ///
    /// **By `Arc`, for [`Rehearsing::start_run`]'s reason.** It runs in the
    /// working tree as it is on disk — there is no throwaway copy and no Where
    /// control, Journey 9, *Running one*.
    ///
    /// **A checkout run and a Job's run do not lock each other out.** One run
    /// at a time is per owner: two runs in one tree fight over one build
    /// directory, and these are two trees.
    ///
    /// [`Refusal::IllegalMove`] where a run is already out in the checkout;
    /// [`Refusal::Unacceptable`] where nothing declares the name.
    fn start_checkout_run(
        self: std::sync::Arc<Self>,
        run: StartCheckoutRun,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<CheckoutRunUnderway, Refusal>> + Send;

    /// `stop_checkout_run` — end the run's process group, and answer with its
    /// record once it is written. [`Refusal::IllegalMove`] on a run not in
    /// flight.
    fn stop_checkout_run(
        &self,
        run: NamedRun,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<CheckoutRunRecord, Refusal>> + Send;

    /// `undo_checkout_run` — put back what one run changed, from the snapshot
    /// taken before it.
    ///
    /// **Never offered where no snapshot was taken.** This tree holds a
    /// person's own uncommitted work, which no Job's worktree does, so a run
    /// with nothing kept behind it is refused rather than discarded from.
    ///
    /// [`Refusal::IllegalMove`] while a run is in flight, on a run already
    /// undone or with no snapshot, and where a path the run changed has moved
    /// since.
    fn undo_checkout_run(
        &self,
        run: NamedRun,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<CheckoutRunRecord, Refusal>> + Send;

    /// `start_checkout_verify` — run setup and every Check once in the main
    /// checkout, one after another, each an ordinary checkout run: Journey 9,
    /// *Verify*. Answers once the first step is out.
    ///
    /// **By `Arc`, for [`Rehearsing::start_run`]'s reason**: the steps outlive
    /// the call.
    ///
    /// `asked.workspace` names a directory below the root whose own
    /// `armada.yml` runs, in that directory, instead of the root's.
    ///
    /// [`Refusal::IllegalMove`] where a run or a Verify is already out in the
    /// checkout; [`Refusal::Unacceptable`] where there is nothing to run, or
    /// the workspace leaves the repository or its file will not load.
    fn start_checkout_verify(
        self: std::sync::Arc<Self>,
        asked: ipc::StartCheckoutVerify,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<ipc::CheckoutVerify, Refusal>> + Send;
}
