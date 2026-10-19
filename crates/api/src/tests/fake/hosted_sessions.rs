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

    async fn send_session_note(self: Arc<Self>, _note: ipc::SendSessionNote) -> Result<ipc::NoteSent, Refusal> {
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

    async fn get_plan_usage(&self) -> Result<ipc::PlanUsage, Refusal> {
        Ok(ipc::PlanUsage::default())
    }

    async fn get_model_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        Ok(ipc::ModelDownshiftState::default())
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

    async fn close_session(&self, _closed: CloseSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn move_session(&self, _moved: MoveSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn pause_session(&self, _asked: ipc::PauseSession) -> Result<SessionRecord, Refusal> {
        Err(Refusal::Fault(ipc::WireError::raised("fake.unused", "unused", ipc::RunId::carried("fake"))))
    }

    async fn resume_session(&self, _asked: ipc::ResumeSession) -> Result<SessionRecord, Refusal> {
        Err(Refusal::Fault(ipc::WireError::raised("fake.unused", "unused", ipc::RunId::carried("fake"))))
    }

    async fn retry_handoff(&self, _retry: ipc::RetryHandoff) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn stop_session(&self, _stopped: StopSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn set_session_workflow(self: Arc<Self>, _set: SetSessionWorkflow) -> Result<SessionRecord, Refusal> {
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
