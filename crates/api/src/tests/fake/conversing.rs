//! A Helm conversation, as the fake holds one: a channel, and the one
//! repository it serves. **No session and no reply** — what a reply is made of
//! is Fleet's, and this proves the routes and the socket carry what they are
//! handed.

use std::sync::Arc;

use ipc::{
    AskHelm, HelmSession, Instant, ManifestId, ReadThought, SessionRecord, TellSession,
    ThoughtKind, ThoughtReading, ThoughtSent, ThoughtTarget,
};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Conversations, Refusal};

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
        let session = *self.session_on.lock().expect("not poisoned");
        if session.is_some() && caller.port() == session {
            return Some(crate::HelmReach::of_a_hosted_session());
        }
        let planted = *self.helm_on.lock().expect("not poisoned");
        let (port, may) = planted?;
        (caller.port() == Some(port)).then(|| crate::HelmReach::deciding(may, "a test withheld it"))
    }
}

impl Conversations for FakeDaemon {
    /// The Helm Session this fake has, once a message was asked of it: no rows, because what a
    /// Session's thread is made of is Fleet's.
    async fn get_helm_session(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmSession, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        let asked = !self.helm_asked.lock().expect("not poisoned").is_empty();
        Ok(HelmSession {
            session: asked.then(|| ipc::SessionThread {
                session: helm_record(&manifest_id),
                rows: Vec::new(),
            }),
            manifest_id,
        })
    }

    async fn ask_helm(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<SessionRecord, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        if asked.text.trim().is_empty() && asked.attachments.is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.session_message_empty",
                "a message needs words or something sent with it",
                run_id(),
            )));
        }
        self.helm_asked.lock().expect("not poisoned").push(asked);
        Ok(helm_record(&manifest_id))
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
    ) -> Result<SessionRecord, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        self.helm_sent
            .lock()
            .expect("not poisoned")
            .push((sent.text, sent.target));
        Ok(helm_record(&manifest_id))
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
        self.helm_sent
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
            note: String::from("You are Helm, docked in Armada."),
            door: String::from(ipc::door::SERVER),
            tools: vec![String::from("list_jobs")],
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
    ) -> Result<HelmSession, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        self.helm_asked.lock().expect("not poisoned").clear();
        Ok(HelmSession {
            manifest_id,
            session: None,
        })
    }
}

/// The fake's Helm Session: a hosted Session row with the mark set and nothing on it.
fn helm_record(manifest_id: &ManifestId) -> SessionRecord {
    let at = Instant::carried("2026-10-13T09:00:00.000Z");
    SessionRecord {
        id: ipc::SessionId::carried("01HELMSESSION00000000000001"),
        harness: String::from("a-harness"),
        origin: ipc::SessionOrigin::Bridge,
        manifest_id: Some(manifest_id.clone()),
        cwd: String::from("/repos/armada"),
        title: Some(String::from("Helm")),
        state: ipc::SessionState::Live,
        started_at: at.clone(),
        last_seen_at: at,
        last_turn_at: None,
        ended_at: None,
        end_reason: None,
        usage: ipc::SessionUsage::default(),
        attachments: Vec::new(),
        hosted: None,
        terminal: None,
        mod_out_of_date: false,
        tools_missing: None,
        waiting_for: Vec::new(),
        done_since: None,
        standing: None,
        held_until: None,
        context_breakdown: None,
        loadout_summary: None,
        helm: true,
        r#mod: false,
        auto_land: false,
        activity: None,
        build: None,
        workflow: None,
        proposal: None,
        plan: None,
        touches: Vec::new(),
        workflow_step: None,
        judging: false,
        judge: None,
        settled: Vec::new(),
        closed: Vec::new(),
        waits: None,
        later: Vec::new(),
        held_open_by: Vec::new(),
        started: None,
        workflow_visits: Vec::new(),
        skipped_steps: Vec::new(),
        workflow_waiting_on: Vec::new(),
        workflow_copy: None,
    }
}
