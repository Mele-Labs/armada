//! How the library is judged: the code searches a Job ran before its first edit, a week before the library and a week
//! after. `docs/concepts/feature-catalog.md`, *How we know it worked*.
//!
//! **Counted from the transcripts Fleet already reads**, a turn at a time (`turn_ingest`), so no Job reports it: Grep and
//! Glob calls and shell `grep`, `rg`, `find` and `git grep`, in the Job's turns and its subagents', up to the first Edit
//! or Write.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FirstEditSearches, SearchWeek};

use crate::daemon::Fleet;

/// How many weeks are read where none is asked for.
const WEEKS: u32 = 8;
/// The most weeks one read goes back.
const MOST_WEEKS: u32 = 52;

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
    /// `get_first_edit_searches`.
    pub(super) async fn first_edit_searches_read(&self, weeks: Option<u32>) -> Result<FirstEditSearches, Refusal> {
        let weeks = weeks.unwrap_or(WEEKS).clamp(1, MOST_WEEKS);
        let now = self.now().as_str().to_string();
        let edits = self.store().lock().await.first_edits(&now, weeks).map_err(|why| self.ledger_fault(why))?;
        let mut by_week: BTreeMap<String, (u32, u32)> = BTreeMap::new();
        let mut unmeasured = 0;
        for edit in edits {
            if !edit.measured {
                unmeasured += 1;
                continue;
            }
            let week = by_week.entry(edit.week).or_default();
            week.0 += 1;
            week.1 += edit.searches;
        }
        let weeks = by_week
            .into_iter()
            .map(|(starts, (jobs, searches))| SearchWeek {
                starts,
                jobs,
                searches,
                per_job: (f64::from(searches) / f64::from(jobs.max(1)) * 10.0).round() / 10.0,
            })
            .collect();
        Ok(FirstEditSearches { weeks, unmeasured })
    }
}
