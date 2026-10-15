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
    AnswerSessionAsk, CloseSession, GateAnswer, MessagesHeld, MoveSession, RunSessionCommand, SendSessionMessage, SessionGate, SessionId, TakeHeld,
    SessionRecord, SessionSubagent, SessionThread, StartSession, StopSession, TuneSession,
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

    /// `dismiss_waiting`: the person drops one item for good. [`Refusal::IllegalMove`] where
    /// nothing holds the item.
    fn dismiss_waiting(
        self: Arc<Self>,
        dismiss: ipc::DismissWaiting,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `get_plan_usage`: the newest reading of the plan's windows. Empty where none has come.
    fn get_plan_usage(&self) -> impl Future<Output = Result<ipc::PlanUsage, Refusal>> + Send;

    /// `get_model_downshift`: where the default model's step down stands. Empty where there is none.
    fn get_model_downshift(&self) -> impl Future<Output = Result<ipc::ModelDownshiftState, Refusal>> + Send;

    /// `accept_downshift`: the lower model becomes the default until the window resets.
    /// [`Refusal::IllegalMove`] where nothing is offered, and whatever a refused settings write says.
    fn accept_downshift(&self) -> impl Future<Output = Result<ipc::ModelDownshiftState, Refusal>> + Send;

    /// `restore_downshift`: the default goes back. [`Refusal::IllegalMove`] unless shifted or reset.
    fn restore_downshift(&self) -> impl Future<Output = Result<ipc::ModelDownshiftState, Refusal>> + Send;

    /// `keep_downshift`: the lower model stays the default. [`Refusal::IllegalMove`] unless reset.
    fn keep_downshift(&self) -> impl Future<Output = Result<ipc::ModelDownshiftState, Refusal>> + Send;

    /// `settle_downshift`: the owner sets the default by hand. [`Refusal::IllegalMove`] unless offered.
    fn settle_downshift(&self) -> impl Future<Output = Result<ipc::ModelDownshiftState, Refusal>> + Send;

    /// `get_sleep`: the switch and the night's rows.
    fn get_sleep(&self) -> impl Future<Output = Result<ipc::SleepState, Refusal>> + Send;

    /// `set_sleep`: on starts a new night, off keeps it for the review.
    fn set_sleep(
        self: Arc<Self>,
        set: ipc::SetSleep,
    ) -> impl Future<Output = Result<ipc::SleepState, Refusal>> + Send;

    /// `override_sleep`: tell the session what the owner would have said. [`Refusal::IllegalMove`]
    /// where no decision holds the id, [`Refusal::Unacceptable`] for blank words.
    fn override_sleep(
        self: Arc<Self>,
        over: ipc::OverrideSleep,
    ) -> impl Future<Output = Result<ipc::SleepState, Refusal>> + Send;

    /// `review_sleep`: the owner kept a decision. [`Refusal::IllegalMove`] where no decision holds the id.
    fn review_sleep(
        self: Arc<Self>,
        review: ipc::ReviewSleep,
    ) -> impl Future<Output = Result<ipc::SleepState, Refusal>> + Send;

    /// `tune_session`.
    fn tune_session(
        &self,
        tuned: TuneSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `run_session_command`: a slash command a terminal session's mod runs as typed.
    fn run_session_command(
        &self,
        run: RunSessionCommand,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `close_session`: ends the process, gives back what the session holds.
    fn close_session(
        &self,
        closed: CloseSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `move_session`: ends the session and continues its conversation in another repository.
    fn move_session(
        &self,
        moved: MoveSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `stop_session`: ends the turn that is running and leaves the session live.
    fn stop_session(
        &self,
        stopped: StopSession,
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
        self: Arc<Self>,
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
