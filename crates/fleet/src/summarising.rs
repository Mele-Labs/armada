//! What a Job looks like as a Board row.
//!
//! **Split from the trait impls, not from a line count.** `serving` and
//! `commanding` are the operations — one method per row of the inventory, each
//! answering what a caller asked. This is what several of them need, and it is
//! not an operation: `summarised` is called on every Job in a list.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Job;

use ipc::JobSummary;

use crate::daemon::Fleet;

/// The bays a Job has held, for its row. **A history the store cannot read leaves the row without
/// it**, as the marks beside it do. Read on every path that builds a row a client replaces its own
/// with, so an event's row does not drop what a refetch carries.
pub(crate) fn bays_on_row(store: &store::Store, job: &Job) -> Vec<ipc::JobBay> {
    store
        .bays_of(job.id())
        .unwrap_or_default()
        .into_iter()
        .map(|one| ipc::JobBay {
            slot: one.slot,
            since: ipc::Instant::carried(one.since),
            until: one.until.map(ipc::Instant::carried),
        })
        .collect()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// A Job as a Board row, with the reason its last transition stored.
    pub(crate) async fn summarised(&self, job: &Job) -> Result<JobSummary, Refusal> {
        let reason = self
            .last_reason(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let started_at = self
            .job_started_at(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        // **What Overview 28 (#1092) needed and the row never had**: a Job
        // killed, failed or rejected has nowhere to stand once `sectionOf`
        // sends it to Done, collapsed, and Overview drops Done outright — so
        // this is the fact the Board's own Recently ended reads instead.
        let ended_at = self
            .job_ended_at(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let mut summary = JobSummary::of(
            job,
            reason.as_ref(),
            None,
            None,
            false,
            None,
            started_at,
            ended_at,
        );
        {
            let store = self.store().lock().await;
            summary.bays = bays_on_row(&store, job);
        }
        Ok(summary)
    }
}
