//! A Session's agent starts a Session, or forks its own conversation into one, and the new one waits
//! for the person. Since 23.80. `docs/concepts/session.md`, *A Session an agent started*.
//!
//! **The new Session is created and listed at once, and nothing runs.** The agent's first message is
//! kept as an `unsent` attachment on its ledger and read back as a waiting item, so it is the
//! person's to send. His ordinary send settles both (`first_message_sent`), and ending the Session
//! gives the attachment back with everything else it holds.
//!
//! **The caller is placed by its connection and never named**, so a fork can only be of the
//! caller's own conversation.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, HostedSessions, Refusal};
use ipc::{
    ForkFrom, ForkThisSession, ManifestId, SessionHeld, SessionLink, SessionRecord, SessionRow,
    StartAnotherSession, StartSession,
};
use store::{AttachmentState, Holder, KeptAttachment};

use super::linking::GROUP;
use crate::daemon::Fleet;

const AGENT_START_UNPLACED: &str = "fleet.agent_start_unplaced";
const AGENT_START_EMPTY: &str = "fleet.agent_start_empty";

/// On the new Session: its first message, which the person has not sent. The target is its words.
pub(crate) const UNSENT: &str = "unsent";
/// The id of the waiting item an `unsent` attachment is read as, followed by the Session's.
pub(crate) const UNSENT_ITEM: &str = "unsent:";
/// On the new Session, naming the one whose agent started it; and on that one, naming the new.
const STARTED_BY: &str = "started_by";
const STARTED_TO: &str = "started_to";

/// The words a call must carry, trimmed.
pub(super) struct Words {
    pub(super) title: String,
    pub(super) message: String,
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
    /// The hosted Session whose process holds this connection, or a 422 that says the call places none.
    fn agent_placed(&self, caller: Option<&Caller>) -> Result<String, Refusal> {
        caller.and_then(|caller| self.session_holding(caller)).ok_or_else(|| {
            self.hosted_refusal(
                AGENT_START_UNPLACED,
                "only a Job's own agent starts or forks a Job, and this call comes from none",
            )
        })
    }

    fn agent_words(&self, title: &str, message: &str) -> Result<Words, Refusal> {
        let (title, message) = (title.trim(), message.trim());
        if title.is_empty() || message.is_empty() {
            return Err(self.hosted_refusal(
                AGENT_START_EMPTY,
                "a Job an agent starts needs a title and a first message",
            ));
        }
        Ok(Words { title: title.to_string(), message: message.to_string() })
    }

    /// `start_another_session`.
    pub(crate) async fn another_session_started(
        self: &Arc<Self>,
        caller: Option<Caller>,
        start: StartAnotherSession,
    ) -> Result<SessionHeld, Refusal> {
        let me = self.agent_placed(caller.as_ref())?;
        let mut words = self.agent_words(&start.title, &start.message)?;
        let mut manifest = start.manifest_id;
        // A Session it waits on is checked before anything is made, so a refused wait leaves no Session behind.
        let after = start.after;
        if let Some(after) = &after {
            self.after_checked("", after).await?;
        }
        // A group of the caller's plan: the Session is the caller's own repository and the group's work.
        let group = match start.group.as_deref().map(str::trim).filter(|one| !one.is_empty()) {
            Some(group) => {
                let (_, hosting) = self.session_and_hosting(&me).await?;
                manifest = ManifestId::carried(&hosting.manifest_id);
                let (title, brief) = self.group_brief(&me, group).await?;
                words = Words { title, message: format!("{brief}\n\n{}", words.message) };
                Some(group.to_string())
            }
            None => None,
        };
        let made = self
            .start_session(StartSession {
                manifest_id: manifest,
                title: Some(words.title.clone()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
            })
            .await?;
        let brief = words.message.clone();
        // The brief is read now, so the proposal is in the dispatcher before the person sends it.
        if let Ok((session, hosting)) = self.session_and_hosting(made.id.as_str()).await {
            self.reading_begun(&session, &hosting, &brief);
        }
        let held = self.held_for_person(&me, made, words, false, group.as_deref(), after.is_none()).await?;
        // A Session that waits holds its first message until the condition is met, and then for the person.
        if let Some(after) = after {
            let told = after.session.as_str() != me;
            if let Err(refusal) = self.wait_begun(held.session_id.as_str(), after, brief, true, ipc::WaitBy::Agent, told).await {
                let _ = self.close_session(ipc::CloseSession { session_id: held.session_id.clone() }).await;
                return Err(refusal);
            }
        }
        Ok(held)
    }

    /// `fork_this_session`.
    pub(crate) async fn this_session_forked(
        &self,
        caller: Option<Caller>,
        fork: ForkThisSession,
    ) -> Result<SessionHeld, Refusal> {
        let me = self.agent_placed(caller.as_ref())?;
        let words = self.agent_words(&fork.title, &fork.message)?;
        let (_, hosting) = self.session_and_hosting(&me).await?;
        // A live conversation is only forked at a message, so no `at` is the latest one written.
        let at = match fork.at {
            Some(at) => at,
            None => self.latest_fork_point(&me).await?,
        };
        let made = self
            .start_session(StartSession {
                manifest_id: ManifestId::carried(&hosting.manifest_id),
                title: Some(words.title.clone()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: Some(ForkFrom {
                    session_id: ipc::SessionId::carried(&me),
                    at: Some(at),
                }),
                request: None,
                after: None,
            })
            .await?;
        self.held_for_person(&me, made, words, true, None, true).await
    }

    /// Keep the first message unsent, link the two ledgers, write the callout in the caller's thread
    /// and publish both. **A fork's links are the fork's own** (`forked_from`, `forked_to`), written
    /// when it was made.
    pub(super) async fn held_for_person(
        &self,
        caller: &str,
        made: SessionRecord,
        words: Words,
        fork: bool,
        group: Option<&str>,
        unsent: bool,
    ) -> Result<SessionHeld, Refusal> {
        let new = made.id.as_str().to_string();
        let manifest = made
            .manifest_id
            .as_ref()
            .map(|id| id.as_str().to_string())
            .unwrap_or_default();
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            // The group a linked Session was handed rides on both link rows, so each ledger serves it.
            let linked: BTreeMap<String, String> =
                group.map(|one| BTreeMap::from([(GROUP.to_string(), one.to_string())])).unwrap_or_default();
            let mut keep = |holder: &str, kind: &str, target: &str, state: AttachmentState, detail: BTreeMap<String, String>| {
                store
                    .attach(
                        &KeptAttachment {
                            holder: Holder::session(holder),
                            kind: kind.to_string(),
                            manifest_id: manifest.clone(),
                            target: target.to_string(),
                            state,
                            detail,
                            since: now.clone(),
                            changed_at: now.clone(),
                        },
                        false,
                    )
                    .map_err(|why| self.ledger_fault(why))
            };
            if !fork {
                // `spent`, as a fork's links are: the link is a fact that stays.
                keep(&new, STARTED_BY, caller, AttachmentState::Spent, linked.clone())?;
                keep(caller, STARTED_TO, &new, AttachmentState::Spent, linked)?;
            }
            if unsent {
                keep(&new, UNSENT, &words.message, AttachmentState::Standing, BTreeMap::new())?;
            }
        }
        self.row_put(
            caller,
            SessionRow::Started {
                id: self.row_id(caller),
                at: self.instant(),
                session: SessionLink { id: made.id.clone(), title: words.title.clone() },
                fork,
            },
        )
        .await;
        let named = self.store().lock().await.session(caller).ok().flatten();
        self.session_spawned(caller, named.map(|one| (one.title, one.manifest_id)).unwrap_or_default(), &words.title, fork);
        let _ = self.published_hosted(caller).await;
        self.published_hosted(&new).await?;
        Ok(SessionHeld { session_id: made.id, title: words.title, fork })
    }

    /// The person sent the Session a message, so the first one an agent held for him is sent: the
    /// attachment is spent and the waiting item it was read as goes with it. Nothing, where none was held.
    pub(crate) async fn first_message_sent(&self, session: &str) {
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let Ok(held) = store.attachments_of(&Holder::session(session)) else {
            return;
        };
        for one in held
            .into_iter()
            .filter(|one| one.kind == UNSENT && one.state == AttachmentState::Standing)
        {
            let _ = store.settle(
                &one.holder,
                UNSENT,
                &one.manifest_id,
                &one.target,
                AttachmentState::Spent,
                &now,
            );
        }
    }
}
