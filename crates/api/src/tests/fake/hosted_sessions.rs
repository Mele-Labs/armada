//! Hosted sessions, as the fake answers them: nothing is hosted here, so every
//! act is refused and the gate passes. What starts, holds and leases is
//! `fleet::session_host`'s, against a real store.

use std::sync::Arc;

use ipc::{
    AnswerSessionAsk, CloseSession, GateAnswer, MessagesHeld, SendSessionMessage, SessionGate,
    SessionId, SessionRecord, SessionThread, StartSession, TakeHeld, TuneSession,
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

    async fn send_session_message(
        self: Arc<Self>,
        _sent: SendSessionMessage,
    ) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn answer_session_ask(&self, _said: AnswerSessionAsk) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn tune_session(&self, _tuned: TuneSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn close_session(&self, _closed: CloseSession) -> Result<SessionRecord, Refusal> {
        Err(nothing_hosted())
    }

    async fn get_session(self: Arc<Self>, _id: SessionId) -> Result<SessionThread, Refusal> {
        Err(nothing_hosted())
    }

    async fn get_session_file(&self, _id: SessionId, _file: String) -> Result<StoredFile, Refusal> {
        Err(nothing_hosted())
    }

    async fn gate_session_call(&self, _gate: SessionGate) -> Result<GateAnswer, Refusal> {
        Ok(GateAnswer::pass())
    }

    async fn take_held_messages(&self, _ask: TakeHeld) -> Result<MessagesHeld, Refusal> {
        Ok(MessagesHeld::default())
    }
}
