//! A Fleet that starts takes its Trigger repairs up again. Nothing a repair
//! was doing is lost with the process: the firing's row says where it stood.
//! `docs/concepts/fleet.md`, *A failed Trigger's repair*.
//!
//! **Nothing is left `repairing` forever.** A firing found `repairing` or
//! `rerunning` is queued again and its interrupted attempt is redone. One
//! found `rerunning` after the owner chose goes back to `fix_ready`, with the
//! choice kept, and is placed again. A slot still held by a repair that no
//! firing is working is given back.

use std::collections::BTreeSet;

use adapter_traits::{AgentHarness, Delivery, SlotHeld, Vcs, WorkProduct};
use core_model::{RepairRecord, TriggerState};

use crate::daemon::Fleet;
use crate::repairing::holder_of;
use crate::trigger_repair::Waiting;

/// The Job a repair's slot holder names, where it is one. `holder_of` writes
/// `<job id>-repair-<firing>`.
pub(crate) fn job_of_holder(holder: &str) -> Option<&str> {
    let holder = holder.strip_suffix("-onto").unwrap_or(holder);
    let (job, firing) = holder.rsplit_once("-repair-")?;
    firing.parse::<i64>().ok().map(|_| job)
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
    /// Queue every unfinished repair again, and give back the slots of repairs
    /// nothing is working. **Once, at start**, before the loop's first turn.
    pub async fn repairs_recovered(&self) {
        let unfinished = self
            .store()
            .lock()
            .await
            .unfinished_repairs()
            .unwrap_or_default();
        let mut working = BTreeSet::new();
        for (job_id, firing_id, firing) in unfinished {
            let Ok(job) = self.load(&job_id).await else {
                continue;
            };
            if firing.repair.choice.is_some() {
                // Chosen, and the rerun on the Job's branch was cut short.
                let kept = RepairRecord {
                    settled_at: Some(self.now()),
                    ..firing.repair.clone()
                };
                let _ = self.store().lock().await.settle_repair(
                    firing_id,
                    TriggerState::FixReady,
                    &kept,
                    None,
                );
                continue;
            }
            let Some(command) = self.command_of(&job, &firing).await else {
                continue;
            };
            working.insert(holder_of(&job_id, firing_id));
            self.trigger_repairs()
                .lock()
                .expect("not poisoned")
                .push(Waiting {
                    job: job_id,
                    firing: firing_id,
                    trigger: firing.name,
                    step: firing.step,
                    command,
                    exit: firing.exit_code,
                    stdout: String::new(),
                    stderr: String::new(),
                    record: firing.repair,
                });
        }
        self.repair_slots_swept(&working);
    }

    /// Give back a slot held by a repair no firing is working: its Job was
    /// forgotten, or the repair ended and the process died before it let go.
    /// The work stays on the repair branch. A held slot is never taken back by
    /// anything else, since a Job's holder lives until it is released.
    fn repair_slots_swept(&self, working: &BTreeSet<String>) {
        for served in self.repositories().served() {
            let pool = crate::leasing::pool_of(&served);
            for reading in self.vcs().slot_pool(&pool) {
                let SlotHeld::Job(holder) = &reading.held else {
                    continue;
                };
                if job_of_holder(holder).is_some() && !working.contains(holder) {
                    let _ = self.vcs().park_slot(&pool, reading.slot, holder);
                }
            }
        }
    }
}
