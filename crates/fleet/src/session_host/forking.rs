//! A session started as a copy of an ended or dead one's conversation. Since
//! 23.61. `docs/concepts/session.md`, *A forked session*.
//!
//! **The fork is a new session.** It has its own id and ledger, holds no slot
//! and no branch of the old one's, and takes a slot on its first write like any
//! other start. What it takes from the old one is the conversation, which the
//! agent copies when its first process starts, and the title.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ForkFrom, SessionRecord, StartSession, WireError};
use store::{AttachmentState, Holder, KeptAttachment, KeptHosting, KeptSession};

use super::rows::new_id;
use super::serving::{mode_text, NO_SUCH_SESSION};
use crate::daemon::Fleet;

/// A fork of a session that is still running. A 409.
const FORK_LIVE: &str = "fleet.session_fork_live";

/// On the old session's ledger, naming the fork.
pub(crate) const FORKED_TO: &str = "forked_to";
/// On the fork's ledger, naming the session it came from.
pub(crate) const FORKED_FROM: &str = "forked_from";

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
    pub(crate) async fn start_forked(
        &self,
        start: StartSession,
        from: ForkFrom,
    ) -> Result<SessionRecord, Refusal> {
        let old_id = from.session_id.as_str().to_string();
        let served = self.served_named(Some(&start.manifest_id))?;
        let manifest = served.manifest().id().as_str().to_string();
        let id = new_id();
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let old = store
                .session(&old_id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| {
                    self.hosted_refusal(NO_SUCH_SESSION, &format!("no session is named {old_id}"))
                })?;
            let hosted = store
                .hosting(&old_id)
                .map_err(|why| self.ledger_fault(why))?
                .is_some();
            // Ended, or a terminal session whose mod stopped asking. A hosted
            // session that is not closed is live whatever its process does.
            let dead = old.state == store::SessionState::Ended
                || (!hosted && !self.hosts().terminals().listening(&old_id));
            if !dead {
                return Err(Refusal::IllegalMove(WireError::raised(
                    FORK_LIVE,
                    format!("session {old_id} is still running, and only an ended one is forked"),
                    self.run_id(),
                )));
            }
            let title = start
                .title
                .map(|title| title.trim().to_string())
                .filter(|title| !title.is_empty())
                .or(old.title.clone());
            store
                .keep_session(&KeptSession {
                    id: id.clone(),
                    harness: String::from(adapters::HOSTED_HARNESS),
                    origin: String::from("bridge"),
                    manifest_id: Some(manifest.clone()),
                    cwd: served.root().to_string(),
                    title,
                    state: store::SessionState::Live,
                    started_at: now.clone(),
                    last_seen_at: now.clone(),
                    last_turn_at: None,
                    ended_at: None,
                    end_reason: None,
                    figures: Default::default(),
                })
                .map_err(|why| self.ledger_fault(why))?;
            store
                .keep_hosting(&KeptHosting {
                    session_id: id.clone(),
                    manifest_id: manifest.clone(),
                    model: start.model.filter(|model| !model.trim().is_empty()),
                    effort: start.effort.filter(|effort| !effort.trim().is_empty()),
                    mode: mode_text(start.mode.unwrap_or_default()).to_string(),
                    ran: false,
                    lease_slot: None,
                    lease_branch: None,
                    fork_of: Some(old_id.clone()),
                })
                .map_err(|why| self.ledger_fault(why))?;
            // `spent`, not `standing`: the old session ending gives back what
            // it holds, and the link is a fact that stays.
            for (holder, kind, target) in [(&id, FORKED_FROM, &old_id), (&old_id, FORKED_TO, &id)] {
                store
                    .attach(
                        &KeptAttachment {
                            holder: Holder::session(holder),
                            kind: kind.to_string(),
                            manifest_id: manifest.clone(),
                            target: target.clone(),
                            state: AttachmentState::Spent,
                            detail: Default::default(),
                            since: now.clone(),
                            changed_at: now.clone(),
                        },
                        false,
                    )
                    .map_err(|why| self.ledger_fault(why))?;
            }
        }
        let _ = self.published_hosted(&old_id).await;
        self.published_hosted(&id).await
    }
}
