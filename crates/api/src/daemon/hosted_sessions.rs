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
    SessionRecord, SessionSubagent, SessionThread, SetSessionWorkflow, StartSession, StopSession, TuneSession,
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

    /// `start_session`, as the route calls it: **a start that carries a `request` is read by the
    /// proposer in a task**, which needs the Arc a plain `&self` call does not have. The default
    /// is a start that reads nothing, for a daemon with no proposer.
    fn start_session_reading(
        self: Arc<Self>,
        start: StartSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send {
        async move { self.start_session(start).await }
    }

    /// `hand_to_session`: the person hands one group of a Session's plan to a linked Session, whose
    /// first message is sent. [`Refusal::Unacceptable`] where the plan holds no such group,
    /// [`Refusal::IllegalMove`] where the group has a Session already.
    fn hand_to_session(
        self: Arc<Self>,
        hand: ipc::HandToSession,
    ) -> impl Future<Output = Result<ipc::SessionHeld, Refusal>> + Send;

    /// `read_session_wait`: the proposer reads a brief that tags a Session for the wait it names.
    /// **Never refused for what it could not read**: that is the defaults. [`Refusal::Unacceptable`]
    /// for a Session Fleet does not host.
    fn read_session_wait(
        self: Arc<Self>,
        read: ipc::ReadSessionWait,
    ) -> impl Future<Output = Result<ipc::WaitReading, Refusal>> + Send;

    /// `start_waiting_now`: the person starts a waiting Session at once, whatever its condition.
    /// [`Refusal::IllegalMove`] where it does not wait.
    fn start_waiting_now(
        self: Arc<Self>,
        start: ipc::StartWaitingNow,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `send_session_message`. **Answers at once**, with the turn taken: the
    /// reply is the thread's, as Helm's is.
    fn send_session_message(
        self: Arc<Self>,
        sent: SendSessionMessage,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `send_session_message` with what he picked in an empty Mod Job. A host with no Mod Jobs
    /// takes the message and leaves the aim.
    fn send_session_message_aiming(
        self: Arc<Self>,
        sent: SendSessionMessage,
        _aim: Option<ipc::ModAim>,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send {
        self.send_session_message(sent)
    }

    /// `send_session_note`: a note pinned on a page, sent as `send_session_message` sends with the
    /// lines of code it was drawn from. **Answers at once**, with the turn taken and the candidates
    /// the turn carried.
    fn send_session_note(
        self: Arc<Self>,
        note: ipc::SendSessionNote,
    ) -> impl Future<Output = Result<ipc::NoteSent, Refusal>> + Send;

    /// `send_session_notes`: a walk's notes as one turn, each with its own lines. **Answers at
    /// once**, with the turn taken and one candidate list a note.
    fn send_session_notes(
        self: Arc<Self>,
        notes: ipc::SendSessionNotes,
    ) -> impl Future<Output = Result<ipc::NotesSent, Refusal>> + Send;

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

    /// `confirm_waiting`: thumbs up on an item read from a reply. [`Refusal::IllegalMove`] where
    /// nothing holds the item, and where it was not read from a reply.
    fn confirm_waiting(
        self: Arc<Self>,
        confirm: ipc::ConfirmWaiting,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `confirm_later`: thumbs up on an open follow-up. [`Refusal::IllegalMove`] where it is not open.
    fn confirm_later(
        &self,
        confirm: ipc::ConfirmLater,
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

    /// `get_agent_sign_in`: whether the agents' account is signed out, and whether Fleet is signing it in.
    fn get_agent_sign_in(&self) -> impl Future<Output = Result<ipc::AgentSignIn, Refusal>> + Send;

    /// `start_agent_sign_in`: Fleet runs the agents' sign-in, answered with the state it left. A press
    /// while one runs starts nothing.
    fn start_agent_sign_in(self: Arc<Self>) -> impl Future<Output = Result<ipc::AgentSignIn, Refusal>> + Send;

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

    /// `close_session`: ends the process, and answers once the row says ended. What the session
    /// holds is parked and given back on a task of its own, hence the shared receiver.
    fn close_session(
        self: std::sync::Arc<Self>,
        closed: CloseSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `move_session`: ends the session and continues its conversation in another repository.
    fn move_session(
        self: std::sync::Arc<Self>,
        moved: MoveSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `stop_session`: ends the turn that is running and leaves the session live.
    fn stop_session(
        &self,
        stopped: StopSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `retry_handoff`: runs the hand off that failed again, and answers the new Job.
    fn retry_handoff(
        self: std::sync::Arc<Self>,
        retry: ipc::RetryHandoff,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `pause_session`: ends the turn, parks the slot's work on its branch and gives the slot back.
    fn pause_session(
        &self,
        asked: ipc::PauseSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `set_session_auto_land`: turns Auto land on or off and answers the Session. The flag changes
    /// before the forge is asked, so a pull request the forge refuses is a refusal with the flag set.
    fn set_session_auto_land(
        &self,
        asked: ipc::SetSessionAutoLand,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `get_session_mcp`: the MCP servers a hosted job's process has, asked of it now.
    fn get_session_mcp(
        &self,
        id: SessionId,
    ) -> impl Future<Output = Result<ipc::SessionMcp, Refusal>> + Send;

    /// `act_on_session_mcp`: reconnects, turns on or off, or signs in to one server, then reads
    /// the servers again.
    fn act_on_session_mcp(
        &self,
        asked: ipc::ActOnSessionMcp,
    ) -> impl Future<Output = Result<ipc::SessionMcpActed, Refusal>> + Send;

    /// `resume_session`: takes a slot again at the branch tip, or waits for one.
    fn resume_session(
        &self,
        asked: ipc::ResumeSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `set_session_workflow`: the person picked the Session's workflow in a panel, or cleared
    /// it. A Session that has run is told at once.
    fn set_session_workflow(
        self: Arc<Self>,
        set: SetSessionWorkflow,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;
    /// `begin_session`: Start pressed on a held Job's start card.
    fn begin_session(
        self: Arc<Self>,
        begin: ipc::BeginSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;
    /// `release_session_holds`: the person gives back what a Session holds on the ledger, or hands
    /// it to another live Session of the same repository. [`Refusal::IllegalMove`] for a target
    /// that is the Session itself, has ended or is of another repository.
    fn release_session_holds(
        &self,
        release: ipc::ReleaseSessionHolds,
    ) -> impl Future<Output = Result<ipc::SessionHoldsReleased, Refusal>> + Send;

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
