//! The Operator as one agent: a hosted session per Mac with Fleet's tools, which puts cards in front
//! of the person and which Fleet tells what it used to rule on. `crates/ipc/operations/raise_card.toml`.
//!
//! **It has no repository, no slot and no Manifest.** It starts in a folder of its own under the
//! Operator's home, as a Mod Job starts in the mods folder where Armada's own repository is not served,
//! so the door answers it without a Manifest and offers it the reads and acts that name their
//! repository themselves. While `features.operator` is on and none runs, the next tick starts one;
//! turning the switch off ends it. A hand on at the context limit keeps `operator` on the successor.

use std::path::PathBuf;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings as keys;
use ipc::operator_page::OperatorFeedKind;
use ipc::{CloseSession, ManifestId, Notice, SendSessionMessage, SessionId, SessionTag, SessionUpload, SessionVoice, StartSession};

use crate::daemon::Fleet;
use super::arriving::{Arrival, Arrived};
use crate::session_host::Marked;

/// What the Operator's session is called where a title is drawn.
pub(crate) const TITLE: &str = "Operator";

/// Its first turn. The rules are in its system prompt, so this only wakes it. **It watches from now**: what waited on him
/// before it came on stays on each Job's own Needs you, so a fresh session does not card the backlog.
const BEGIN: &str = "You are up. Your watch starts now: what already waits on him stays on each Job's own Needs you, \
so put no card up for it. Put a card in front of him only for something that arrives from here on and needs his decision.";

/// The refusal for an act only the Operator's session may make.
pub(crate) const NOT_THE_OPERATOR: &str = "fleet.not_the_operator";

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
    /// Whether `id` is the Operator's own session, as its placed process says.
    pub(crate) fn session_is_the_operator(&self, id: &str) -> bool {
        self.hosts().of(id).state().operator
    }

    /// The folder the Operator's session runs in, beside its journal.
    pub(crate) fn operator_dir(&self) -> PathBuf {
        let journal = self.journal_dir();
        journal.parent().map(|home| home.join("session")).unwrap_or_else(|| journal.join("session"))
    }

    /// The live Operator session's id, where one runs.
    pub(crate) async fn operator_session_id(&self) -> Option<String> {
        self.store().lock().await.live_operator_hosting().ok().flatten().map(|hosting| hosting.session_id)
    }

    /// Refuse a caller that is not the Operator's session, and name the session otherwise.
    pub(crate) fn operator_called(&self, caller: Option<api::Caller>) -> Result<String, Refusal> {
        caller
            .and_then(|caller| self.session_holding(&caller))
            .filter(|id| self.session_is_the_operator(id))
            .ok_or_else(|| self.hosted_refusal(NOT_THE_OPERATOR, "only the Operator's own session may call this"))
    }

    /// Refuse a call through the door that is not the Operator's session, and let through a call that arrived by no door.
    /// **The door is the only thing that places a caller** (`api::acting::asking`), so a call with none is the person's own
    /// from Bridge, and `api::callers::THE_PERSONS_OWN` has already refused it to a process held inside a Job.
    pub(crate) fn owner_or_operator_called(&self, caller: Option<api::Caller>) -> Result<(), Refusal> {
        match caller {
            None => Ok(()),
            Some(_) => self
                .operator_called(caller)
                .map(drop)
                .map_err(|_| self.hosted_refusal(NOT_THE_OPERATOR, "only Bridge or the Operator's own session may call this")),
        }
    }

    /// Once a tick: start the Operator's session while the switch is on and none runs, and end it once
    /// the switch is off. While it is on, a picked card nothing settled in time comes back to the front.
    pub(crate) async fn operator_session_kept(self: &Arc<Self>) {
        let _one = self.operator_seat().session.lock().await;
        let live = self.operator_session_id().await;
        match (self.operator_on(), live) {
            (true, None) => {
                if let Err(why) = self.operator_session_started().await {
                    self.operator_warned("the Operator's session did not start", format!("{why:?}"));
                }
            }
            (false, Some(id)) => {
                if let Err(why) = self.closed_and_parked(CloseSession { session_id: ipc::SessionId::carried(&id) }).await {
                    self.operator_warned("the Operator's session did not end", format!("{why:?}"));
                }
            }
            _ => {}
        }
        if self.operator_on() {
            self.cards_stalled_returned().await;
        }
    }

    /// Start the Operator's session on the Operator model, and send its first turn.
    async fn operator_session_started(self: &Arc<Self>) -> Result<String, Refusal> {
        let dir = self.operator_dir();
        std::fs::create_dir_all(&dir).map_err(|why| self.hosted_refusal("fleet.operator_session_folder", &format!("the Operator's folder could not be made: {why}")))?;
        let model: String = self.machine_settings().now().get(keys::MODELS_OPERATOR);
        let start = StartSession {
            project: None,
            manifest_id: ManifestId::carried(""),
            title: Some(TITLE.to_string()),
            model: Some(model).filter(|model| !model.trim().is_empty()),
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
            workflow: None,
        };
        let id = self.started_hosted(start, Marked::Operator).await?.id.as_str().to_string();
        self.first_message_from(&id, SessionVoice::You, BEGIN).await?;
        Ok(id)
    }

    /// Fleet's word to the Operator's session, where one runs. `false` where none does, or it could not
    /// be told, and the caller goes on as it did before the session.
    pub(crate) async fn operator_told(self: &Arc<Self>, text: &str) -> bool {
        let Some(id) = self.operator_session_id().await else { return false };
        matches!(self.told_by_fleet(&id, text, Notice::Operator).await, Ok(true))
    }

    /// Tell the Operator's session of `arrival`, once. **`false` where no session runs, or it could not be told**,
    /// and the caller goes on as it did with the Operator off.
    pub(crate) async fn arrival_told(self: &Arc<Self>, arrival: &Arrival) -> bool {
        self.arrival_handed(arrival).await != Handed::NotRunning
    }

    /// [`arrival_told`](Self::arrival_told), saying whether the session was told now or had been before.
    pub(crate) async fn arrival_handed(self: &Arc<Self>, arrival: &Arrival) -> Handed {
        if self.operator_session_id().await.is_none() {
            return Handed::NotRunning;
        }
        let now = self.now();
        if !self.store().lock().await.keep_linked_told("operator", TOLD, &arrival.id, &now).unwrap_or(false) {
            return Handed::Before;
        }
        let repository = self.board_repository(&arrival.manifest_id).map(|name| format!(" in {name}")).unwrap_or_default();
        let text = format!(
            "{}{repository}: {}\n\n{}\n\nSettle it with your own tools, or put a card in front of him where it is his to decide.",
            arrival.arrived.said(),
            arrival.subject,
            arrival.words
        );
        if !self.operator_told(&text).await {
            return Handed::NotRunning;
        }
        let kind = if matches!(arrival.arrived, Arrived::Conflict | Arrived::Held) { OperatorFeedKind::Asked } else { OperatorFeedKind::Told };
        let session = Some(arrival.session_id.as_str()).filter(|id| !id.is_empty());
        self.feed_kept(kind, &arrival.subject, session, Some(arrival.arrived.said())).await;
        Handed::Now
    }

    /// The person's words to the Operator's session, as he wrote them, with the files and tags he sent. A refusal
    /// says it is not running.
    pub(crate) async fn operator_told_as_him(self: &Arc<Self>, text: &str, attachments: Vec<SessionUpload>, mentions: Vec<SessionTag>) -> Result<(), Refusal> {
        let Some(id) = self.operator_session_id().await else {
            return Err(self.hosted_refusal(NOT_RUNNING, "the Operator's session is not running, so nothing here reaches it"));
        };
        let sent = SendSessionMessage { session_id: SessionId::carried(&id), text: text.to_string(), attachments, mentions, sketches: Vec::new() };
        self.message_from(sent, SessionVoice::You).await
    }
}

/// How the session came to hear of an arrival.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handed {
    /// No session runs, or it could not be told.
    NotRunning,
    /// It was told now.
    Now,
    /// It had been told already.
    Before,
}

/// What `keep_linked_told` files an arrival told to the Operator's session under.
pub(super) const TOLD: &str = "told";

/// The Operator's session is not running.
pub(crate) const NOT_RUNNING: &str = "fleet.operator_session_not_running";
