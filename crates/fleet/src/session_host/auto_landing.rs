//! Auto land on a hosted Session: the forge's auto-merge follows the flag. Turned on, every open
//! pull request the Session holds is asked for it, and so is each that attaches later (the pull
//! reading's attach and `claim_pull_request`) and each the pull reading reads passed; turned off, the
//! open ones are asked to stop.
//! `docs/concepts/session.md`, *Auto land*.
//!
//! **The flag changes first and the forge is asked after**, so a pull request the forge refuses is
//! a refusal that carries the forge's words and leaves the flag as the person set it. A pull
//! request that has merged or closed is left alone, and a ledger row that lags the forge is settled.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, PullQueue, Vcs, FollowedPull, WorkProduct};
use api::Refusal;
use ipc::{ManifestId, PullRequestStanding, SessionRecord, SetSessionAutoLand};
use store::{AttachmentState, Holder};

use crate::daemon::Fleet;
use crate::pull_owners::Owner;
use crate::pull_requesting::{AUTO_MERGE_REFUSED, PR};
use crate::repositories::Served;
use super::queue_guard::{Blocker, Guard};

/// The key a pull request's commit is kept under once Auto land has asked, or seen it queued.
const AUTO_LAND: &str = "auto_land";

/// The keys of the second ask, and of the tell after it, for a commit that is still not merging.
const AUTO_LAND_AGAIN: &str = "auto_land_again";
const AUTO_LAND_STUCK: &str = "auto_land_stuck";

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
    /// are not Fleet's to land) and an ended one. Otherwise the flag is kept and published and the
    /// reply goes; **each open pull request is asked after it**, in a task of its own, because a
    /// Job holding five asks the forge some twenty-five times and Bridge stops waiting at twenty
    /// seconds. The forge's refusals reach the Job as one `auto_land_refused` notice.
    pub(crate) async fn session_auto_land_set(
        self: &Arc<Self>,
        asked: SetSessionAutoLand,
    ) -> Result<SessionRecord, Refusal> {
        let (record, manifest) = self.auto_land_flag_set(&asked).await?;
        let (fleet, id) = (Arc::clone(self), asked.id.as_str().to_string());
        tokio::spawn(async move { fleet.auto_land_asked_of_forge(&id, &manifest, asked.on).await });
        Ok(record)
    }

    /// The flag alone, kept and published, with the Manifest its pull requests are in. What a
    /// delivery named in the first message takes, before any pull request exists.
    pub(crate) async fn auto_land_flag_set(&self, asked: &SetSessionAutoLand) -> Result<(SessionRecord, String), Refusal> {
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
        let record = self.published_hosted(&id).await?;
        Ok((record, hosting.manifest_id))
    }

    /// Each open pull request asked to follow the flag, after the reply. **Stops where the flag has
    /// changed since**, so an off pressed straight after an on is not undone by the on's asks.
    async fn auto_land_asked_of_forge(self: &Arc<Self>, id: &str, manifest: &str, on: bool) {
        let manifest_id = ManifestId::carried(manifest);
        let mut refused = Vec::new();
        for number in self.open_pulls_of(id, manifest).await {
            if self.auto_land_on(id).await != on {
                return;
            }
            let done = match on {
                true => self.auto_merge_asked_for(&manifest_id, number, None).await.map(|_| ()),
                false => self.auto_merge_withdrawn(&manifest_id, number).await,
            };
            if let Err(refusal) = done {
                refused.push(format!("#{number}: {}", refusal.error().message));
            }
        }
        if refused.is_empty() {
            return;
        }
        let (state, verb) = if on { ("on", "turn on") } else { ("off", "turn off") };
        let text = format!("Auto land is {state}, and the forge would not {verb} auto-merge for {}", refused.join("; "));
        let _ = self.told_by_fleet(id, &text, ipc::Notice::AutoLandRefused).await;
    }

    /// The numbers of the pull requests a Session holds standing, in one repository.
    pub(crate) async fn open_pulls_of(&self, session: &str, manifest: &str) -> Vec<u64> {
        let store = self.store().lock().await;
        let rows = store.attachments_of(&Holder::session(session)).unwrap_or_default();
        rows.iter()
            .filter(|row| row.kind == PR && row.state == AttachmentState::Standing)
            .filter(|row| row.manifest_id == manifest)
            .filter_map(|row| row.target.parse().ok())
            .collect()
    }

    /// Auto land's ask of the forge for one pull request: skipped where it is merged or closed
    /// (the read settles its row), and refused as `enable_auto_merge` refuses otherwise. **Not made
    /// while the pull request clashes with one ahead of it in the merge queue**, which is then the
    /// `Ok(Some(_))`; the pull reading tells the Job and asks again once the clash is gone.
    /// `head` is the commit where the caller has read it, else the forge is asked.
    pub(crate) async fn auto_merge_asked_for(&self, manifest: &ManifestId, number: u64, head: Option<&str>) -> Result<Option<Blocker>, Refusal> {
        let served = self.served_named(Some(manifest))?;
        let before = self.pull_request_read_off(manifest, served.root(), number).await?;
        if matches!(before.state, PullRequestStanding::Merged | PullRequestStanding::Closed) {
            self.ledger_follows(&before).await;
            return Ok(None);
        }
        let head = match head {
            Some(head) => Some(head.to_string()),
            None => {
                let gate = served.manifest().ci_gate().to_string();
                let read = self.forge_asked(served.root(), move |vcs: &V, root: &str| vcs.followed_pull(root, &gate, number)).await;
                read.and_then(|pull| pull.head)
            }
        };
        if let Some(head) = head {
            match self.queue_guarded(&served, number, &head).await {
                Guard::Hold(blocker) | Guard::Dequeue(blocker) => return Ok(Some(blocker)),
                Guard::Go | Guard::Unchecked => {}
            }
        }
        self.enable_auto_merge(manifest.clone(), number).await.map(|_| None)
    }

    /// Take auto-merge off one pull request that is open and has it. The forge's words come back
    /// as `fleet.auto_merge_refused`, as turning it on does.
    async fn auto_merge_withdrawn(&self, manifest: &ManifestId, number: u64) -> Result<(), Refusal> {
        let served = self.served_named(Some(manifest))?;
        let root = served.root();
        let before = self.pull_request_read_off(manifest, root, number).await?;
        if before.state != PullRequestStanding::Open || !(before.auto_merge || before.queued) {
            self.ledger_follows(&before).await;
            return Ok(());
        }
        self.forge_asked(root, move |vcs: &V, root: &str| vcs.disable_auto_merge(root, &number.to_string()))
            .await
            .map_err(|said| Refusal::IllegalMove(self.pr_refusal(AUTO_MERGE_REFUSED, said)))?;
        let after = ipc::PullRequestState { auto_merge: false, ..before };
        let state = self.pull_request_read_off(manifest, root, number).await.unwrap_or(after);
        self.ledger_follows(&state).await;
        Ok(())
    }

    /// A pull request just attached to a Session: asked for auto-merge where Auto land is on.
    /// `None` where it is off, or the forge took it, or it is held behind a clash (the pull reading
    /// tells the Job so); the forge's words where it did not.
    pub(crate) async fn auto_land_attached(&self, session: &str, manifest: &str, number: u64) -> Option<String> {
        self.auto_land_tried(session, manifest, number, None).await.err()
    }

    /// [`Self::auto_land_attached`], answering `Ok(Some(_))` for the pull request ahead that it is
    /// held behind.
    async fn auto_land_tried(&self, session: &str, manifest: &str, number: u64, head: Option<&str>) -> Result<Option<Blocker>, String> {
        if !self.auto_land_on(session).await {
            return Ok(None);
        }
        self.auto_merge_asked_for(&ManifestId::carried(manifest), number, head)
            .await
            .map_err(|refusal| refusal.error().message.clone())
    }

    pub(crate) async fn auto_land_on(&self, session: &str) -> bool {
        self.store().lock().await.hosting(session).ok().flatten().is_some_and(|it| it.auto_land)
    }

    /// The pull reading's half of Auto land, for a pull request however it attached: the one a Session
    /// opened itself is reported by its mod, and nothing asked for it then. One whose checks passed,
    /// that conflicts with nothing and that nothing merges is asked for auto-merge **once a commit**
    /// for each owner with Auto land on. A commit seen queued counts as asked, so a
    /// merge someone stopped is not started again, nor a commit the queue dropped.
    ///
    /// **A pull request that clashes with one ahead in the merge queue is not asked, and is not kept
    /// as asked**, so it is tested again on every reading and asked once the clash is gone. One
    /// already queued is tested against what is ahead of it each reading, and taken out if it
    /// now clashes.
    pub(crate) async fn auto_land_followed(self: &Arc<Self>, served: &Served, pull: &FollowedPull, head: &str) {
        let queued = pull.queue != PullQueue::Outside;
        let merging = pull.auto_merge || queued;
        let ready = pull.passed() && !pull.conflicting && !merging;
        if !merging && !ready {
            return;
        }
        let (root, manifest) = (served.root(), served.manifest().id().as_str());
        for owner in self.owners_of(served, pull.number, pull.branch.as_written(), false).await {
            let Owner::Session(id) = &owner;
            let recipient = owner.key();
            if queued && self.queued_retested(served, pull, head, id, &recipient).await {
                continue;
            }
            let asked = self.store().lock().await.pull_notice_kept(root, pull.number, head, AUTO_LAND, &recipient).unwrap_or(true);
            if asked && ready && self.auto_land_on(id).await {
                self.auto_land_stalled(root, pull, head, id, manifest, &recipient).await;
                continue;
            }
            if asked || (ready && !self.auto_land_on(id).await) {
                continue;
            }
            if ready {
                match self.auto_land_tried(id, manifest, pull.number, Some(head)).await {
                    Ok(Some(blocker)) => {
                        self.hold_told(root, id, &recipient, (pull.number, head), &blocker, false).await;
                        continue;
                    }
                    Ok(None) => {}
                    Err(why) => {
                        let number = pull.number;
                        let _ = self
                            .told_by_fleet(id, &format!("Auto land is on, and the forge would not turn on auto-merge for #{number}: {why}"), ipc::Notice::AutoLandRefused)
                            .await;
                    }
                }
            }
            // Only auto-merge seen is not asked: the forge drops that request when it queues the pull
            // request, and a pull request that then sits unqueued has to be asked again.
            if !ready && !queued {
                continue;
            }
            let now = self.now();
            let _ = self.store().lock().await.keep_pull_notice(root, pull.number, head, AUTO_LAND, &recipient, &now);
        }
    }

    async fn auto_land_kept(&self, root: &str, number: u64, head: &str, key: &str, recipient: &str) -> bool {
        self.store().lock().await.pull_notice_kept(root, number, head, key, recipient).unwrap_or(true)
    }

    /// A pull request asked for auto-merge at this commit that has passed and is neither on it nor
    /// queued: the forge took the request and nothing merges it, as when the merge queue dropped it
    /// between two readings. **Asked once more, then the Job is told**, each once a commit.
    async fn auto_land_stalled(self: &Arc<Self>, root: &str, pull: &FollowedPull, head: &str, session: &str, manifest: &str, recipient: &str) {
        let number = pull.number;
        if !self.auto_land_kept(root, number, head, AUTO_LAND_AGAIN, recipient).await {
            match self.auto_land_tried(session, manifest, number, Some(head)).await {
                Ok(Some(blocker)) => return self.hold_told(root, session, recipient, (number, head), &blocker, false).await,
                Ok(None) => {}
                Err(why) => {
                    let text = format!("Auto land is on, and the forge would not turn on auto-merge for #{number}: {why}");
                    let _ = self.told_by_fleet(session, &text, ipc::Notice::AutoLandRefused).await;
                }
            }
            let now = self.now();
            let _ = self.store().lock().await.keep_pull_notice(root, number, head, AUTO_LAND_AGAIN, recipient, &now);
        } else if !self.auto_land_kept(root, number, head, AUTO_LAND_STUCK, recipient).await {
            let text = format!(
                "Auto land is on, and #{number} passed its checks but is neither on auto-merge nor in the merge queue, so nothing will merge it.\n{}",
                pull.url.as_written()
            );
            if self.told_by_fleet(session, &text, ipc::Notice::AutoLandRefused).await.unwrap_or(false) {
                let now = self.now();
                let _ = self.store().lock().await.keep_pull_notice(root, number, head, AUTO_LAND_STUCK, recipient, &now);
            }
        }
    }

    /// A queued pull request of a Session with Auto land on, tested against the entries ahead of it:
    /// taken out of the queue (and off auto-merge, which would queue it again) where it now clashes,
    /// and the Job told. **True where it was taken out**, and its commit is no longer kept as asked,
    /// so it is asked again once the clash is gone.
    async fn queued_retested(self: &Arc<Self>, served: &Served, pull: &FollowedPull, head: &str, session: &str, recipient: &str) -> bool {
        if !self.auto_land_on(session).await {
            return false;
        }
        let Guard::Dequeue(blocker) = self.queue_guarded(served, pull.number, head).await else {
            return false;
        };
        let (root, named) = (served.root(), pull.number.to_string());
        if self.vcs().dequeue(root, &named).is_err() {
            return false;
        }
        let _ = self.vcs().disable_auto_merge(root, &named);
        let _ = self.store().lock().await.forget_pull_notices(root, pull.number, AUTO_LAND);
        self.hold_told(root, session, recipient, (pull.number, head), &blocker, true).await;
        true
    }
}
