//! A result told the Drone as it lands, while the rest of its run goes on.
//! #1062.
//!
//! **Never the last turn.** A result that ends the run — the last to finish,
//! or the failure that stops the rest — is in the report
//! [`Fleet::dry_run_ends`] sends, so no result is told twice.

use std::future::Future;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::JobId;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{ChecksReported, HEADING};
use crate::checking::Completed;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};
use crate::underway::{Heard, Landed};

impl ChecksReported {
    /// One result, landed while others still run.
    pub(super) fn landed(landed: &Landed) -> ChecksReported {
        let came_to = landed
            .ran
            .as_ref()
            .map_or("finished", |run| run.outcome.as_wire())
            .replace('_', " ");
        let still = landed
            .still
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ");
        ChecksReported(format!(
            "{HEADING}\n\n`{}` {came_to} in {:.1}s. Still going: {still}. Each result \
             arrives as its own turn, and the last one says the run is over.",
            landed.name,
            landed.took.as_secs_f64(),
        ))
    }
}

impl ChecksReported {
    /// The run is waiting for a place; said once.
    pub(super) fn queued(others: usize, of: usize) -> ChecksReported {
        ChecksReported(format!(
            "{HEADING}\n\nWaiting for a Check slot, {others} of {of} in use. Your run \
             starts when one is free, and each result arrives as its own turn."
        ))
    }
}

impl ChecksReported {
    /// The run outlived its time-box and was stopped.
    pub(super) fn timed_out(whole: std::time::Duration, running: &[String]) -> ChecksReported {
        let minutes = whole.as_secs().div_ceil(60);
        let unit = if minutes == 1 { "minute" } else { "minutes" };
        let where_it_stood = match running.is_empty() {
            true => "It was still waiting for a Check slot.".to_string(),
            false => format!("It was running {}.", running.join(", ")),
        };
        ChecksReported(format!(
            "{HEADING}\n\nThe checks did not finish in {minutes} {unit}, so they were \
             stopped. {where_it_stood} Ask again, or submit when the work is done."
        ))
    }
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
    /// Run `running`, telling the Drone each result that lands before it ends.
    pub(super) async fn heard_while(
        &self,
        caller: &JobId,
        run: u64,
        running: impl Future<Output = Vec<Completed>>,
        mut hearing: UnboundedReceiver<Heard>,
    ) -> Vec<Completed> {
        tokio::pin!(running);
        let completed = loop {
            tokio::select! {
                biased;
                Some(heard) = hearing.recv() => self.told_heard(caller, run, heard).await,
                completed = &mut running => break completed,
            }
        };
        // A result heard just before the run ended still goes before the report.
        while let Ok(heard) = hearing.try_recv() {
            self.told_heard(caller, run, heard).await;
        }
        completed
    }

    /// Tell the Drone one result, only while its run is still the one in flight.
    async fn told_heard(&self, caller: &JobId, run: u64, heard: Heard) {
        let Some(slot) = self.slot_of(caller).await else {
            return;
        };
        let working = slot.lock().await;
        let Some(at_work) = working
            .as_ref()
            .filter(|at_work| at_work.checks_in_flight(run))
        else {
            return;
        };
        let told = match &heard {
            Heard::Landed(landed) => ChecksReported::landed(landed),
            Heard::Queued(others) => ChecksReported::queued(*others, self.checks_at_once().get()),
        };
        // Written down before the send, `Fleet::tell`'s order.
        at_work.instructed(Occasion::Checks, told.text());
        let _ = at_work.session().checks(&told).await;
    }
}
