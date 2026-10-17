//! What Fleet answers when it is asked: the boot read and the reads every surface makes after it.
//!
//! **A read never quietly shortens.** [`Fleet::every_job`] answers with the rows that would not
//! load beside the ones that did: a caller handed a short list with nothing in the signature
//! saying so cannot tell it from a complete one.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobId, TransitionReason};
use store::{LoadAllError, LoadJobError, Loaded, Moved};

use super::Fleet;
use crate::adrift::Adrift;
use crate::transcript::Rekeyed;

/// What the boot read found, and what the reconciliation did about it.
#[derive(Debug, Default)]
pub struct Reconciled {
    /// Rows whose cached status disagreed with the log and were corrected.
    pub repaired: usize,
    /// Rows that would not rebuild at all. **Never dropped** — carried out so a caller cannot end
    /// up holding a short list with nothing saying so.
    pub unreadable: Vec<String>,
    /// What was moved out from under a ULID and under the handle beside it —
    /// `crate::transcript::migrating`. Zeroes on every boot after the first.
    pub rekeyed: Rekeyed,
    /// Links on a Studio whose address an adapter recognised, written back as the Issue, Pull
    /// request or Epic they name. Nought on every boot after the one that converted them.
    pub recognised: usize,
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
    /// **The boot read, and the reconciliation.** No command is answered until it has: `armada
    /// serve` runs it in a task after the router is up, with reads and health answering meanwhile
    /// and commands held by `api::Reconciliation`.
    pub async fn reconcile(&self) -> Result<Reconciled, Adrift> {
        // First of all: the servers a crashed Fleet left running are ended, so the ports they
        // held are free before the probe below. See `crate::servers::left`.
        for repository in self.repositories().every() {
            self.reaped_left_servers(repository.records_root()).await;
        }
        // Before anything else: a main-checkout claim a crashed Fleet left behind is re-probed
        // before this process trusts it. See `crate::ports::Fleet::reconciled_checkout_ports`.
        self.reconciled_checkout_ports().await;
        // A scout does not outlive the Fleet reading it. `crate::scouting`.
        self.scouts_left_gathering().await;
        self.rescues_left_reading().await;
        // Housekeeping: a store that will not delete now leaves them listed.
        let _ = self.store().lock().await.forget_blank_sessions();
        // Before any Studio is read: the Links this build reads as an Issue, a Pull request or an
        // Epic. `crate::recognising`.
        let recognised = self.links_recognised().await;
        let (loaded, unreadable) = self.every_job().await?;
        // Before anything else reads a path: every Job's name. See [`mod@crate::naming`].
        self.names().learn_all(&loaded.jobs);
        let mut reconciled = Reconciled {
            repaired: loaded.repaired.len(),
            unreadable,
            recognised,
            ..Reconciled::default()
        };
        for served in self.repositories().served() {
            self.reconciled_in_part(&served, &loaded.jobs, &mut reconciled)
                .await;
        }
        Ok(reconciled)
    }

    /// The same cleanup for one repository added after Fleet started: the servers it left and the
    /// Jobs the store already holds for it.
    pub(crate) async fn reconciled_in(
        &self,
        served: &crate::repositories::Served,
    ) -> Result<Reconciled, Adrift> {
        self.reaped_left_servers(served.records_root()).await;
        let (loaded, _) = self.every_job().await?;
        self.names().learn_all(&loaded.jobs);
        let mut reconciled = Reconciled::default();
        self.reconciled_in_part(served, &loaded.jobs, &mut reconciled)
            .await;
        Ok(reconciled)
    }

    /// One repository's Jobs, out of every Job the boot read found.
    async fn reconciled_in_part(
        &self,
        served: &crate::repositories::Served,
        every: &[Job],
        reconciled: &mut Reconciled,
    ) {
        let jobs: Vec<Job> = every
            .iter()
            .filter(|job| served.owns(job.owner_manifest_id().as_str()))
            .cloned()
            .collect();
        // Once, the clone's needs into the ledger. `crate::needing`, `crate::ledgering`.
        self.needs_converted(served, &jobs).await;
        let rekeyed = crate::transcript::rekeyed(served.records_root(), &jobs).await;
        reconciled.rekeyed.logs += rekeyed.logs;
        reconciled.rekeyed.transcripts += rekeyed.transcripts;
        reconciled.rekeyed.refused.extend(rekeyed.refused);
        // A slot still held by a session that ended: parked now, before anything looks for a
        // free one.
        self.freed_from_ended_sessions(served).await;
    }

    /// Every Job, **and every row that would not load**.
    ///
    /// The store refuses to hand back a short list; this refuses to complete
    /// one. Both halves come out together and the caller decides.
    pub async fn every_job(&self) -> Result<(Loaded, Vec<String>), Adrift> {
        match self.store.lock().await.load_all_jobs() {
            Ok(loaded) => Ok((loaded, Vec::new())),
            Err(LoadAllError::SomeJobsUnreadable { loaded, failed }) => Ok((
                loaded,
                failed.iter().map(|refusal| refusal.to_string()).collect(),
            )),
            Err(fault) => Err(Adrift::BootRead(fault)),
        }
    }

    /// One Job, folded from its events. The status column is not read.
    ///
    /// **Every read takes the Job's name**, which is what keeps
    /// [`mod@crate::naming`] total for a Job created by a Fleet that has since
    /// been restarted with an older store.
    pub async fn load(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self
            .store
            .lock()
            .await
            .load_job(job_id)
            .map_err(Adrift::Reading)?;
        self.learn_the_name(&job);
        Ok(job)
    }

    /// The qualifying reason the Job's last transition stored, where it stored
    /// one. Read from the log, because the `jobs` row does not carry it and
    /// only a caller holding the log can supply it.
    pub async fn last_reason(&self, job_id: &JobId) -> Result<Option<TransitionReason>, Adrift> {
        let events = self
            .store
            .lock()
            .await
            .events_for(job_id)
            .map_err(|cause| Adrift::Reading(LoadJobError::Unreadable(cause)))?;
        Ok(events.iter().rev().find_map(|event| match event.moved() {
            Moved::Job { reason, .. } => Some(reason.clone()),
            Moved::Step { .. } | Moved::Drone { .. } => None,
        }))
    }

    /// The instant this Job first reached `running` — [`JobSummary`](ipc::JobSummary)'s
    /// `started_at`. Absent until the Job's first Drone starts, and unmoved by
    /// a later return to `awaiting_approval` or `queued`: `crate::wire::job_started_at`
    /// keeps only the first arrival.
    pub async fn job_started_at(
        &self,
        job_id: &JobId,
    ) -> Result<Option<core_model::Timestamp>, Adrift> {
        let store = self.store.lock().await;
        crate::wire::job_started_at(&store, job_id).map_err(Adrift::Reading)
    }

    /// The instant this Job arrived at a terminal status —
    /// [`JobSummary`](ipc::JobSummary)'s `ended_at`. Absent until the Job is
    /// over. Overview 28 (#1092).
    pub async fn job_ended_at(
        &self,
        job_id: &JobId,
    ) -> Result<Option<core_model::Timestamp>, Adrift> {
        let store = self.store.lock().await;
        crate::wire::job_ended_at(&store, job_id).map_err(Adrift::Reading)
    }

    /// The stream Fleet publishes transitions on. Cloned for the listener.
    pub fn events(&self) -> api::Broadcaster {
        self.events.clone()
    }
}
