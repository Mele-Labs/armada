//! What the Usage page reads of where time went: the hours, the builds and the Jobs. Each is one
//! summed read of the store, so the lock is held for tens of milliseconds and never across anything
//! else. The sums are `store`'s, `time_summary.rs`; the wire maps these onto its own types.
//! `docs/concepts/session.md`, *Where a Job's time went*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use store::{BuildFigures, HoursRead, KeptJobTime, TimeRead, Usual, DEFAULT_HOURS, DEFAULT_SLOWEST};

use crate::daemon::Fleet;

impl Fleet
{
    /// `hours` hours (48 where not named) from `since`, which is an hour's start, or ending with this hour.
    pub async fn usage_hours_read(&self, since: Option<String>, hours: Option<u32>) -> Result<HoursRead, Refusal> {
        let now = self.now().as_str().to_string();
        let read = self.store().lock().await.usage_hours(since.as_deref(), hours.unwrap_or(DEFAULT_HOURS), &now);
        read.map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.ledger_unnamed("a time to begin the hours at"))
    }

    /// The builds that ran since `since` (48 hours back where not named), oldest first.
    pub async fn usage_builds_read(&self, since: Option<String>) -> Result<Vec<BuildFigures>, Refusal> {
        let now = self.now().as_str().to_string();
        let read = self.store().lock().await.usage_builds(since.as_deref(), &now);
        read.map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.ledger_unnamed("a time to look for builds from"))
    }

    /// The Jobs that ended in the window: their time together, the slowest, and each workflow's usual.
    pub async fn usage_time_read(
        &self,
        since: Option<String>,
        until: Option<String>,
        slowest: Option<usize>,
    ) -> Result<TimeRead, Refusal> {
        let now = self.now().as_str().to_string();
        let read = self.store().lock().await.usage_time(since.as_deref(), until.as_deref(), &now, slowest.unwrap_or(DEFAULT_SLOWEST));
        read.map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.ledger_unnamed("a time for each bound of the window"))
    }

    /// A Job's kept split and, where its workflow has a name, the usual of every workflow, read in one
    /// short hold. `None` where no split is kept for it. A Job Fleet does not know is refused.
    pub async fn job_split_read(&self, id: &str) -> Result<Option<(KeptJobTime, Vec<(String, Usual)>)>, Refusal> {
        let store = self.store().lock().await;
        if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Err(self.ledger_unnamed("a job Fleet knows"));
        }
        let Some(kept) = store.job_time(id).map_err(|why| self.ledger_fault(why))? else {
            return Ok(None);
        };
        let usual = match kept.workflow.is_empty() {
            true => Vec::new(),
            false => store.usual_splits().map_err(|why| self.ledger_fault(why))?,
        };
        Ok(Some((kept, usual)))
    }
}
