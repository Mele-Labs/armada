//! `api::Conversations`, implemented over a real Fleet: Helm's Session found, written to, ended
//! and recorded (`session.rs`). `#939`.
//!
//! **Helm is a hosted Session** and its messages are a Session's, so what a turn is made of is
//! `session_host`'s.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Conversations as Surface, Refusal};
use ipc::{
    AnswerHelmCall, AskHelm, AskingToRun, EventsSince, HelmCallsWaiting, HelmDebugInfo,
    HelmSession, ManifestId, ReadThought, RunOrNot, SessionRecord, TellSession, ThoughtReading,
    ThoughtSent, ThoughtTarget,
};

use crate::daemon::Fleet;

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
        self: Arc<Self>,
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
