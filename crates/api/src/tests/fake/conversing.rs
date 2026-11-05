//! Words for a Job and calls to a person, as the fake holds them: the one repository it serves.
//! What reads the words and what a send is are Fleet's, and this proves the routes and the socket
//! carry what they are handed.

use std::sync::Arc;

use ipc::{ManifestId, ReadThought, TellSession, ThoughtKind, ThoughtReading, ThoughtTarget};

use super::FakeDaemon;
use crate::{Conversations, Refusal};

/// The one Manifest the fake serves.
pub const SERVED_MANIFEST: &str = "armada";

impl crate::Admitting for FakeDaemon {
    fn words_reach_at(&self, caller: crate::Caller) -> Option<crate::WordsReach> {
        let session = *self.session_on.lock().expect("not poisoned");
        if session.is_some() && caller.port() == session {
            return Some(crate::WordsReach::of_a_hosted_session());
        }
        let planted = *self.words_on.lock().expect("not poisoned");
        let (port, may) = planted?;
        (caller.port() == Some(port)).then(|| crate::WordsReach::deciding(may, "a test withheld it"))
    }
}

impl Conversations for FakeDaemon {
    /// **A question, unless the words ask to be routed**: then a thought for the one Session the
    /// fake knows. What reads the words is Fleet's.
    async fn read_thought(&self, read: ReadThought) -> ThoughtReading {
        if !read.text.contains("route") {
            return ThoughtReading::Question;
        }
        ThoughtReading::Thought {
            target: Some(ThoughtTarget {
                kind: ThoughtKind::Session,
                id: String::from("01SESSION000000000000000001"),
                title: String::from("Sessions list views"),
                manifest_id: Some(ManifestId::carried(SERVED_MANIFEST)),
                slot: Some(3),
            }),
        }
    }

    /// Keeps the words and answers where they went. **What a send is, is Fleet's**, and so is
    /// which repository serves the thread: the door names the Manifest the caller stands in, and
    /// the fake takes it as it is.
    async fn tell_session(
        self: Arc<Self>,
        tell: TellSession,
        manifest_id: Option<ManifestId>,
    ) -> Result<ThoughtTarget, Refusal> {
        let manifest_id = manifest_id.unwrap_or_else(|| ManifestId::carried(SERVED_MANIFEST));
        let target = ThoughtTarget {
            kind: ThoughtKind::Session,
            id: tell.session_id.as_str().to_string(),
            title: String::from("Sessions list views"),
            manifest_id: Some(manifest_id),
            slot: None,
        };
        self.words_sent
            .lock()
            .expect("not poisoned")
            .push((tell.text, target.clone()));
        Ok(target)
    }

    /// Answers `allow` at once. **The wait is Fleet's**, and a fake that held
    /// the call would make every door case in this crate take five minutes.
    async fn ask_the_person(
        &self,
        asking: ipc::AskingToRun,
        _manifest_id: Option<ManifestId>,
    ) -> Result<ipc::RunOrNot, Refusal> {
        self.asked_to_run
            .lock()
            .expect("not poisoned")
            .push(asking.clone());
        Ok(ipc::RunOrNot::Allow {
            updated_input: asking.input,
        })
    }

    async fn list_calls(&self) -> Result<ipc::CallsWaiting, Refusal> {
        Ok(ipc::CallsWaiting {
            waiting: Vec::new(),
        })
    }

    async fn answer_call(
        &self,
        _said: ipc::AnswerCall,
    ) -> Result<ipc::CallsWaiting, Refusal> {
        Ok(ipc::CallsWaiting {
            waiting: Vec::new(),
        })
    }
}
