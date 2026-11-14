//! `mark_artifact_seen`: the newest revision of an artifact he opened. **Nothing is sent to the Job**: it is what
//! Bridge reads to tell him what changed since he last looked. `docs/concepts/artifact.md`, *Seen and anchored notes*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ArtifactSeen, MarkArtifactSeen};

use super::page_sealed::ARTIFACT_REVISION_UNKNOWN;
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
    /// Keep that he opened `mark.revision`, where it is newer than what is kept. **An older one is answered with
    /// the newest, not refused.** Archived or not: opening changes nothing else. `SessionChanged` is published
    /// where `seen` moved, so a Bridge reads it again, and not otherwise.
    pub(crate) async fn artifact_seen_marked(&self, mark: MarkArtifactSeen) -> Result<ArtifactSeen, Refusal> {
        let id = mark.session_id.as_str().trim().to_string();
        let held = self.artifact_held(&id, &mark.artifact).await?;
        let newest = held.revisions.last().map_or(1, |one| one.number);
        if mark.revision == 0 || mark.revision > newest {
            return Err(self.hosted_refusal(
                ARTIFACT_REVISION_UNKNOWN,
                &format!("revision {} of `{}` is not kept. Kept: 1 to {newest}", mark.revision, held.title),
            ));
        }
        let (seen, moved) = self
            .store()
            .lock()
            .await
            .keep_artifact_seen(&id, &held.id, mark.revision)
            .map_err(|why| self.ledger_fault(why))?;
        if moved {
            self.published_hosted(&id).await?;
        }
        Ok(ArtifactSeen { seen })
    }
}
