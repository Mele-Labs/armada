//! A Job's own slot and branch as rows on the session ledger, so `who_owns` can
//! name a Job that holds a slot and a Job that moves slots gives the old one
//! back. `docs/capabilities/needs.md`, *Not decided*, closed here;
//! `docs/concepts/session.md`.
//!
//! **Written where Fleet already decides it**: `slotted` and `reseat` take the
//! slot, `branded` takes the branch, and the places that give a slot back give
//! the row back with it. Both kinds are exclusive, as a session's are, so
//! taking a second slot is giving the first back in the same write.
//!
//! **A fault is a line in the Job's log and never fails the lease or the
//! branch**: the pool and the Job's own record are the authority, and this is
//! a reading of them.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, Level};
use store::{AttachmentState, Holder, KeptAttachment};

use crate::daemon::Fleet;

const SLOT: &str = "slot";
const BRANCH: &str = "branch";

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
    fn job_row(&self, job: &Job, kind: &str, target: String) -> KeptAttachment {
        let now = self.now().as_str().to_string();
        KeptAttachment {
            holder: Holder::job(job.id().as_str()),
            kind: kind.into(),
            manifest_id: job.owner_manifest_id().as_str().to_string(),
            target,
            state: AttachmentState::Standing,
            detail: Default::default(),
            since: now.clone(),
            changed_at: now,
        }
    }

    async fn job_took(&self, job: &Job, kind: &'static str, target: String) {
        let row = self.job_row(job, kind, target);
        let wrote = self.store().lock().await.attach(&row, true);
        if let Err(why) = wrote {
            self.said_about_the_ledger(
                job.id(),
                Level::Warn,
                "what the Job holds could not be written to the ledger",
                Some(&why.to_string()),
            );
        }
    }

    /// The Job holds `slot`, and the one it held before is given back.
    pub(crate) async fn job_holds_slot(&self, job: &Job, slot: u32) {
        self.job_took(job, SLOT, slot.to_string()).await;
    }

    /// The Job's work is on its branch.
    pub(crate) async fn job_holds_branch(&self, job: &Job) {
        if let Some(branch) = job.branch() {
            self.job_took(job, BRANCH, branch.as_str().to_string())
                .await;
        }
    }

    /// The pool took the Job's slot back, or parked its work and freed it.
    pub(crate) async fn job_gave_back_slot(&self, job: &Job) {
        let holder = Holder::job(job.id().as_str());
        let now = self.now().as_str().to_string();
        let gave = self
            .store()
            .lock()
            .await
            .give_back(&holder, Some(SLOT), &now);
        if let Err(why) = gave {
            self.said_about_the_ledger(
                job.id(),
                Level::Warn,
                "the Job's slot was given back and the ledger could not be told",
                Some(&why.to_string()),
            );
        }
    }

    /// Every Job still working in this repository holds its branch and its slot
    /// on the ledger, whenever it began. Idempotent: a row that stands changes
    /// nothing.
    pub(crate) async fn live_jobs_on_the_ledger(&self, manifest: &str, jobs: &[Job]) {
        for job in jobs.iter().filter(|job| {
            job.owner_manifest_id().as_str() == manifest && !job.status().is_terminal()
        }) {
            self.job_holds_branch(job).await;
            if let Some(slot) = job.worktree_slot().filter(|_| !job.is_parked()) {
                self.job_holds_slot(job, slot).await;
            }
        }
    }
}
