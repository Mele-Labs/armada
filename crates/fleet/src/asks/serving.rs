//! `api::Conversations`, implemented over a real Fleet: the calls held for a person and the words
//! passed to a running Job. `#939`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Conversations as Surface, Refusal};
use ipc::{
    AnswerCall, AskingToRun, CallsWaiting, ManifestId, ReadThought, RunOrNot, TellSession,
    ThoughtReading, ThoughtTarget,
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
    async fn ask_the_person(
        &self,
        asking: AskingToRun,
        manifest_id: Option<ManifestId>,
    ) -> Result<RunOrNot, Refusal> {
        self.call_permission(asking, manifest_id).await
    }

    async fn ask_the_person_at(
        self: Arc<Self>,
        asking: AskingToRun,
        manifest_id: Option<ManifestId>,
        caller: api::Caller,
    ) -> Result<RunOrNot, Refusal> {
        match self.session_holding(&caller) {
            Some(session) => self.session_permission(&session, asking).await,
            None => self.call_permission(asking, manifest_id).await,
        }
    }

    async fn list_calls(&self) -> Result<CallsWaiting, Refusal> {
        Ok(self.calls_waiting())
    }

    async fn answer_call(&self, said: AnswerCall) -> Result<CallsWaiting, Refusal> {
        self.call_answered(said)
    }

    async fn read_thought(&self, read: ReadThought) -> ThoughtReading {
        self.thought_read(read).await
    }

    async fn tell_session(
        self: Arc<Self>,
        tell: TellSession,
        manifest_id: Option<ManifestId>,
    ) -> Result<ThoughtTarget, Refusal> {
        self.session_told(tell, manifest_id).await
    }
}
