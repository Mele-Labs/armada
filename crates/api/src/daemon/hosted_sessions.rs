//! A session Bridge hosts: start it, talk to it, answer what it asks, tune it,
//! close it, read its thread. Since 23.49. `docs/concepts/session.md`.
//!
//! **Beside [`Sessions`](super::Sessions) and not in it**, for that trait's
//! reason one subject over: the ledger is what a harness reports and these are
//! what Fleet itself drives, and a handler that takes only the first cannot
//! start a process.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    AnswerSessionAsk, CloseSession, GateAnswer, MessagesHeld, SendSessionMessage, SessionGate, SessionId, TakeHeld,
    SessionRecord, SessionSubagent, SessionThread, StartSession, TuneSession,
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

    /// `answer_waiting`: the person settles one item of what a session waits on. Routed by the
    /// item's source. [`Refusal::IllegalMove`] where nothing holds the item, and
    /// [`Refusal::Unacceptable`] for an answer that says nothing.
    fn answer_waiting(
        self: Arc<Self>,
        said: ipc::AnswerWaiting,
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
        self: Arc<Self>,
        id: SessionId,
    ) -> impl Future<Output = Result<SessionThread, Refusal>> + Send;

    /// `get_session_subagent`: a subagent's own thread.
    fn get_session_subagent(
        &self,
        id: SessionId,
        subagent: String,
    ) -> impl Future<Output = Result<SessionSubagent, Refusal>> + Send;

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

    /// `ask_from_terminal`: a terminal session's question put to Bridge, or the
    /// terminal's own prompt having ended first.
    fn ask_from_terminal(
        &self,
        ask: ipc::TerminalAsk,
    ) -> impl Future<Output = Result<ipc::TerminalAsked, Refusal>> + Send;

    /// `take_held_messages`: what a person sent a terminal session, handed to
    /// its mod once. **Every ask also says the session is listening.**
    fn take_held_messages(
        &self,
        ask: TakeHeld,
    ) -> impl Future<Output = Result<MessagesHeld, Refusal>> + Send;
}
