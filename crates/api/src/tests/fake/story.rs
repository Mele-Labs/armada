//! The words, asks and story as the fake answers them: nothing is sorted and no ask is kept. What is sorted, opened and
//! moved is `fleet`'s.

use ipc::{Ask, ChangeAsk, FeatureAsks, FileAsk, JobAsks, JobStory, MessageSorting, MessageSortings, SessionId, SetMessageKind, SortedBy, Instant};

use super::FakeDaemon;
use crate::{Refusal, Story};

impl Story for FakeDaemon {
    async fn list_message_kinds(&self, session_id: SessionId) -> Result<MessageSortings, Refusal> {
        Ok(MessageSortings { session_id, messages: Vec::new() })
    }

    async fn set_message_kind(self: std::sync::Arc<Self>, set: SetMessageKind) -> Result<MessageSorting, Refusal> {
        Ok(MessageSorting {
            session_id: set.session_id,
            row_id: set.row_id,
            kind: set.kind,
            ask_kind: set.ask_kind,
            ask_id: set.ask_id,
            title: None,
            sorted_by: SortedBy::Owner,
            sorted_at: Instant::carried("2026-11-16T09:00:00.000Z"),
        })
    }

    async fn list_job_asks(&self, session_id: SessionId) -> Result<JobAsks, Refusal> {
        Ok(JobAsks { session_id, asks: Vec::new() })
    }

    async fn get_job_story(&self, session_id: SessionId) -> Result<JobStory, Refusal> {
        Ok(JobStory { session_id, asks: Vec::new(), shown_otherwise: Vec::new(), lead: Vec::new() })
    }

    async fn change_ask(&self, change: ChangeAsk) -> Result<Ask, Refusal> {
        Err(Refusal::NotFound(ipc::WireError::raised("fleet.no_such_ask", format!("no ask is named {}", change.ask_id), crate::tests::shapes::run_id())))
    }

    async fn list_feature_asks(&self, project: String, feature: Option<String>) -> Result<FeatureAsks, Refusal> {
        Ok(FeatureAsks { project, feature, asks: Vec::new() })
    }

    async fn file_ask(&self, _caller: Option<crate::Caller>, file: FileAsk) -> Result<Ask, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised("fleet.feature_project", format!("no project is named {}", file.project), crate::tests::shapes::run_id())))
    }
}
