//! Giving back the slot an old Job held, on the session ledger and in the bay history. Nothing
//! takes a slot for a Job any more; the Jobs that did are cleaned up through this.
//!
//! **A fault is a line in the Job's log and never fails the lease**: the pool is the authority,
//! and this is a reading of it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level};
use store::Holder;

use crate::daemon::Fleet;

const SLOT: &str = "slot";

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
    /// The pool took the Job's slot back, or parked its work and freed it.
    pub(crate) async fn job_gave_back_slot(&self, job: &Job) {
        self.hosts().slot_freed();
        let holder = Holder::job(job.id().as_str());
        let now = self.now().as_str().to_string();
        let gave = self
            .store()
            .lock()
            .await
            .give_back(&holder, Some(SLOT), &now);
        // The bay history closes whether or not the ledger had a row to give back.
        if let Err(why) = self.store().lock().await.bay_given_back(job.id(), &now) {
            self.said_about_the_ledger(
                job.id(),
                Level::Warn,
                "the Job's slot was given back and its bay history could not be told",
                Some(&why.to_string()),
            );
        }
        match gave {
            Ok(_) => {}
            Err(why) => self.said_about_the_ledger(
                job.id(),
                Level::Warn,
                "the Job's slot was given back and the ledger could not be told",
                Some(&why.to_string()),
            ),
        }
    }

    pub(crate) fn said_about_the_ledger(
        &self,
        job: &JobId,
        level: Level,
        said: &'static str,
        cause: Option<&str>,
    ) {
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.as_ulid().clone());
        if let Some(cause) = cause {
            envelope = envelope.with_field("cause", FieldValue::Str(cause.to_string()));
        }
        self.noted_in_the_log(job, &envelope);
    }
}
