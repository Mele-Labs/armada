//! A Job's sketches, drafts and pages across his Macs. The Mac that holds a Job lists them and seals a page for the Mac
//! that asks, through the share filter; the Mac that asks reads them through the link each time, as it reads a thread,
//! and keeps none. A page's margin threads are read the same way. His acts on them cross as `JobAct` kinds
//! (`crate::mac_acts`).
//! `docs/practices/protocol/his-other-macs-fleets-half.md`, *Acting across Macs*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, SealedPage};
use ipc::{ArtifactPageShared, GetMarginThreads, JobArtifact, MarginThreads, SessionId, WireError};

use crate::daemon::Fleet;
use crate::other_macs::{MAC_UNREACHABLE, MAC_UNREADABLE};

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
    /// `path`, one of Mac `mac`'s `/other_mac/*` reads, decoded as `what`. **What it refuses is passed on as it was said there.**
    pub(crate) async fn read_from_other_mac<T: serde::de::DeserializeOwned>(&self, mac: &str, path: &str, what: &'static str) -> Result<T, Refusal> {
        let answered = self.phone_switch()?.read_other_mac(mac, path).await;
        let (status, bytes) = answered.map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREACHABLE, why, self.run_id())))?;
        if (200..300).contains(&status) {
            return ipc::decode(what, &bytes)
                .map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREADABLE, format!("Mac {mac} sent {what} this build cannot read: {why}"), self.run_id())));
        }
        Err(self.refusal_passed_on(mac, status, &bytes))
    }

    /// A Job held from Mac `mac`: its artifacts, read from that Mac's own Fleet.
    pub(crate) async fn artifacts_from_other_mac(&self, mac: &str, job: &SessionId) -> Result<Vec<JobArtifact>, Refusal> {
        let path = format!("/other_mac/artifacts?session_id={}", ipc::door::encoded(job.as_str()));
        self.read_from_other_mac(mac, &path, "a Job's artifacts").await
    }

    /// A Job held from Mac `mac`: one revision of its page, sealed by that Mac's own Fleet, for a host that draws the
    /// kit itself where `host_kit`.
    pub(crate) async fn artifact_page_from_other_mac(
        &self,
        mac: &str,
        job: &SessionId,
        artifact: &str,
        revision: Option<u32>,
        host_kit: bool,
    ) -> Result<SealedPage, Refusal> {
        let mut path = format!("/other_mac/artifact_page?session_id={}&artifact={}", ipc::door::encoded(job.as_str()), ipc::door::encoded(artifact));
        if let Some(revision) = revision {
            path.push_str(&format!("&revision={revision}"));
        }
        if host_kit {
            path.push_str("&host_kit=true");
        }
        let page: ArtifactPageShared = self.read_from_other_mac(mac, &path, "a Job's page").await?;
        Ok(SealedPage { revision: page.revision, html: page.html, policy: page.policy })
    }

    /// A Job held from Mac `mac`: one page's margin threads, read from that Mac's own Fleet.
    pub(crate) async fn margin_from_other_mac(&self, mac: &str, get: &GetMarginThreads) -> Result<MarginThreads, Refusal> {
        let path = format!("/other_mac/margin?session_id={}&document={}", ipc::door::encoded(get.session_id.as_str()), ipc::door::encoded(&get.document));
        self.read_from_other_mac(mac, &path, "a page's margin threads").await
    }

    /// Job `job` of this Mac's artifacts, for another Mac. **Refused as an act on a Job that does not exist** where the Job is not shared.
    pub(crate) async fn job_artifacts_shared(&self, job: &SessionId) -> Result<Vec<JobArtifact>, Refusal> {
        self.act_from_other_mac(job).await?;
        self.job_artifacts(job.as_str()).await
    }

    /// One revision of Job `job`'s page, sealed, for another Mac, refused as [`Self::job_artifacts_shared`] is.
    pub(crate) async fn job_artifact_page_shared(&self, job: &SessionId, artifact: &str, revision: Option<u32>, host_kit: bool) -> Result<ArtifactPageShared, Refusal> {
        self.act_from_other_mac(job).await?;
        let page = match host_kit {
            true => self.hosted_artifact_page(job.as_str(), artifact, revision).await?,
            false => self.artifact_page(job.as_str(), artifact, revision).await?,
        };
        Ok(ArtifactPageShared { revision: page.revision, html: page.html, policy: page.policy })
    }

    /// Job `job`'s margin threads on one page, for another Mac, refused as [`Self::job_artifacts_shared`] is.
    pub(crate) async fn job_margin_shared(&self, get: GetMarginThreads) -> Result<MarginThreads, Refusal> {
        self.act_from_other_mac(&get.session_id).await?;
        self.margin_threads_read(get).await
    }
}
