//! A session Bridge hosts: start it, talk to it, answer what it asks, tune it,
//! close it, read its thread. Since 23.47. `docs/concepts/session.md`.
//!
//! **Beside [`Sessions`](super::Sessions) and not in it**, for that trait's
//! reason one subject over: the ledger is what a harness reports and these are
//! what Fleet itself drives, and a handler that takes only the first cannot
//! start a process.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    AnswerSessionAsk, CloseSession, GateAnswer, SendSessionMessage, SessionGate, SessionId,
    SessionRecord, SessionThread, StartSession, TuneSession,
};

use crate::daemon::Refusal;

/// One stored file, as `get_session_file` serves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFile {
    pub media_type: String,
    pub bytes: Vec<u8>,
}

pub trait HostedSessions: Send + Sync + 'static {
    /// `start_session`. The row exists and holds no slot and no branch; no
    /// process starts until the first message.
    fn start_session(
        &self,
        start: StartSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `send_session_message`. **Answers at once**, with the turn taken: the
    /// reply is the thread's, as Helm's is.
    fn send_session_message(
        self: Arc<Self>,
        sent: SendSessionMessage,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `answer_session_ask`. [`Refusal::IllegalMove`] where nothing is waiting.
    fn answer_session_ask(
        &self,
        said: AnswerSessionAsk,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `tune_session`.
    fn tune_session(
        &self,
        tuned: TuneSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `close_session`: ends the process, gives back what the session holds.
    fn close_session(
        &self,
        closed: CloseSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `get_session`: the row and its thread.
    fn get_session(
        &self,
        id: SessionId,
    ) -> impl Future<Output = Result<SessionThread, Refusal>> + Send;

    /// `get_session_file`: what a message carried.
    fn get_session_file(
        &self,
        id: SessionId,
        file: String,
    ) -> impl Future<Output = Result<StoredFile, Refusal>> + Send;

    /// `gate_session_call`: the hook a hosted session runs before a tool.
    fn gate_session_call(
        &self,
        gate: SessionGate,
    ) -> impl Future<Output = Result<GateAnswer, Refusal>> + Send;
}
