//! Hosted sessions, as the fake answers them: nothing is hosted here, so every
//! act is refused and the gate passes. What starts, holds and leases is
//! `fleet::session_host`'s, against a real store.

use std::sync::Arc;

use ipc::{
    AnswerSessionAsk, CloseSession, GateAnswer, MessagesHeld, MoveSession, RunSessionCommand, SendSessionMessage, SessionGate, SessionId, TakeHeld,
    SessionRecord, SessionSubagent, SessionThread, SetSessionWorkflow, StartSession, StopSession, TuneSession,
};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{HostedSessions, Refusal, StoredFile};

fn nothing_hosted() -> Refusal {
    Refusal::Unacceptable(ipc::WireError::raised(
        "fake.no_hosted_sessions",
        "the fake hosts no sessions",
        run_id(),
    ))
}

impl HostedSessions for FakeDaemon {
    async fn start_session(&self, _start: StartSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn hand_to_session(self: Arc<Self>, _hand: ipc::HandToSession) -> Result<ipc::SessionHeld, Refusal> {
        Err(nothing_hosted())
    }

    async fn read_session_wait(self: Arc<Self>, _read: ipc::ReadSessionWait) -> Result<ipc::WaitReading, Refusal> {
        Err(nothing_hosted())
    }

    async fn start_waiting_now(self: Arc<Self>, _start: ipc::StartWaitingNow) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn send_session_message(
        self: Arc<Self>,
        _sent: SendSessionMessage,
    ) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn send_session_message_aiming(
        self: Arc<Self>,
        sent: SendSessionMessage,
        aim: Option<ipc::ModAim>,
    ) -> Result<SessionRecord, Refusal> {
        self.messages_aimed.lock().expect("messages aimed").push((sent, aim));
        Err(nothing_hosted())
    }

    async fn send_session_note(self: Arc<Self>, _note: ipc::SendSessionNote) -> Result<ipc::NoteSent, Refusal> {
        Err(nothing_hosted())
    }

    async fn send_session_notes(self: Arc<Self>, _notes: ipc::SendSessionNotes) -> Result<ipc::NotesSent, Refusal> {
        Err(nothing_hosted())
    }

    async fn answer_session_ask(&self, _said: AnswerSessionAsk) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn answer_waiting(
        self: Arc<Self>,
        _said: ipc::AnswerWaiting,
    ) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn dismiss_waiting(
        self: Arc<Self>,
        _dismiss: ipc::DismissWaiting,
    ) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn reopen_waiting(
        self: Arc<Self>,
        _reopen: ipc::ReopenWaiting,
    ) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn confirm_waiting(
        self: Arc<Self>,
        _confirm: ipc::ConfirmWaiting,
    ) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn confirm_later(&self, _confirm: ipc::ConfirmLater) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn get_plan_usage(&self) -> Result<ipc::PlanUsage, Refusal> {
        Ok(ipc::PlanUsage::default())
    }

    async fn get_model_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        Ok(ipc::ModelDownshiftState::default())
    }

    async fn get_agent_sign_in(&self) -> Result<ipc::AgentSignIn, Refusal> {
        Ok(ipc::AgentSignIn::default())
    }

    async fn start_agent_sign_in(self: Arc<Self>) -> Result<ipc::AgentSignIn, Refusal> {
        Err(nothing_hosted())
    }

    async fn accept_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        Err(nothing_hosted())
    }

    async fn restore_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        Err(nothing_hosted())
    }

    async fn keep_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        Err(nothing_hosted())
    }

    async fn settle_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        Err(nothing_hosted())
    }

    async fn get_sleep(&self) -> Result<ipc::SleepState, Refusal> {
        Ok(ipc::SleepState::default())
    }

    async fn set_sleep(self: Arc<Self>, set: ipc::SetSleep) -> Result<ipc::SleepState, Refusal> {
        Ok(ipc::SleepState { on: set.on, ..Default::default() })
    }

    async fn override_sleep(self: Arc<Self>, _over: ipc::OverrideSleep) -> Result<ipc::SleepState, Refusal> {
        Err(nothing_hosted())
    }

    async fn review_sleep(self: Arc<Self>, _review: ipc::ReviewSleep) -> Result<ipc::SleepState, Refusal> {
        Err(nothing_hosted())
    }

    async fn tune_session(&self, _tuned: TuneSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn run_session_command(&self, _run: RunSessionCommand) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn close_session(self: std::sync::Arc<Self>, _closed: CloseSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn move_session(self: std::sync::Arc<Self>, _moved: MoveSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn set_session_auto_land(&self, _asked: ipc::SetSessionAutoLand) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    /// Two servers, one waiting on a sign-in; the job named `closed` is refused as Fleet refuses one.
    async fn get_session_mcp(&self, id: SessionId) -> Result<ipc::SessionMcp, Refusal> {
        if id.as_str() == "closed" {
            return Err(Refusal::IllegalMove(ipc::WireError::raised("fleet.session_closed", "job closed was closed", run_id())));
        }
        Ok(ipc::SessionMcp { servers: two_servers() })
    }

    async fn act_on_session_mcp(&self, asked: ipc::ActOnSessionMcp) -> Result<ipc::SessionMcpActed, Refusal> {
        let sign_in = (asked.act == ipc::McpServerAct::SignIn).then(|| ipc::McpSignIn {
            url: format!("https://{}.example/authorize", asked.server),
            callback_expected: true,
        });
        Ok(ipc::SessionMcpActed { servers: two_servers(), sign_in })
    }

    async fn pause_session(&self, _asked: ipc::PauseSession) -> Result<SessionRecord, Refusal> {
        Err(Refusal::Fault(ipc::WireError::raised("fake.unused", "unused", ipc::RunId::carried("fake"))))
    }

    /// Refused as Fleet refuses a job it does not host, so a route test reads a refusal; the job named
    /// `closed` is refused as a closed one, any other restarts.
    async fn mend_session_tools(self: std::sync::Arc<Self>, asked: ipc::MendSessionTools) -> Result<SessionRecord, Refusal> {
        match asked.session_id.as_str() {
            "closed" => Err(Refusal::IllegalMove(ipc::WireError::raised("fleet.session_closed", "job closed was closed", run_id()))),
            "terminal" => Err(Refusal::Unacceptable(ipc::WireError::raised("fleet.session_tools_unmendable", "job terminal runs in a terminal", run_id()))),
            _ => Err(Refusal::Fault(ipc::WireError::raised("fake.unused", "unused", ipc::RunId::carried("fake")))),
        }
    }

    async fn start_again(self: std::sync::Arc<Self>, _asked: ipc::StartAgain) -> Result<SessionRecord, Refusal> {
        Err(Refusal::Fault(ipc::WireError::raised("fake.unused", "unused", ipc::RunId::carried("fake"))))
    }

    async fn resume_session(self: std::sync::Arc<Self>, _asked: ipc::ResumeSession) -> Result<SessionRecord, Refusal> {
        Err(Refusal::Fault(ipc::WireError::raised("fake.unused", "unused", ipc::RunId::carried("fake"))))
    }

    async fn retry_handoff(self: std::sync::Arc<Self>, _retry: ipc::RetryHandoff) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn stop_session(&self, _stopped: StopSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn set_session_workflow(self: Arc<Self>, _set: SetSessionWorkflow) -> Result<SessionRecord, Refusal> {
        self.session_row.lock().expect("session row").clone().ok_or_else(nothing_hosted)
    }

    async fn skip_session_step(self: Arc<Self>, _skip: ipc::SkipSessionStep) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    /// A Job's copy with a rule long enough to show whether a door's answer carries the text of a piece.
    async fn change_session_steps(self: Arc<Self>, change: ipc::ChangeSessionSteps) -> Result<ipc::JobCopy, Refusal> {
        let step = |id: &str| ipc::JobCopyStep { id: id.into(), label: id.into(), about: None, instructions: None, ends_on: Vec::new(), skipped: false };
        Ok(ipc::JobCopy {
            session_id: change.session_id,
            workflow: Some(ipc::WorkflowId::carried("bug")),
            own: true,
            steps: vec![step("plan"), step("dispatch"), step("handoff")],
            lifecycles: Vec::new(),
            rules: vec![ipc::JobCopyPiece { id: "deploy_watch".into(), text: "version: 1\n".repeat(200) }],
            changes: vec![ipc::JobCopyChange {
                lane: "workflow".into(),
                item: "dispatch".into(),
                text: "Added after plan by the Job".into(),
                by: ipc::ChangedBy::Job,
                at: ipc::Instant::carried("2026-10-30T09:00:00.000Z"),
            }],
        })
    }

    async fn change_job_copy(self: Arc<Self>, _change: ipc::ChangeJobCopy) -> Result<ipc::JobCopy, Refusal> {
        Err(nothing_hosted())
    }

    async fn get_job_copy(self: Arc<Self>, _session_id: Option<SessionId>) -> Result<ipc::JobCopy, Refusal> {
        Err(nothing_hosted())
    }

    async fn keep_job_workflow(self: Arc<Self>, _keep: ipc::KeepJobWorkflow) -> Result<ipc::WorkflowChange, Refusal> {
        Err(nothing_hosted())
    }

    async fn begin_session(self: Arc<Self>, _begin: ipc::BeginSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn release_session_holds(
        &self,
        _release: ipc::ReleaseSessionHolds,
    ) -> Result<ipc::SessionHoldsReleased, Refusal> {
        Err(nothing_hosted())
    }

    async fn get_session(self: Arc<Self>, _id: SessionId) -> Result<SessionThread, Refusal> {
        Err(nothing_hosted())
    }

    async fn get_session_subagent(&self, _id: SessionId, _subagent: String) -> Result<SessionSubagent, Refusal> {
        Err(nothing_hosted())
    }

    async fn get_session_file(&self, _id: SessionId, _file: String) -> Result<StoredFile, Refusal> {
        Err(nothing_hosted())
    }

    async fn gate_session_call(self: Arc<Self>, _gate: SessionGate) -> Result<GateAnswer, Refusal> {
        Ok(GateAnswer::pass())
    }

    async fn ask_from_terminal(&self, _ask: ipc::TerminalAsk) -> Result<ipc::TerminalAsked, Refusal> {
        Ok(ipc::TerminalAsked::Gone {})
    }

    async fn take_held_messages(&self, _ask: TakeHeld) -> Result<MessagesHeld, Refusal> {
        Ok(MessagesHeld::default())
    }
}

fn two_servers() -> Vec<ipc::SessionMcpServer> {
    vec![
        ipc::SessionMcpServer {
            name: "ledger".to_string(),
            state: ipc::McpServerState::Connected,
            error: None,
            scope: Some("user".to_string()),
            tools: 3,
        },
        ipc::SessionMcpServer {
            name: "linear".to_string(),
            state: ipc::McpServerState::NeedsSignIn,
            error: None,
            scope: None,
            tools: 0,
        },
    ]
}
