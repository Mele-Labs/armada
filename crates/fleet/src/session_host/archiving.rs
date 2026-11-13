//! `archive_artifact` and `restore_artifact`: the agent or the person folds a stale artifact away, and brings
//! it back. **Fleet never archives one itself**: it cannot tell which artifact replaces which, so it does not
//! guess. `docs/concepts/artifact.md`, *Archive*.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ArtifactBy, ArtifactRef, JobArtifact, SendSessionMessage, SessionId};
use store::KeptArchive;

use crate::daemon::Fleet;

const ARTIFACT_UNNAMED: &str = "fleet.artifact_unnamed";
const ARTIFACT_NOT_HELD: &str = "fleet.artifact_not_held";
const ARTIFACT_ARCHIVED_ALREADY: &str = "fleet.artifact_archived_already";
const ARTIFACT_NOT_ARCHIVED: &str = "fleet.artifact_not_archived";
/// The most ids a refusal names.
const MOST_NAMED: usize = 20;

/// What the Job is sent when he archives one that waited on him.
pub(crate) fn archived_told(title: &str) -> String {
    format!("He archived \"{title}\". It is off his list with no verdict.")
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
    /// Archive one artifact. Where it waits on him, its item is dismissed in the same write, and where he
    /// archived it the Job is told first, so a send that fails leaves it as it was.
    pub(crate) async fn artifact_archived(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        by: ArtifactBy,
        archive: ArtifactRef,
    ) -> Result<JobArtifact, Refusal> {
        let id = self.artifact_job(caller, archive.session_id)?;
        let artifact = self.artifact_held(&id, &archive.artifact).await?;
        let already = || self.hosted_refusal(ARTIFACT_ARCHIVED_ALREADY, &format!("`{}` is archived already", artifact.id));
        if artifact.archived.is_some() {
            return Err(already());
        }
        if by == ArtifactBy::Owner && artifact.item.is_some() {
            let terminal = self.terminal_session(&id).await?;
            let told = SendSessionMessage {
                session_id: SessionId::carried(&id),
                text: archived_told(&artifact.title),
                attachments: Vec::new(),
                mentions: Vec::new(),
                sketches: Vec::new(),
            };
            Arc::clone(&self).send_known(terminal, told).await?;
        }
        let kept = KeptArchive {
            artifact: artifact.id.clone(),
            by: String::from(match by {
                ArtifactBy::Agent => "agent",
                ArtifactBy::Owner => "owner",
            }),
            at: self.instant().as_str().to_string(),
            item: artifact.item.clone(),
        };
        if !self.store().lock().await.archive_artifact(&id, &kept).map_err(|why| self.ledger_fault(why))? {
            return Err(already());
        }
        self.published_hosted(&id).await?;
        self.artifact_held(&id, &artifact.id).await
    }

    /// Lift one artifact's archive, and the dismissal the archive wrote. Nothing is sent.
    pub(crate) async fn artifact_restored(&self, caller: Option<api::Caller>, restore: ArtifactRef) -> Result<JobArtifact, Refusal> {
        let id = self.artifact_job(caller, restore.session_id)?;
        let artifact = self.artifact_held(&id, &restore.artifact).await?;
        let lifted = self.store().lock().await.restore_artifact(&id, &artifact.id).map_err(|why| self.ledger_fault(why))?;
        if lifted.is_none() {
            return Err(self.hosted_refusal(ARTIFACT_NOT_ARCHIVED, &format!("`{}` is not archived", artifact.id)));
        }
        self.published_hosted(&id).await?;
        self.artifact_held(&id, &artifact.id).await
    }

    /// The Job the connection places, else the one the body names.
    fn artifact_job(&self, caller: Option<api::Caller>, named: Option<SessionId>) -> Result<String, Refusal> {
        caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(ARTIFACT_UNNAMED, "a job whose artifact it is"))
    }

    /// The artifact the Job holds as `named`, refused naming the ids it does hold.
    pub(super) async fn artifact_held(&self, id: &str, named: &str) -> Result<JobArtifact, Refusal> {
        let named = named.trim();
        let held = self.job_artifacts(id).await?;
        if let Some(artifact) = held.iter().find(|one| one.id == named) {
            return Ok(artifact.clone());
        }
        let ids: Vec<&str> = held.iter().take(MOST_NAMED).map(|one| one.id.as_str()).collect();
        let holds = match ids.is_empty() {
            true => String::from("none"),
            false => ids.join(", "),
        };
        Err(self.hosted_refusal(ARTIFACT_NOT_HELD, &format!("this job holds no artifact `{named}`. Artifacts it holds: {holds}")))
    }
}
