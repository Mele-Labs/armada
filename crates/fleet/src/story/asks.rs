//! An ask opened from a sorted message, and what he changes: a message's kind, an ask's state, kind, title and features.
//! `docs/concepts/job-story.md`, *An ask*.
//!
//! **An ask opens building, in the Job it was said in**: he said it to the Job that is working. Fleet moves it from
//! there as it sees the work (`super::progress`), and he moves it to the backlog or drops it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Ask, AskKind, AskState, AskWords, ChangeAsk, Event, Instant, MessageKind, MessageSorting, SessionId, SetMessageKind, SortedBy, StoryChanged, WireError};
use store::{ask_state, message_kind, KeptAsk, KeptMessage, Store};

use super::sorting::{self, Sorted};
use crate::daemon::Fleet;

pub(super) const NO_SUCH_MESSAGE: &str = "fleet.no_such_message";
pub(super) const NO_SUCH_ASK: &str = "fleet.no_such_ask";
const ASK_NEEDED: &str = "fleet.ask_needed";
const ASK_HAS_WORDS: &str = "fleet.ask_has_words";
const ASK_STATE_NOT_SET: &str = "fleet.ask_state_not_set";
const ASK_TITLE_BLANK: &str = "fleet.ask_title_blank";

pub(super) fn state_word(state: AskState) -> &'static str {
    match state {
        AskState::Backlog => ask_state::BACKLOG,
        AskState::Building => ask_state::BUILDING,
        AskState::Built => ask_state::BUILT,
        AskState::Handed => ask_state::HANDED,
        AskState::Dropped => ask_state::DROPPED,
    }
}

pub(super) fn state_of(word: &str) -> Option<AskState> {
    Some(match word {
        ask_state::BACKLOG => AskState::Backlog,
        ask_state::BUILDING => AskState::Building,
        ask_state::BUILT => AskState::Built,
        ask_state::HANDED => AskState::Handed,
        ask_state::DROPPED => AskState::Dropped,
        _ => return None,
    })
}

pub(super) fn message_kind_of(word: &str) -> Option<MessageKind> {
    Some(match word {
        message_kind::NEW_ASK => MessageKind::NewAsk,
        message_kind::REVISION => MessageKind::Revision,
        message_kind::ANSWER => MessageKind::Answer,
        message_kind::TALK => MessageKind::Talk,
        _ => return None,
    })
}

fn sorter_of(word: &str) -> SortedBy {
    match word {
        store::sorted_by::OWNER => SortedBy::Owner,
        _ => SortedBy::Fleet,
    }
}

/// The ask as the wire carries it, read whole from the store. `None` for one this build cannot read.
pub(super) fn ask_wire(store: &Store, kept: &KeptAsk) -> Result<Option<Ask>, store::WriteError> {
    let (Some(kind), Some(state)) = (sorting::ask_kind_of(&kept.kind), state_of(&kept.state)) else { return Ok(None) };
    let words = store.messages_of_ask(&kept.ask_id)?;
    let first = words.iter().find(|one| one.kind == message_kind::NEW_ASK);
    let said = words
        .iter()
        .filter_map(|one| {
            Some(AskWords {
                session_id: SessionId::carried(&one.session_id),
                row_id: one.row_id.clone(),
                kind: message_kind_of(&one.kind).filter(|kind| matches!(kind, MessageKind::Revision | MessageKind::Answer))?,
                words: one.words.clone(),
                at: Instant::carried(&one.at),
            })
        })
        .collect();
    Ok(Some(Ask {
        ask_id: kept.ask_id.clone(),
        kind,
        title: kept.title.clone(),
        first_words: first.map(|one| one.words.clone()).unwrap_or_else(|| kept.title.clone()),
        first_row: first.map(|one| one.row_id.clone()).unwrap_or_default(),
        said_in: SessionId::carried(&kept.session_id),
        building: kept.building.as_deref().map(SessionId::carried),
        features: store.features_of_ask(&kept.ask_id)?,
        state,
        state_at: Instant::carried(&kept.state_at),
        opened_at: Instant::carried(&kept.opened_at),
        said,
    }))
}

fn sorting_wire(kept: &KeptMessage, ask: Option<&KeptAsk>) -> Option<MessageSorting> {
    let kind = message_kind_of(&kept.kind)?;
    let opened = ask.filter(|_| kind == MessageKind::NewAsk);
    Some(MessageSorting {
        session_id: SessionId::carried(&kept.session_id),
        row_id: kept.row_id.clone(),
        kind,
        ask_kind: opened.and_then(|ask| sorting::ask_kind_of(&ask.kind)),
        ask_id: kept.ask_id.clone(),
        title: opened.map(|ask| ask.title.clone()),
        sorted_by: sorter_of(&kept.sorted_by),
        sorted_at: Instant::carried(&kept.sorted_at),
    })
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
    pub(super) fn story_not_found(&self, code: &'static str, what: String) -> Refusal {
        Refusal::NotFound(WireError::raised(code, what, self.run_id()))
    }

    pub(crate) fn story_changed(&self, sessions: &[&str]) {
        let mut told: Vec<&str> = Vec::new();
        for session in sessions {
            if !told.contains(session) {
                told.push(session);
                self.publish(Event::SessionStory(StoryChanged { session_id: SessionId::carried(*session) }));
            }
        }
    }

    /// What the model made of a message, kept as `by` sorted it. **A read of the model is kept only while its message is still
    /// waiting on it**: one he sorted meanwhile is his, and opens no ask.
    pub(super) async fn sorting_kept(&self, message: &KeptMessage, sorted: Sorted, by: &str) -> Result<(), Refusal> {
        let (_, hosting) = self.session_and_hosting(&message.session_id).await?;
        let now = self.now().as_str().to_string();
        let mut kept = KeptMessage {
            kind: sorting::kind_word(sorted.kind).to_string(),
            ask_id: sorted.ask.clone(),
            sorted_by: by.to_string(),
            sorted_at: now.clone(),
            ..message.clone()
        };
        let opened = (sorted.kind == MessageKind::NewAsk).then(|| KeptAsk {
            ask_id: self.mint().ulid().as_str().to_lowercase(),
            session_id: message.session_id.clone(),
            manifest_id: Some(hosting.manifest_id.clone()),
            project: hosting.project.clone(),
            kind: sorting::ask_kind_word(sorted.ask_kind.unwrap_or(AskKind::ChangeRequest)).to_string(),
            title: sorted.title.clone().unwrap_or_else(|| sorting::title_from(&message.words)),
            state: ask_state::BUILDING.to_string(),
            building: Some(message.session_id.clone()),
            opened_at: message.at.clone(),
            state_at: now,
        });
        kept.ask_id = opened.as_ref().map(|ask| ask.ask_id.clone()).or(kept.ask_id);
        {
            let mut store = self.store().lock().await;
            let taken = match by == store::sorted_by::FLEET {
                true => store.keep_message_read(&kept),
                false => store.keep_message(&kept).map(|_| true),
            }
            .map_err(|why| self.ledger_fault(why))?;
            if !taken {
                return Ok(());
            }
            if let Some(ask) = &opened {
                store.keep_ask(ask).map_err(|why| self.ledger_fault(why))?;
            }
        }
        self.story_changed(&[&message.session_id]);
        Ok(())
    }

    /// `list_message_kinds`.
    pub(crate) async fn message_kinds_listed(&self, session: &SessionId) -> Result<ipc::MessageSortings, Refusal> {
        let id = session.as_str();
        let store = self.store().lock().await;
        if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Err(self.story_not_found(super::NO_SUCH_SESSION, format!("no Job Fleet hosts is named {id}")));
        }
        let mut messages = Vec::new();
        for kept in store.messages_sorted_in(id).map_err(|why| self.ledger_fault(why))? {
            let ask = match &kept.ask_id {
                Some(ask) => store.ask(ask).map_err(|why| self.ledger_fault(why))?,
                None => None,
            };
            messages.extend(sorting_wire(&kept, ask.as_ref()));
        }
        Ok(ipc::MessageSortings { session_id: session.clone(), messages })
    }

    /// `set_message_kind`: his correction, kept, and the ask it opens, joins or leaves.
    pub(crate) async fn message_kind_set(self: &Arc<Self>, set: SetMessageKind) -> Result<MessageSorting, Refusal> {
        let id = set.session_id.as_str().to_string();
        self.session_and_hosting(&id).await?;
        let row = self.rows_of(&id).await?.into_iter().find_map(|row| match row {
            ipc::SessionRow::Message { id: row_id, at, from: ipc::SessionVoice::You, text, .. } if row_id == set.row_id => Some((at, text)),
            _ => None,
        });
        let Some((at, text)) = row else {
            return Err(self.story_not_found(NO_SUCH_MESSAGE, format!("job {id} holds no message of his named {}", set.row_id)));
        };
        let held = {
            let store = self.store().lock().await;
            let held = store.message_sorted(&id, &set.row_id).map_err(|why| self.ledger_fault(why))?;
            let asks = store.asks_of_session(&id).map_err(|why| self.ledger_fault(why))?;
            (held, asks)
        };
        let (kept, asks) = held;
        let words = kept.as_ref().map(|one| one.words.clone()).unwrap_or_else(|| text.trim().to_string());
        let message = KeptMessage {
            session_id: id.clone(),
            row_id: set.row_id.clone(),
            kind: String::new(),
            ask_id: None,
            words,
            at: kept.as_ref().map(|one| one.at.clone()).unwrap_or_else(|| at.as_str().to_string()),
            sorted_by: store::sorted_by::OWNER.to_string(),
            sorted_at: String::new(),
        };
        let opened_before = kept.as_ref().filter(|one| one.kind == message_kind::NEW_ASK).and_then(|one| one.ask_id.clone());
        let joined = match set.kind {
            MessageKind::Revision | MessageKind::Answer => {
                let Some(ask) = set.ask_id.as_deref().filter(|ask| !ask.is_empty()) else {
                    return Err(Refusal::Unacceptable(WireError::raised(ASK_NEEDED, "a revision or an answer joins an ask, and names it with `ask_id`", self.run_id())));
                };
                if !asks.iter().any(|one| one.ask_id == ask) {
                    return Err(self.story_not_found(NO_SUCH_ASK, format!("job {id} holds no ask named {ask}")));
                }
                Some(ask.to_string())
            }
            _ => None,
        };
        // A message that opened an ask and is now something else takes the ask with it, unless the ask has more words than its first.
        let kind_kept_ask = set.kind == MessageKind::NewAsk && opened_before.is_some();
        if let Some(ask) = opened_before.as_deref().filter(|_| !kind_kept_ask) {
            let others = self.store().lock().await.messages_of_ask(ask).map_err(|why| self.ledger_fault(why))?;
            if others.iter().any(|one| one.row_id != set.row_id) {
                return Err(Refusal::IllegalMove(WireError::raised(
                    ASK_HAS_WORDS,
                    "the ask this message opened holds revisions or answers, so it cannot become something else",
                    self.run_id(),
                )));
            }
            self.store().lock().await.remove_ask(ask).map_err(|why| self.ledger_fault(why))?;
        }
        if kind_kept_ask {
            // The same ask, so only its kind moves, and only where he named one.
            let ask = opened_before.clone().unwrap_or_default();
            let mut store = self.store().lock().await;
            if let (Some(mut one), Some(kind)) = (store.ask(&ask).map_err(|why| self.ledger_fault(why))?, set.ask_kind) {
                one.kind = sorting::ask_kind_word(kind).to_string();
                store.keep_ask(&one).map_err(|why| self.ledger_fault(why))?;
            }
            let kept = KeptMessage { kind: message_kind::NEW_ASK.to_string(), ask_id: Some(ask), sorted_at: self.now().as_str().to_string(), ..message };
            store.keep_message(&kept).map_err(|why| self.ledger_fault(why))?;
        } else {
            let sorted = Sorted { kind: set.kind, ask_kind: set.ask_kind.or(Some(AskKind::ChangeRequest)).filter(|_| set.kind == MessageKind::NewAsk), ask: joined, title: None };
            self.sorting_kept(&message, sorted, store::sorted_by::OWNER).await?;
        }
        self.story_changed(&[&id]);
        let store = self.store().lock().await;
        let kept = store.message_sorted(&id, &set.row_id).map_err(|why| self.ledger_fault(why))?;
        let ask = match kept.as_ref().and_then(|one| one.ask_id.as_deref()) {
            Some(ask) => store.ask(ask).map_err(|why| self.ledger_fault(why))?,
            None => None,
        };
        kept.as_ref().and_then(|kept| sorting_wire(kept, ask.as_ref())).ok_or_else(|| self.story_not_found(NO_SUCH_MESSAGE, format!("job {id} holds no message of his named {}", set.row_id)))
    }

    /// `change_ask`.
    pub(crate) async fn ask_changed(&self, change: ChangeAsk) -> Result<Ask, Refusal> {
        if let Some(state) = change.state {
            if !matches!(state, AskState::Backlog | AskState::Dropped) {
                return Err(Refusal::Unacceptable(WireError::raised(ASK_STATE_NOT_SET, "an ask is moved to the backlog or dropped; Fleet moves the rest", self.run_id())));
            }
        }
        let title = change.title.as_deref().map(str::trim);
        if title.is_some_and(str::is_empty) {
            return Err(Refusal::Unacceptable(WireError::raised(ASK_TITLE_BLANK, "an ask needs a title", self.run_id())));
        }
        let now = self.now().as_str().to_string();
        let (wire, jobs) = {
            let mut store = self.store().lock().await;
            let Some(mut ask) = store.ask(&change.ask_id).map_err(|why| self.ledger_fault(why))? else {
                return Err(self.story_not_found(NO_SUCH_ASK, format!("no ask is named {}", change.ask_id)));
            };
            let jobs = [Some(ask.session_id.clone()), ask.building.clone()];
            if let Some(kind) = change.kind {
                ask.kind = sorting::ask_kind_word(kind).to_string();
            }
            if let Some(title) = title {
                ask.title = sorting::cut(title, sorting::TITLE_CHARS);
            }
            if let Some(state) = change.state {
                ask.state = state_word(state).to_string();
                ask.state_at = now;
                if state == AskState::Backlog {
                    ask.building = None;
                }
            }
            store.keep_ask(&ask).map_err(|why| self.ledger_fault(why))?;
            if let Some(slugs) = &change.features {
                let slugs: Vec<String> = slugs.iter().map(|slug| slug.trim().to_string()).filter(|slug| !slug.is_empty()).collect();
                store.keep_ask_features(&ask.ask_id, &slugs).map_err(|why| self.ledger_fault(why))?;
            }
            let wire = ask_wire(&store, &ask).map_err(|why| self.ledger_fault(why))?;
            (wire, jobs)
        };
        let jobs: Vec<String> = jobs.into_iter().flatten().collect();
        self.story_changed(&jobs.iter().map(String::as_str).collect::<Vec<_>>());
        wire.ok_or_else(|| self.story_not_found(NO_SUCH_ASK, format!("no ask is named {}", change.ask_id)))
    }

    /// `list_job_asks`.
    pub(crate) async fn job_asks_listed(&self, session: &SessionId) -> Result<ipc::JobAsks, Refusal> {
        let id = session.as_str();
        let store = self.store().lock().await;
        if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Err(self.story_not_found(super::NO_SUCH_SESSION, format!("no Job Fleet hosts is named {id}")));
        }
        let mut asks = Vec::new();
        for kept in store.asks_of_session(id).map_err(|why| self.ledger_fault(why))? {
            asks.extend(ask_wire(&store, &kept).map_err(|why| self.ledger_fault(why))?);
        }
        Ok(ipc::JobAsks { session_id: session.clone(), asks })
    }

    /// `list_feature_asks`.
    pub(crate) async fn feature_asks_listed(&self, project: &str, feature: Option<&str>) -> Result<ipc::FeatureAsks, Refusal> {
        let store = self.store().lock().await;
        let feature = feature.map(str::trim).filter(|slug| !slug.is_empty());
        let mut asks = Vec::new();
        for kept in store.asks_of_project(project, feature).map_err(|why| self.ledger_fault(why))? {
            asks.extend(ask_wire(&store, &kept).map_err(|why| self.ledger_fault(why))?);
        }
        Ok(ipc::FeatureAsks { project: project.to_string(), feature: feature.map(String::from), asks })
    }
}
