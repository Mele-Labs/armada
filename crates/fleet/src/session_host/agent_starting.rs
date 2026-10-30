//! A Session's agent starts a Session, or forks its own conversation into one, and the new one waits
//! for the person. Since 23.80. `docs/concepts/session.md`, *A Session an agent started*.
//!
//! **The new Session is created and listed at once, held on its start card, and nothing runs.** The
//! agent's first message is its held first turn (`start_card.rs`), which the person edits in his
//! message box and sends with Start; until then it waits in his list as a `first_message` item.
//!
//! **The caller is placed by its connection and never named**, so a fork can only be of the
//! caller's own conversation.
//!
//! **Until he presses Start the caller's own list waits on it too**, as `held_child` (`Press Start on
//! "<title>"`), read off the child each time, so the caller's agent lists nothing of its own for it.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, HostedSessions, Refusal};
use ipc::{
    ForkFrom, ForkThisSession, Instant, ManifestId, SessionHeld, SessionLink, SessionRecord, SessionRow,
    StartAnotherSession, StartSession, WaitingAct, WaitingActKind, WaitingItem, WaitingSource,
};
use store::{AttachmentState, Holder, KeptAttachment, Store};

use super::linking::GROUP;
use crate::daemon::Fleet;

const AGENT_START_UNPLACED: &str = "fleet.agent_start_unplaced";
const AGENT_START_EMPTY: &str = "fleet.agent_start_empty";

/// On the new Session, naming the one whose agent started it; and on that one, naming the new.
pub(crate) const STARTED_BY: &str = "started_by";
const STARTED_TO: &str = "started_to";

/// A Job this one's agent started that still waits on its start card.
#[derive(Clone)]
pub(crate) struct HeldChild {
    pub id: String,
    pub title: String,
    /// When the link was written: the item waits from then.
    pub since: String,
}

/// The Jobs `parent`'s agent started that are live and held on their start cards. A press queued
/// while the proposer reads leaves one held, so it is still listed.
pub(crate) fn held_children(store: &Store, parent: &str) -> Result<Vec<HeldChild>, store::WriteError> {
    let mut held = Vec::new();
    for link in store.attachments_of(&Holder::session(parent))?.into_iter().filter(|one| one.kind == STARTED_TO) {
        let Some(child) = store.session(&link.target)? else { continue };
        if child.state == store::SessionState::Ended || !store.hosting(&child.id)?.is_some_and(|one| one.start_held) {
            continue;
        }
        held.push(HeldChild { title: child.title.unwrap_or_else(|| child.id.clone()), id: child.id, since: link.since });
    }
    Ok(held)
}

/// The parent's item for a held child: pressing it opens the child, whose Start settles it.
pub(crate) fn held_child_item(child: &HeldChild) -> WaitingItem {
    WaitingItem {
        id: format!("start:{}", child.id),
        text: format!("Press Start on \"{}\"", child.title),
        since: Instant::carried(&child.since),
        source: WaitingSource::HeldChild,
        act: Some(WaitingAct { kind: WaitingActKind::Message, target: child.id.clone() }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
    }
}

/// An agent's own item that sends the person to one of `held`, which the held child's item already says.
pub(crate) fn repeats_held_child(item: &WaitingItem, held: &[HeldChild]) -> bool {
    item.source == WaitingSource::Agent
        && item.act.as_ref().is_some_and(|act| act.kind == WaitingActKind::Message && held.iter().any(|child| child.id == act.target))
}

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
        self.another_session_started_by(me, start).await
    }

    /// [`Self::another_session_started`] for a Job named, which is the parent. **The door a rule of
    /// the Job goes through**: no connection places it, and what it makes is held for the person to
    /// start as anything an agent starts is.
    pub(crate) async fn another_session_started_by(
        self: &Arc<Self>,
        me: String,
        start: StartAnotherSession,
    ) -> Result<SessionHeld, Refusal> {
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
                // A Session that waits is held once its wait is met.
                held: after.is_none(),
                project_rows: Vec::new(),
            })
            .await?;
        let brief = words.message.clone();
        // The agent's title, a plan group's too, is a hint the proposer names the Job over.
        self.hosts().of(made.id.as_str()).state().proposal.hint = Some(words.title.clone());
        // The brief is read now, so the proposal is in the dispatcher before the person sends it.
        if let Ok((session, hosting)) = self.session_and_hosting(made.id.as_str()).await {
            self.reading_begun(&session, &hosting, &brief);
        }
        let new = made.id.as_str().to_string();
        if after.is_none() {
            self.held_with(&new, &brief).await?;
        }
        let held = self.held_for_person(&me, made, words, false, group.as_deref()).await?;
        // A Session that waits holds its first message until the condition is met, and then for the person.
        if let Some(after) = after {
            let told = after.session.as_str() != me;
            if let Err(refusal) = self.wait_begun(held.session_id.as_str(), after, brief, true, ipc::WaitBy::Agent, told).await {
                let _ = self.closed_and_parked(ipc::CloseSession { session_id: held.session_id.clone() }).await;
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
                held: true,
                project_rows: Vec::new(),
            })
            .await?;
        // A fork is not read, so its title alone is asked for when Start sends it (`fork_named`).
        {
            let runtime = self.hosts().of(made.id.as_str());
            let mut state = runtime.state();
            state.proposal.hint = Some(words.title.clone());
            state.proposal.stand_in = Some(words.title.clone());
        }
        self.held_with(made.id.as_str(), &words.message).await?;
        self.held_for_person(&me, made, words, true, None).await
    }

    /// Link the two ledgers, write the callout in the caller's thread and publish both. **A fork's
    /// links are the fork's own** (`forked_from`, `forked_to`), written when it was made.
    pub(super) async fn held_for_person(
        &self,
        caller: &str,
        made: SessionRecord,
        words: Words,
        fork: bool,
        group: Option<&str>,
    ) -> Result<SessionHeld, Refusal> {
        let new = made.id.as_str().to_string();
        let manifest = made
            .manifest_id
            .as_ref()
            .map(|id| id.as_str().to_string())
            .unwrap_or_default();
        if !fork {
            self.started_linked(caller, &new, &manifest, group).await?;
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

    /// `started_by` on `new` and `started_to` on `caller`, which Bridge nests `new` under. **Written
    /// alone** where the caller's thread already names the new Session, as a dispute's row does.
    pub(super) async fn started_linked(&self, caller: &str, new: &str, manifest: &str, group: Option<&str>) -> Result<(), Refusal> {
        let now = self.now().as_str().to_string();
        // The group a linked Session was handed rides on both link rows, so each ledger serves it.
        let linked: BTreeMap<String, String> =
            group.map(|one| BTreeMap::from([(GROUP.to_string(), one.to_string())])).unwrap_or_default();
        let mut store = self.store().lock().await;
        // `spent`, as a fork's links are: the link is a fact that stays.
        for (holder, kind, target) in [(new, STARTED_BY, caller), (caller, STARTED_TO, new)] {
            let link = attachment(holder, kind, manifest, target, AttachmentState::Spent, linked.clone(), &now);
            store.attach(&link, false).map_err(|why| self.ledger_fault(why))?;
        }
        Ok(())
    }
}

fn attachment(
    holder: &str,
    kind: &str,
    manifest: &str,
    target: &str,
    state: AttachmentState,
    detail: BTreeMap<String, String>,
    now: &str,
) -> KeptAttachment {
    KeptAttachment {
        holder: Holder::session(holder),
        kind: kind.to_string(),
        manifest_id: manifest.to_string(),
        target: target.to_string(),
        state,
        detail,
        since: now.to_string(),
        changed_at: now.to_string(),
    }
}
