//! The fetch every catch-up opens with.
//!
//! A catch-up merges the remote's copy of the base, never the local branch of
//! the same name, which is rarely fast-forwarded (Job 14, 12 Oct 2026).
//! `adapters` reads whatever the last fetch left and does not fetch, so this is
//! where a failure is seen.
//!
//! **A fetch that fails stops the catch-up.** Merging the local branch, or the
//! copy of the remote's an earlier fetch left, would report the branch up to
//! date against a base the forge no longer has: the exact failure this exists
//! for, and silent. The Job's log says which remote, which base and what git
//! said. The cost is that a Job whose remote is unreachable cannot start a
//! step — but its push would be refused for the same reason, and it is
//! stopped here with the reason instead of at a pull request that conflicts.

use std::sync::Arc;

use adapter_traits::{AgentHarness, BaseFetched, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, JobId, Level};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

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
    /// Fetch `base` from the remote, in the repository `dir` is in.
    ///
    /// `Ok` is a base now current, or one with no remote copy to be stale
    /// against. `Err` is a fetch that failed, already written to the Job's log
    /// at `Warn`. **Callers hold `merge_end`**: two fetches of one tracking ref
    /// can refuse each other.
    pub(crate) async fn base_fetched(
        &self,
        job_id: &JobId,
        dir: &str,
        base: &str,
    ) -> Result<(), Adrift> {
        let (vcs, dir, owned_base) = (Arc::clone(self.vcs()), dir.to_string(), base.to_string());
        // Off the runtime: a fetch is the network, and is bounded only by its
        // own timeout. `#693`.
        let fetched = tokio::task::spawn_blocking(move || vcs.fetch_base(&dir, &owned_base))
            .await
            .expect("git panicked fetching the base");
        let BaseFetched::Failed { remote, said } = fetched else {
            return Ok(());
        };
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            format!(
                "the base `{base}` could not be fetched from `{remote}`, so the branch was not \
                 brought up to date: merging a base this machine holds an old copy of would \
                 leave the pull request conflicting"
            ),
        )
        .in_job(job_id.as_ulid().clone())
        .with_field("remote", FieldValue::Str(remote.clone()))
        .with_field("base", FieldValue::Str(base.to_string()))
        .with_field("cause", FieldValue::Str(said.clone()));
        self.noted_in_the_log(job_id, &envelope);
        Err(Adrift::NotDelivered {
            job: job_id.clone(),
            doing: "fetching the base from the remote",
            said: format!("`{remote}/{base}`: {said}"),
        })
    }
}
