//! The Manifest a Job actually sees — what was snapshotted at its creation
//! (`docs/concepts/drone.md`), never whatever `armada.yml` says by the time
//! somebody asks.
//!
//! [`effective_manifest`](Fleet::effective_manifest) is where every reader
//! `#650` names — `crate::spawning`'s toolbelt, `crate::preparing`'s setup,
//! `crate::rehearsing`'s run sheet — resolves instead of reading
//! `Fleet::manifest` live.
//!
//! **Text, re-parsed on every read.** `crate::store` sits under `config`, not
//! beside it, so it cannot hold a `config::Manifest` — only `armada.yml`'s own
//! bytes, which this module turns back into one. A snapshot that cannot be
//! taken or will not parse degrades to the live Manifest rather than failing
//! the Job: the same reading every Job got before this existed.

use config::Manifest;
use core_model::Job;

use crate::daemon::Fleet;

impl<H, V, W> Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{

    /// What this Job's Drones, gate and run route resolve their Commands,
    /// Checks and Setup against, and whether that is actually what the Job
    /// froze.
    ///
    /// **The snapshot, re-parsed — or, absent one, Fleet's live Manifest, and
    /// the second element of the pair says which.** `false` is a Job created
    /// before `#650`'s migration, and reading it the live way is not a
    /// special case: it is what every one of these callers already did, so a
    /// pre-migration Job is unaffected rather than downgraded. A snapshot
    /// that will not parse — a build older than the one that wrote it,
    /// reading a shape it does not have — degrades the same way and answers
    /// `false` too, for [`manifest_snapshotted`](Fleet::manifest_snapshotted)'s
    /// reason: a Job already dispatched must not stop working over this, and
    /// a caller that cannot trust what it read must not claim it as frozen.
    pub(crate) async fn effective_manifest(
        &self,
        job: &Job,
    ) -> Result<(Manifest, bool), crate::adrift::Adrift> {
        let served = self.served_by(job)?;
        Ok(self.effective_manifest_in(&served, job).await)
    }

    /// [`effective_manifest`](Fleet::effective_manifest), for a caller already
    /// holding the Job's repository.
    pub(crate) async fn effective_manifest_in(
        &self,
        served: &crate::repositories::Served,
        job: &Job,
    ) -> (Manifest, bool) {
        let snapshot = self
            .store()
            .lock()
            .await
            .manifest_snapshot(job.id())
            .ok()
            .flatten();
        match snapshot.and_then(|text| Manifest::parse(served.manifest().path(), &text).ok()) {
            Some(parsed) => (parsed, true),
            None => (served.manifest().clone(), false),
        }
    }
}
