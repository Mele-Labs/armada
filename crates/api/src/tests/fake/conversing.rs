//! A Helm conversation, as the fake holds one: a channel, and the one
//! repository it serves. **No session and no reply** — what a reply is made of
//! is Fleet's, and this proves the routes and the socket carry what they are
//! handed.

use std::sync::Arc;

use ipc::{
    AskHelm, HelmAsked, HelmBoxFacts, HelmClosed, HelmConversation, HelmMessage, HelmSent, HelmSilence, Instant,
    ManifestId, ReadThought, TellSession, ThoughtKind, ThoughtReading, ThoughtSent, ThoughtTarget, TuneHelm,
};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Conversations, ObservedHelm, Refusal, StoredFile};

/// The one Manifest the fake serves.
pub const SERVED_MANIFEST: &str = "armada";

impl FakeDaemon {
    fn serving(&self, manifest_id: Option<ManifestId>) -> Result<ManifestId, Refusal> {
        let named = manifest_id.unwrap_or_else(|| ManifestId::carried(SERVED_MANIFEST));
        if named.as_str() != SERVED_MANIFEST {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.no_such_manifest",
                format!("no repository with Manifest `{}` is served", named.as_str()),
                run_id(),
            )));
        }
        Ok(named)
    }
}

impl crate::Admitting for FakeDaemon {
    fn helm_at(&self, caller: crate::Caller) -> Option<crate::HelmReach> {
        let planted = *self.helm_on.lock().expect("not poisoned");
        let (port, may) = planted?;
        (caller.port() == Some(port)).then(|| crate::HelmReach::deciding(may, "a test withheld it"))
    }

    fn commander_at(&self, caller: crate::Caller) -> Option<crate::CommanderReach> {
        let port = (*self.commander_on.lock().expect("not poisoned"))?;
        (caller.port() == Some(port)).then(crate::CommanderReach::of_the_allowlist)
    }
}

impl Conversations for FakeDaemon {
    async fn observe_helm(&self, manifest_id: Option<ManifestId>) -> Result<ObservedHelm, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        Ok(ObservedHelm {
            manifest_id,
            replying: false,
            live: self.helm.watch(),
            history: self.helm_thread.lock().expect("not poisoned").clone(),
            skipped: 0,
            facts: self.helm_facts.lock().expect("not poisoned").clone(),
        })
    }

    async fn ask_helm(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        if asked.text.trim().is_empty() && asked.attachments.is_empty() && asked.mentions.is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.helm_message_empty",
                "A message to Helm needs words or something sent with it",
                run_id(),
            )));
        }
        self.helm.offer(HelmMessage::Asked(HelmAsked {
            ts: Instant::carried("2026-09-13T09:00:00.000Z"),
            text: asked.text,
            files: Vec::new(),
            tags: asked.mentions,
        }));
        Ok(HelmConversation {
            manifest_id,
            replying: true,
            resumes: false,
        })
    }

    /// Keeps the choice and offers it, as Fleet does. Which efforts are acceptable is Fleet's.
    async fn tune_helm(
        &self,
        tuned: TuneHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmBoxFacts, Refusal> {
        self.serving(manifest_id)?;
        let facts = HelmBoxFacts {
            model: tuned.model,
            effort: tuned.effort,
            commands: Vec::new(),
        };
        *self.helm_facts.lock().expect("not poisoned") = facts.clone();
        self.helm.offer(HelmMessage::Facts(facts.clone()));
        Ok(facts)
    }

    /// One file, `f1`, which was sent as a picture. Any other id is none.
    async fn get_helm_file(
        &self,
        manifest_id: Option<ManifestId>,
        file: String,
    ) -> Result<StoredFile, Refusal> {
        self.serving(manifest_id)?;
        match file.as_str() {
            "f1" => Ok(StoredFile {
                media_type: String::from("image/png"),
                bytes: vec![0x89, b'P', b'N', b'G'],
            }),
            _ => Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.session_attachment_refused",
                "nothing was sent under that id",
                run_id(),
            ))),
        }
    }

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

    async fn record_thought_sent(
        &self,
        sent: ThoughtSent,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        self.helm.offer(HelmMessage::Sent(HelmSent {
            ts: Instant::carried("2026-09-13T09:00:00.000Z"),
            text: sent.text,
            target: sent.target,
        }));
        Ok(HelmConversation {
            manifest_id,
            replying: false,
            resumes: false,
        })
    }

    /// Keeps the row and answers where the words went. **What a send is, is Fleet's**, and so is
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
        self.helm.offer(HelmMessage::Sent(HelmSent {
            ts: Instant::carried("2026-09-13T09:00:00.000Z"),
            text: tell.text,
            target: target.clone(),
        }));
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

    async fn list_helm_calls(&self) -> Result<ipc::HelmCallsWaiting, Refusal> {
        Ok(ipc::HelmCallsWaiting {
            waiting: Vec::new(),
        })
    }

    async fn answer_helm_call(
        &self,
        _said: ipc::AnswerHelmCall,
    ) -> Result<ipc::HelmCallsWaiting, Refusal> {
        Ok(ipc::HelmCallsWaiting {
            waiting: Vec::new(),
        })
    }

    /// The thread this fake holds, with the facts a record needs around it.
    /// **Nothing is assembled from a session**, for this module's reason.
    async fn get_helm_debug_info(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<ipc::HelmDebugInfo, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        Ok(ipc::HelmDebugInfo {
            manifest_id,
            checkout: String::from("/repos/armada"),
            authority: ipc::HelmActionAuthority::Acting,
            model: String::from("a-model"),
            brief: String::from("You are Helm, in Armada."),
            door: String::from(ipc::door::SERVER),
            tools: vec![String::from("list_jobs")],
            servers: Some(19),
            session: Some(String::from("a-session")),
            thread: Vec::new(),
            cut: 0,
            polled: self.helm_polled.lock().expect("not poisoned").clone(),
            run_id: run_id().as_str().to_string(),
            protocol_id: ipc::ProtocolId::current(),
            at: Instant::carried("2026-09-17T09:00:00.000Z"),
        })
    }

    async fn helm_polled(&self, _manifest_id: ManifestId, counted: ipc::EventsSince) {
        *self.helm_polled.lock().expect("not poisoned") = Some(counted);
    }

    async fn start_helm_fresh(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        self.helm_thread.lock().expect("not poisoned").clear();
        self.helm.offer(HelmMessage::Closed(HelmClosed {
            because: HelmSilence::StartedFresh,
        }));
        Ok(HelmConversation {
            manifest_id,
            replying: false,
            resumes: false,
        })
    }
}
