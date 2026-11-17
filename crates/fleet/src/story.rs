//! What the owner said in a Job, sorted by kind, the asks that came of it, and a Job's story. `docs/concepts/job-story.md`.
//!
//! **The words are sorted after they are delivered** (`reading`), an ask opens from a sorted message (`asks`), moves as Fleet
//! sees the work (`progress`), and the story is built from the asks, the Job's artifacts and its merges (`telling`).
//! Feature slugs are plain strings here: the library that names them is another surface's.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Story};
use ipc::{Ask, ChangeAsk, FeatureAsks, FileAsk, JobAsks, JobStory, MessageSorting, MessageSortings, SessionId, SetMessageKind};

use crate::daemon::Fleet;
use crate::session_host::serving::NO_SUCH_SESSION;

mod asks;
mod outside;
mod progress;
mod reading;
pub(crate) mod sorting;
pub(crate) mod telling;


impl<H, V, W> Story for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn list_message_kinds(&self, session_id: SessionId) -> Result<MessageSortings, Refusal> {
        self.message_kinds_listed(&session_id).await
    }

    async fn set_message_kind(self: Arc<Self>, set: SetMessageKind) -> Result<MessageSorting, Refusal> {
        self.message_kind_set(set).await
    }

    async fn list_job_asks(&self, session_id: SessionId) -> Result<JobAsks, Refusal> {
        self.job_asks_listed(&session_id).await
    }

    async fn get_job_story(&self, session_id: SessionId) -> Result<JobStory, Refusal> {
        self.job_story_read(&session_id).await
    }

    async fn change_ask(&self, change: ChangeAsk) -> Result<Ask, Refusal> {
        self.ask_changed(change).await
    }

    async fn list_feature_asks(&self, project: String, feature: Option<String>) -> Result<FeatureAsks, Refusal> {
        self.feature_asks_listed(&project, feature.as_deref()).await
    }

    async fn file_ask(&self, caller: Option<api::Caller>, file: FileAsk) -> Result<Ask, Refusal> {
        self.ask_filed(caller, file).await
    }
}
