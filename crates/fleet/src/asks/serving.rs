//! `api::Conversations`, implemented over a real Fleet: the calls a hosted Job puts to a person and
//! the words passed to a running Job. `#939`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Conversations as Surface, Refusal};
use ipc::{AskingToRun, ManifestId, ReadThought, RunOrNot, TellSession, ThoughtReading, ThoughtTarget};

use crate::asks::unplaced;
use crate::daemon::Fleet;

impl Surface for Fleet
{
    async fn ask_the_person(
        &self,
        _asking: AskingToRun,
        _manifest_id: Option<ManifestId>,
    ) -> Result<RunOrNot, Refusal> {
        Ok(unplaced())
    }

    async fn ask_the_person_at(
        self: Arc<Self>,
        asking: AskingToRun,
        _manifest_id: Option<ManifestId>,
        caller: api::Caller,
    ) -> Result<RunOrNot, Refusal> {
        match self.session_holding(&caller) {
            Some(session) => self.session_permission(&session, asking).await,
            // The door offers this to a hosted Job and to no one else, so only a hand-made POST gets here.
            None => Ok(unplaced()),
        }
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
