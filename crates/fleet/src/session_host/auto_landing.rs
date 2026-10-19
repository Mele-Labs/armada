//! Auto land on a hosted Session: the forge's auto-merge follows the flag. Turned on, every open
//! pull request the Session holds is asked for it, and so is each that attaches later (the pull
//! watch's attach and `claim_pull_request`); turned off, the open ones are asked to stop.
//! `docs/concepts/session.md`, *Auto land*.
//!
//! **The flag changes first and the forge is asked after**, so a pull request the forge refuses is
//! a refusal that carries the forge's words and leaves the flag as the person set it. A pull
//! request that has merged or closed is left alone, and a ledger row that lags the forge is settled.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{PullRequests, Refusal};
use ipc::{ManifestId, PullRequestStanding, SessionRecord, SetSessionAutoLand};
use store::{AttachmentState, Holder};

use crate::daemon::Fleet;
use crate::pull_requesting::{AUTO_MERGE_REFUSED, PR};

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
    /// `set_session_auto_land`. **Refused, nothing changed**, for a terminal Job (whose pull requests
    /// are not Fleet's to land) and an ended one. Otherwise the flag is kept and published, and
    /// then each open pull request is asked; the forge's refusals come back after, as one.
    pub(crate) async fn session_auto_land_set(
        &self,
        asked: SetSessionAutoLand,
    ) -> Result<SessionRecord, Refusal> {
        let id = asked.id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                super::serving::SESSION_COMMAND_REFUSED,
                &format!("job {id} runs in a terminal, and only a hosted job lands from here"),
            ));
        }
        let (session, mut hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.hosted_refusal(
                super::serving::SESSION_COMMAND_REFUSED,
                &format!("job {id} has ended, and its pull requests are not landed from here"),
            ));
        }
        hosting.auto_land = asked.on;
        self.store().lock().await.keep_hosting(&hosting).map_err(|why| self.ledger_fault(why))?;

        let manifest = ManifestId::carried(&hosting.manifest_id);
        let numbers = self.open_pulls_of(&id, &hosting.manifest_id).await;
        let mut refused = Vec::new();
        for number in numbers {
            let done = match asked.on {
                true => self.auto_merge_asked_for(&manifest, number).await,
                false => self.auto_merge_withdrawn(&manifest, number).await,
            };
            if let Err(refusal) = done {
                refused.push(refusal);
            }
        }
        let record = self.published_hosted(&id).await?;
        match refused.len() {
            0 => Ok(record),
            1 => Err(refused.remove(0)),
            _ => Err(Refusal::IllegalMove(self.pr_refusal(
                AUTO_MERGE_REFUSED,
                refused.iter().map(|one| one.error().message.as_str()).collect::<Vec<_>>().join("; "),
            ))),
        }
    }

    /// The numbers of the pull requests a Session holds standing, in one repository.
    async fn open_pulls_of(&self, session: &str, manifest: &str) -> Vec<u64> {
        let store = self.store().lock().await;
        let rows = store.attachments_of(&Holder::session(session)).unwrap_or_default();
        rows.iter()
            .filter(|row| row.kind == PR && row.state == AttachmentState::Standing)
            .filter(|row| row.manifest_id == manifest)
            .filter_map(|row| row.target.parse().ok())
            .collect()
    }

    /// Auto land's ask of the forge for one pull request: skipped where it is merged or closed
    /// (the read settles its row), and refused as `enable_auto_merge` refuses otherwise.
    async fn auto_merge_asked_for(&self, manifest: &ManifestId, number: u64) -> Result<(), Refusal> {
        let served = self.served_named(Some(manifest))?;
        let before = self.pull_request_read(manifest, served.root(), number)?;
        if matches!(before.state, PullRequestStanding::Merged | PullRequestStanding::Closed) {
            self.ledger_follows(&before).await;
            return Ok(());
        }
        self.enable_auto_merge(manifest.clone(), number).await.map(|_| ())
    }

    /// Take auto-merge off one pull request that is open and has it. The forge's words come back
    /// as `fleet.auto_merge_refused`, as turning it on does.
    async fn auto_merge_withdrawn(&self, manifest: &ManifestId, number: u64) -> Result<(), Refusal> {
        let served = self.served_named(Some(manifest))?;
        let root = served.root();
        let before = self.pull_request_read(manifest, root, number)?;
        if before.state != PullRequestStanding::Open || !(before.auto_merge || before.queued) {
            self.ledger_follows(&before).await;
            return Ok(());
        }
        self.vcs()
            .disable_auto_merge(root, &number.to_string())
            .map_err(|said| Refusal::IllegalMove(self.pr_refusal(AUTO_MERGE_REFUSED, said)))?;
        let after = ipc::PullRequestState { auto_merge: false, ..before };
        let state = self.pull_request_read(manifest, root, number).unwrap_or(after);
        self.ledger_follows(&state).await;
        Ok(())
    }

    /// A pull request just attached to a Session: asked for auto-merge where Auto land is on.
    /// `None` where it is off, or the forge took it; the forge's words where it did not.
    pub(crate) async fn auto_land_attached(&self, session: &str, manifest: &str, number: u64) -> Option<String> {
        let on = self.store().lock().await.hosting(session).ok().flatten().is_some_and(|it| it.auto_land);
        if !on {
            return None;
        }
        self.auto_merge_asked_for(&ManifestId::carried(manifest), number)
            .await
            .err()
            .map(|refusal| refusal.error().message.clone())
    }
}
