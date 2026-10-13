//! `api::Conversations`, implemented over a real Fleet: a message taken, a
//! reply carried by whichever host Fleet holds, a conversation started over.
//! `#939`.
//!
//! **What a message resumes is decided here and in no host.** The stored
//! session is read; a new session is told Helm's brief before the message; and
//! a session the host could not find is forgotten and started again, once, with
//! the thread told why.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, DroneEvent, Speaker, Vcs, WorkProduct};
use api::{Conversations as Surface, ObservedHelm, Refusal, StoredFile};
use core_model::StepId;
use ipc::{
    AnswerHelmCall, AskHelm, AskingToRun, EventsSince, Freshness, HelmAsked, HelmBoxFacts,
    HelmCallsWaiting, HelmChangedCheckout, HelmConversation, HelmDebugInfo,
    HelmFresh, HelmMessage, HelmUnanswered, Instant, ManifestId, ReadThought,
    RunOrNot, Shown, TellSession, ThoughtReading, ThoughtSent, ThoughtTarget, TuneHelm, WireError,
};

use super::composing::Message;
use super::conversation::{Conversation, ConversationKey};
use super::hosting::{Carried, Carry, Heard};
use super::{recording, unanswered, worded, Authority};
use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::prompts::Prompts;
use crate::repositories::Served;

/// Start fresh, pressed while a reply is being written.
const HELM_STILL_REPLYING: &str = "fleet.helm_still_replying";
/// The stored session would not read or write.
const HELM_SESSION_UNREADABLE: &str = "fleet.helm_session_unreadable";

impl<H, V, W> Surface for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn observe_helm(&self, manifest_id: Option<ManifestId>) -> Result<ObservedHelm, Refusal> {
        let (served, key) = self.helm_of(manifest_id.as_ref())?;
        let conversation = self.helm().open(&key, served.records_root());
        let (live, history, skipped) = conversation.thread.watched();
        // Read after the subscription opens, so a change between the two reaches this viewer as
        // `facts` and is not lost.
        let facts = self.helm_facts(&key, &conversation).await?;
        Ok(ObservedHelm {
            manifest_id: on_the_wire(&served),
            replying: conversation.replying(),
            live,
            history,
            skipped,
            facts,
        })
    }

    async fn ask_helm(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        let (served, key) = self.helm_of(manifest_id.as_ref())?;
        let conversation = self.helm().open(&key, served.records_root());
        let message = self.helm_message(&key, asked).await?;
        let resumes = self
            .store()
            .lock()
            .await
            .helm_session(key.as_str())
            .map_err(|why| self.helm_fault(why))?
            .is_some();
        let answered = HelmConversation {
            manifest_id: on_the_wire(&served),
            replying: true,
            resumes,
        };
        conversation.taken();
        let fleet = Arc::clone(&self);
        tokio::spawn(async move {
            fleet.reply(served, key, conversation, message).await;
        });
        Ok(answered)
    }

    async fn tune_helm(
        &self,
        tuned: TuneHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmBoxFacts, Refusal> {
        self.helm_tuned(tuned, manifest_id).await
    }

    async fn get_helm_file(
        &self,
        manifest_id: Option<ManifestId>,
        file: String,
    ) -> Result<StoredFile, Refusal> {
        self.helm_file(manifest_id, file).await
    }

    /// **Taken at a moment, and never a second `observe_helm`.** Everything
    /// here is read in one pass: nothing is subscribed to and nothing follows.
    async fn get_helm_debug_info(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmDebugInfo, Refusal> {
        let (served, key) = self.helm_of(manifest_id.as_ref())?;
        let conversation = self.helm().open(&key, served.records_root());
        let (messages, dropped) = conversation.thread.taken();
        let (thread, cut) = recording::thread(&messages, dropped);
        let authority = self.helm_authority();
        let session = self
            .store()
            .lock()
            .await
            .helm_session(key.as_str())
            .map_err(|why| self.helm_fault(why))?;
        Ok(HelmDebugInfo {
            manifest_id: on_the_wire(&served),
            checkout: served.root().to_string(),
            authority: recording::authority_on_the_wire(authority),
            model: self.helm().host().model(),
            // The brief this session was sent, assembled the one way
            // `carrying` assembles it — never a second wording of it.
            brief: worded(&self.prompts(), authority, served.manifest(), None)
                .as_str()
                .to_string(),
            door: ipc::door::SERVER.to_string(),
            tools: recording::tools(authority),
            servers: recording::servers(&messages),
            session,
            thread,
            cut,
            polled: conversation.last_poll(),
            run_id: self.run_id().as_str().to_string(),
            protocol_id: ipc::ProtocolId::current(),
            at: Instant::from(&self.now()),
        })
    }

    async fn helm_polled(&self, manifest_id: ManifestId, counted: EventsSince) {
        let Ok((served, key)) = self.helm_of(Some(&manifest_id)) else {
            return;
        };
        self.helm()
            .open(&key, served.records_root())
            .polled(counted);
    }

    async fn ask_the_person(
        &self,
        asking: AskingToRun,
        manifest_id: Option<ManifestId>,
    ) -> Result<RunOrNot, Refusal> {
        self.helm_permission(asking, manifest_id).await
    }

    async fn ask_the_person_at(
        &self,
        asking: AskingToRun,
        manifest_id: Option<ManifestId>,
        caller: api::Caller,
    ) -> Result<RunOrNot, Refusal> {
        match self.session_holding(&caller) {
            Some(session) => self.session_permission(&session, asking).await,
            None => self.helm_permission(asking, manifest_id).await,
        }
    }

    async fn list_helm_calls(&self) -> Result<HelmCallsWaiting, Refusal> {
        Ok(self.helm_calls_waiting())
    }

    async fn answer_helm_call(&self, said: AnswerHelmCall) -> Result<HelmCallsWaiting, Refusal> {
        self.helm_call_answered(said)
    }

    async fn read_thought(&self, read: ReadThought) -> ThoughtReading {
        self.thought_read(read).await
    }

    async fn record_thought_sent(
        &self,
        sent: ThoughtSent,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        self.thought_sent(sent, manifest_id).await
    }

    async fn tell_session(
        self: Arc<Self>,
        tell: TellSession,
        manifest_id: Option<ManifestId>,
    ) -> Result<ThoughtTarget, Refusal> {
        self.session_told(tell, manifest_id).await
    }

    async fn start_helm_fresh(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        let (served, key) = self.helm_of(manifest_id.as_ref())?;
        let conversation = self.helm().open(&key, served.records_root());
        let still = || {
            Refusal::IllegalMove(WireError::raised(
                HELM_STILL_REPLYING,
                "Helm is still answering. Start fresh once the reply is in",
                self.run_id(),
            ))
        };
        if conversation.replying() {
            return Err(still());
        }
        let Ok(_turn) = conversation.turn.try_lock() else {
            return Err(still());
        };
        self.store()
            .lock()
            .await
            .forget_helm_session(key.as_str())
            .map_err(|why| self.helm_fault(why))?;
        conversation.thread.clear();
        // What its messages carried goes with them; the thread no longer names any of it.
        let _ = std::fs::remove_dir_all(key.uploads_in(&self.host().attachments_dir));
        Ok(HelmConversation {
            manifest_id: on_the_wire(&served),
            replying: false,
            resumes: false,
        })
    }
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
    pub(super) fn helm_of(&self, named: Option<&ManifestId>) -> Result<(Served, ConversationKey), Refusal> {
        let served = self.served_named(named)?;
        let key = ConversationKey::of_repository(served.manifest().id());
        Ok((served, key))
    }

    pub(super) fn helm_fault(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(
            HELM_SESSION_UNREADABLE,
            unanswered::stored_session_unreachable(why),
            self.run_id(),
        ))
    }

    /// Forget every Helm session past `settings.helm-session-retention-
    /// expiry`, on every reply rather than on a timer. **Best-effort**: a
    /// store fault sweeping old rows must not stop the reply it rides in on,
    /// so nothing here is surfaced past a swallow. `#943`.
    async fn swept_stale_helm_sessions(&self) {
        let now = self.now();
        let _ = self
            .store()
            .lock()
            .await
            .forget_stale_helm_sessions(&now, self.helm_session_retention());
    }

    /// One reply, start to finish. **Always answered**: whatever happens, the
    /// thread says what, and the conversation stops counting this message.
    async fn reply(
        &self,
        served: Served,
        key: ConversationKey,
        conversation: Arc<Conversation>,
        message: Message,
    ) {
        let _turn = conversation.turn.lock().await;
        self.swept_stale_helm_sessions().await;
        conversation.thread.say(HelmMessage::Asked(HelmAsked {
            ts: Instant::from(&self.now()),
            text: message.text.clone(),
            files: message.files.clone(),
            tags: message.tags.clone(),
        }));
        let heard = Rows {
            conversation: Arc::clone(&conversation),
            clock: Arc::clone(self.clock()),
            manifest_id: on_the_wire(&served),
            root: served.root().to_string(),
            events: self.events().clone(),
        };
        if let Err(why) = self
            .replied(&served, &key, &message, &heard)
            .await
        {
            conversation
                .thread
                .say(HelmMessage::Unanswered(HelmUnanswered {
                    ts: Instant::from(&self.now()),
                    why,
                }));
        }
        conversation.answered();
    }

    async fn replied(
        &self,
        served: &Served,
        key: &ConversationKey,
        message: &Message,
        heard: &Rows,
    ) -> Result<(), String> {
        let (model, effort) = self.helm_tuning(key, &heard.conversation).await?;
        let uploads = key.uploads_in(&self.host().attachments_dir);
        let readable = uploads
            .is_dir()
            .then(|| uploads.to_string_lossy().into_owned());
        let tuning = Tuning {
            model,
            effort,
            readable,
        };
        let stored = self
            .store()
            .lock()
            .await
            .helm_session(key.as_str())
            .map_err(unanswered::session_would_not_read)?;
        let host = self.helm().host();
        let prompts = self.prompts();
        let authority = self.helm_authority();
        let mut carried = host
            .carry(
                carrying(served, message, stored.clone(), &prompts, authority, &tuning),
                heard,
            )
            .await;
        if carried == Carried::NoSuchSession && stored.is_some() {
            self.store()
                .lock()
                .await
                .forget_helm_session(key.as_str())
                .map_err(unanswered::lost_session_not_forgotten)?;
            heard.conversation.thread.say(HelmMessage::Fresh(HelmFresh {
                ts: Instant::from(&self.now()),
                because: Freshness::SessionNotFound,
            }));
            carried = host
                .carry(
                    carrying(served, message, None, &prompts, authority, &tuning),
                    heard,
                )
                .await;
        }
        match carried {
            Carried::Answered { session } => self
                .store()
                .lock()
                .await
                .keep_helm_session(key.as_str(), &session, &self.now())
                .map_err(unanswered::session_not_kept),
            Carried::NoSuchSession => Err(unanswered::no_session_to_resume()),
            Carried::Failed { why } => Err(why),
        }
    }
}

/// What a reply runs on beyond the message: a person's choice, and where uploads are read.
struct Tuning {
    model: Option<String>,
    effort: Option<String>,
    readable: Option<String>,
}

/// A message as its session is told it: after Helm's brief on a new session,
/// alone on one that already heard the brief — and, on every turn, `where`'s
/// line ahead of what the person typed. `#1075`.
fn carrying(
    served: &Served,
    message: &Message,
    resuming: Option<String>,
    prompts: &Prompts,
    authority: Authority,
    tuning: &Tuning,
) -> Carry {
    let told = message.told();
    let turn = match resuming {
        Some(_) => told,
        None => format!(
            "{}\n\n{told}",
            worded(prompts, authority, served.manifest(), None).as_str()
        ),
    };
    Carry {
        directory: served.root().to_string(),
        turn,
        resuming,
        pictures: message.pictures.clone(),
        model: tuning.model.clone(),
        effort: tuning.effort.clone(),
        readable: tuning.readable.clone(),
    }
}

pub(super) fn on_the_wire(served: &Served) -> ManifestId {
    ManifestId::carried(served.manifest().id().as_str())
}

/// What a session says, as rows in its thread — and, where it wrote a file in
/// the checkout, as an event of Helm's own.
struct Rows {
    conversation: Arc<Conversation>,
    clock: Arc<dyn Clock>,
    manifest_id: ManifestId,
    root: String,
    events: api::Broadcaster,
}

impl Rows {
    /// `helm.changed_checkout`, where this call was one of the built-ins that
    /// edits a file. **A path and never the contents**: what was written is on
    /// the thread, and this stream is for a client that is not watching the
    /// dock. `docs/concepts/helm.md`, *Audit trail*.
    fn published_as_a_write(&self, event: &DroneEvent, at: &core_model::Timestamp) {
        let DroneEvent::Called { tool, detail, .. } = event else {
            return;
        };
        if !adapters::wrote_the_checkout(tool) {
            return;
        }
        let path = adapters::path_written(detail.whole().unwrap_or(detail.shown()));
        if path.is_empty() {
            return;
        }
        let inside = format!("{}/", self.root.trim_end_matches('/'));
        self.events
            .publish(ipc::Event::HelmChangedCheckout(HelmChangedCheckout {
                manifest_id: self.manifest_id.clone(),
                tool: tool.clone(),
                path: path.strip_prefix(&inside).unwrap_or(path).to_string(),
                at: Instant::from(at),
            }));
    }
}

impl Heard for Rows {
    fn commands(&self, names: Vec<String>) {
        super::facts::commands_changed(&self.conversation, names);
    }

    fn heard(&self, events: &[DroneEvent]) {
        let at = self.clock.now();
        for event in events {
            // **Prose that arrived on the session's input channel is never
            // Helm's reply.** A skill the session calls is injected as a user
            // turn carrying its whole body, and it reads as text exactly as
            // the brief coming back does; the thread draws every `said` row
            // as Helm's, so it is dropped here, where `Speaker` is still known.
            if matches!(
                event,
                DroneEvent::Said {
                    by: Speaker::Armada,
                    ..
                }
            ) {
                continue;
            }
            self.published_as_a_write(event, &at);
            // `seen` labels a row with a Drone's step, and a conversation has
            // none, so the label is taken off again.
            let mut row = crate::transcript::seen(&at, &StepId::new(""), event);
            row.step = None;
            if let Some(shown) = Shown::of(row) {
                self.conversation.thread.say(HelmMessage::Row(shown));
            }
        }
    }
}
