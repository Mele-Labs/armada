//! `api::Conversations`, implemented over a real Fleet: Helm's Session found, written to, ended
//! and recorded (`session.rs`), a Job's Commander's turn carried by the host. `#939`.
//!
//! **Helm is a hosted Session** and its messages are a Session's, so what a turn is made of is
//! `session_host`'s. What a Commander's turn resumes is decided here and in no host: the stored
//! session is read, and a session the host could not find is forgotten and started again, once,
//! with the thread told why.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Conversations as Surface, Refusal};
use ipc::{
    AnswerHelmCall, AskHelm, AskingToRun, EventsSince, HelmCallsWaiting, HelmDebugInfo,
    HelmSession, ManifestId, ReadThought, RunOrNot, SessionRecord, TellSession, ThoughtReading,
    ThoughtSent, ThoughtTarget, WireError,
};

use super::conversation::ConversationKey;
use super::hosting::{Carried, Carry, Heard};
use super::unanswered;
use crate::daemon::Fleet;

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
    async fn observe_commander(
        &self,
        job: api::Resolved,
    ) -> Result<api::ObservedCommander, Refusal> {
        self.commander_observed(job).await
    }

    async fn ask_the_owner(
        self: Arc<Self>,
        job: api::Resolved,
        asked: ipc::AskTheOwner,
    ) -> Result<ipc::CommanderCard, Refusal> {
        self.commander_card_raised(job, asked).await
    }

    async fn send_repair_drone(
        self: Arc<Self>,
        job: api::Resolved,
        sending: ipc::SendRepairDrone,
    ) -> Result<ipc::RepairSent, Refusal> {
        let refusal = Arc::clone(&self);
        self.repair_sent(&job.id().to_domain(), sending.broke)
            .await
            .map_err(|why| refusal.refusal(why))
    }

    async fn answer_commander_card(
        self: Arc<Self>,
        job: api::Resolved,
        answered: ipc::AnswerCommanderCard,
    ) -> Result<ipc::CommanderCardAnswered, Refusal> {
        self.commander_card_answered(job, answered).await
    }

    async fn ask_commander(
        self: Arc<Self>,
        job: api::Resolved,
        asked: ipc::AskCommander,
    ) -> Result<ipc::CommanderConversation, Refusal> {
        self.commander_asked(job, asked).await
    }

    async fn get_helm_session(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmSession, Refusal> {
        self.helm_session_read(manifest_id).await
    }

    async fn ask_helm(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<SessionRecord, Refusal> {
        self.helm_asked(asked, manifest_id).await
    }

    async fn get_helm_debug_info(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmDebugInfo, Refusal> {
        self.helm_debugged(manifest_id).await
    }

    async fn helm_polled(&self, manifest_id: ManifestId, counted: EventsSince) {
        self.helm_poll_kept(manifest_id, counted).await;
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
    ) -> Result<SessionRecord, Refusal> {
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
    ) -> Result<HelmSession, Refusal> {
        self.helm_started_fresh(manifest_id).await
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
    pub(super) async fn swept_stale_helm_sessions(&self) {
        let now = self.now();
        let _ = self
            .store()
            .lock()
            .await
            .forget_stale_helm_sessions(&now, self.helm_session_retention());
    }

    /// **The turn every conversation takes**: the stored session is read, the
    /// message is carried to the host, a session the host could not find is
    /// forgotten and the message carried again on a new one (`fresh` tells the
    /// thread why), and the session the host answered under is kept. A Job's
    /// Commander's turn is the caller (`#2166`); Helm's replies were another before Helm became a
    /// Session.
    ///
    /// `carry` builds the message for the session it is given: the stored one,
    /// then none.
    pub(super) async fn turn_carried(
        &self,
        key: &ConversationKey,
        heard: &dyn Heard,
        fresh: impl Fn(),
        carry: impl Fn(Option<String>) -> Carry,
    ) -> Result<(), String> {
        let stored = self
            .store()
            .lock()
            .await
            .helm_session(key.as_str())
            .map_err(unanswered::session_would_not_read)?;
        let host = self.helm().host();
        let mut carried = host.carry(carry(stored.clone()), heard).await;
        if carried == Carried::NoSuchSession && stored.is_some() {
            self.store()
                .lock()
                .await
                .forget_helm_session(key.as_str())
                .map_err(unanswered::lost_session_not_forgotten)?;
            fresh();
            carried = host.carry(carry(None), heard).await;
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
