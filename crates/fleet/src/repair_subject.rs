//! What a repair does to the thing it repairs, which is a firing of a Trigger
//! or a step added to one Job. `docs/concepts/trigger.md`.
//!
//! **One repair and two places it is kept.** The queue, the Drone, the tries
//! and the choice are `crate::repairing` and `crate::placing_a_fix`, and they
//! ask this file where a row lives and how it moves. A branch the repair is
//! done with is also given back here.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, WorktreeSpec};
use core_model::{Fired, Job, JobId, Level, RepairRecord, TriggerState};

use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::trigger_repair::Subject;

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
    /// The spec a repair's branch is cut from, and the only way to name it for
    /// deleting.
    pub(crate) fn repair_spec(
        &self,
        served: &Served,
        job: &Job,
        subject: &Subject,
    ) -> Result<WorktreeSpec, adapter_traits::WorktreeSpecRefused> {
        WorktreeSpec::for_job(served.root(), &format!("repair-{}-{subject}", job.handle()))
    }

    /// Write where a repair has got to: the state and the record, and an end
    /// where the repair is over.
    pub(crate) async fn repair_settled(
        &self,
        job: &Job,
        subject: &Subject,
        state: TriggerState,
        record: &RepairRecord,
        ended: bool,
    ) -> Result<(), String> {
        match subject {
            Subject::Firing(id) => {
                let at = ended.then(|| self.now());
                self.store()
                    .lock()
                    .await
                    .settle_repair(*id, state, record, at.as_ref())
                    .map_err(|why| why.to_string())
            }
            Subject::Addition(id) => {
                let mut store = self.store().lock().await;
                let held = store
                    .job_additions(job.id())
                    .map_err(|why| why.to_string())?;
                let added = held
                    .iter()
                    .find(|one| &one.id == id)
                    .ok_or_else(|| format!("the Job holds no added step `{id}`"))?;
                let before = added
                    .fired
                    .clone()
                    .unwrap_or_else(|| Fired::running(self.now()));
                let over = ended || matches!(state, TriggerState::Passed);
                let fired = Fired {
                    state,
                    ended_at: over.then(|| self.now()),
                    ..before
                };
                store
                    .settle_addition_repair(job.id(), id, &fired, record)
                    .map_err(|why| why.to_string())
            }
        }
    }

    /// `job.trigger_changed` or `job.addition_changed` for a row a repair has
    /// just written, read back so the event carries what the row holds.
    pub(crate) async fn repair_moved(&self, job: &Job, subject: &Subject) {
        match subject {
            Subject::Firing(firing_id) => {
                let held = self.store().lock().await.firings_with_ids(job.id());
                if let Some((_, firing)) = held
                    .ok()
                    .and_then(|all| all.into_iter().find(|(id, _)| id == firing_id))
                {
                    self.trigger_moved(job, &firing);
                }
            }
            Subject::Addition(id) => {
                let held = self.store().lock().await.job_additions(job.id());
                if let Some(added) = held
                    .ok()
                    .and_then(|all| all.into_iter().find(|one| &one.id == id))
                {
                    self.addition_moved(job, &added, false);
                }
            }
        }
    }

    /// Where a repair that did not fix it leaves a subject that blocks: still
    /// holding the Job, so `Held` and not `Failed`.
    pub(crate) async fn held_where_it_blocks(
        &self,
        job: &Job,
        subject: &Subject,
        state: TriggerState,
    ) -> TriggerState {
        match subject {
            Subject::Firing(firing_id) => {
                let held = self.store().lock().await.firings_with_ids(job.id());
                held.ok()
                    .and_then(|all| all.into_iter().find(|(id, _)| id == firing_id))
                    .map_or(state, |(_, firing)| firing.settled_as(state))
            }
            Subject::Addition(id) => {
                let held = self.store().lock().await.job_additions(job.id());
                let holds = held
                    .ok()
                    .and_then(|all| all.into_iter().find(|one| &one.id == id))
                    .is_some_and(|added| {
                        added.on_failure.block
                            && core_model::can_hold(job.workflow(), added.when, &added.step)
                    });
                match state {
                    TriggerState::Failed if holds => TriggerState::Held,
                    other => other,
                }
            }
        }
    }

    /// A repair that passed lets the hold go: the next entry to a
    /// `step_starts` moment passes it by.
    pub(crate) async fn hold_released_by_repair(&self, job_id: &JobId, subject: &Subject) {
        let mut store = self.store().lock().await;
        let _ = match subject {
            Subject::Firing(id) => store.mark_hold_released(*id),
            Subject::Addition(id) => store.mark_addition_hold_released(job_id, id),
        };
    }

    /// Delete the branch a repair Drone wrote on, now its fix is merged onto
    /// the Job's or the repair failed. **Not a `new_pr` one**: that branch is
    /// the pull request's head. The slot was given back before this, so the
    /// branch is checked out nowhere. Idempotent, which is how a restart
    /// that died between the end and the delete finishes it.
    pub(crate) async fn repair_branch_given_back(
        &self,
        job: &Job,
        subject: &Subject,
        record: &RepairRecord,
    ) {
        let Some(branch) = record.branch.as_deref() else {
            return;
        };
        // Whatever asked, a pull request's head is never deleted from here.
        if record.choice == Some(core_model::FixChoice::NewPr) {
            return;
        }
        let Ok(served) = self.served_by(job) else {
            return;
        };
        let Ok(spec) = self.repair_spec(&served, job, subject) else {
            return;
        };
        // Only the branch Fleet cut for this repair is ever deleted.
        if spec.branch() != branch {
            return;
        }
        let (level, said) = match self.vcs().delete_repair_branch(&spec) {
            Ok(true) => (Level::Info, format!("repair branch {branch} given back")),
            Ok(false) => return,
            Err(adapter_traits::BranchKept(why)) => (
                Level::Warn,
                format!("repair branch {branch} could not be deleted: {why}"),
            ),
        };
        self.logged(job.id(), self.trigger_line(job, level, &said));
    }

    /// Give back the branches of repairs that ended before their branch was
    /// deleted, which is a Fleet that stopped between the two.
    pub(crate) async fn repair_branches_swept(&self) {
        let (firings, additions) = {
            let store = self.store().lock().await;
            (
                store.repair_branches_left().unwrap_or_default(),
                store.addition_repair_branches_left().unwrap_or_default(),
            )
        };
        let left = firings
            .into_iter()
            .map(|(job, id, firing)| (job, Subject::Firing(id), firing.repair))
            .chain(
                additions
                    .into_iter()
                    .map(|(job, added)| (job, Subject::Addition(added.id), added.repair)),
            );
        for (job_id, subject, record) in left {
            if let Ok(job) = self.load(&job_id).await {
                self.repair_branch_given_back(&job, &subject, &record).await;
            }
        }
    }
}
