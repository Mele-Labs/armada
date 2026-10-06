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

use super::{ChecksReported, Plan, HEADING};
use crate::checking::Completed;
use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};
use crate::underway::{Heard, Landed};

impl ChecksReported {
    /// The task running the Checks died before it could report.
    pub(super) fn lost(panicked: bool) -> ChecksReported {
        let how = match panicked {
            true => "Fleet's task for them failed",
            false => "Fleet's task for them was cancelled",
        };
        ChecksReported(format!(
            "{HEADING}\n\nThe checks stopped before they finished: {how}. This is a fault in \
             Fleet and not in your work. Ask again, or submit when the work is done."
        ))
    }

    /// Where a run that is still going stands, told on a timer so the Drone
    /// never has to ask: what runs and for how long, what waits, what is done.
    pub(super) fn standing(
        rows: &[ipc::CheckUnderway],
        now: &core_model::Timestamp,
    ) -> ChecksReported {
        let named = |names: Vec<String>| names.join(", ");
        let running = rows
            .iter()
            .filter(|row| row.ran.is_none())
            .filter_map(|row| {
                let began = row.started_at.as_ref()?.to_domain();
                let secs = elapsed(&began, now).as_secs();
                Some(format!("`{}` ({}m {:02}s)", row.name, secs / 60, secs % 60))
            })
            .collect::<Vec<_>>();
        let waiting = rows
            .iter()
            .filter(|row| row.started_at.is_none() && row.ran.is_none())
            .map(|row| format!("`{}`", row.name))
            .collect::<Vec<_>>();
        let done = rows
            .iter()
            .filter(|row| row.ran.is_some())
            .map(|row| format!("`{}`", row.name))
            .collect::<Vec<_>>();
        let mut lines = vec![format!("{HEADING}\n\nThe run is still going.")];
        if !running.is_empty() {
            lines.push(format!("Running: {}.", named(running)));
        }
        if !waiting.is_empty() {
            lines.push(format!(
                "Waiting for a Check slot or a Command: {}.",
                named(waiting)
            ));
        }
        if !done.is_empty() {
            lines.push(format!("Done: {}.", named(done)));
        }
        lines.push(
            "Each result arrives as its own turn, and the last one says the run is over.".into(),
        );
        ChecksReported(lines.join(" ").replacen(". Running", ".\n\nRunning", 1))
    }

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
        plan: &Plan,
        running: impl Future<Output = Vec<Completed>>,
        mut hearing: UnboundedReceiver<Heard>,
    ) -> Vec<Completed> {
        tokio::pin!(running);
        let every = self.budget().status_every();
        let mut ticking = tokio::time::interval_at(tokio::time::Instant::now() + every, every);
        ticking.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let completed = loop {
            tokio::select! {
                biased;
                Some(heard) = hearing.recv() => self.told_heard(caller, run, heard).await,
                _ = ticking.tick() => self.told_standing(caller, run, plan).await,
                completed = &mut running => break completed,
            }
        };
        // A result heard just before the run ended still goes before the report.
        while let Ok(heard) = hearing.try_recv() {
            self.told_heard(caller, run, heard).await;
        }
        completed
    }

    /// Tell the Drone where its run stands, only while it is still the one in flight.
    async fn told_standing(&self, caller: &JobId, run: u64, plan: &Plan) {
        let Some(underway) = self.underway().dry_run_on(
            &ipc::JobId::from(plan.record.id()),
            &ipc::StepId::from(&plan.step),
        ) else {
            return;
        };
        let told = ChecksReported::standing(&underway.checks, &self.now());
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
        at_work.instructed(Occasion::Checks, told.text());
        let _ = at_work.session().checks(&told).await;
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
