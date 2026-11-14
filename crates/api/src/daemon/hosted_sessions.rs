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

    /// `dispatch_job`: the person's work, started as a Job or held by the Operator, and what Bridge sends into the
    /// Job sent after it.
    fn dispatch_job(
        self: Arc<Self>,
        dispatch: ipc::DispatchJob,
    ) -> impl Future<Output = Result<ipc::JobDispatched, Refusal>> + Send;

    /// `list_parked_ideas`: the dispatches kept for later, oldest first.
    fn list_parked_ideas(self: Arc<Self>) -> impl Future<Output = Result<ipc::ParkedIdeas, Refusal>> + Send;

    /// `remove_parked_idea`: one press. [`Refusal::Unacceptable`] where no idea is kept under the id.
    fn remove_parked_idea(self: Arc<Self>, remove: ipc::RemoveParkedIdea) -> impl Future<Output = Result<ipc::ParkedIdeas, Refusal>> + Send;

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

    /// `message_reach`: whether the Job holding `caller`'s connection may `SendMessage` the address
    /// `to`. **Allowed by default**, and for any caller that is not a hosted Job.
    fn message_reach(
        self: Arc<Self>,
        _caller: crate::Caller,
        _to: ipc::MessageTo,
    ) -> impl Future<Output = Result<ipc::MessageReach, Refusal>> + Send {
        async { Ok(ipc::MessageReach::Allowed {}) }
    }

    /// `start_waiting_now`: the person starts a waiting Session at once, whatever its condition.
    /// [`Refusal::IllegalMove`] where it does not wait.
    fn start_waiting_now(
        self: Arc<Self>,
        start: ipc::StartWaitingNow,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `send_session_message`. **Answers at once**, with the turn taken: the
    /// reply is the thread's.
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

    /// `edit_session_message`: new words for a message the person sent. A turn still held changes in
    /// place; one already written is corrected, the row marked and the agent told. **Answers at once**,
    /// the reply the thread's.
    fn edit_session_message(
        self: Arc<Self>,
        edit: ipc::EditSessionMessage,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

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

    /// `answer_waiting_parts`: the person answers an item by picking parts across its options' pictures.
    /// [`Refusal::IllegalMove`] where nothing holds the item, and [`Refusal::Unacceptable`] for no part, an
    /// option the item lacks, or a part its picture lacks.
    fn answer_waiting_parts(
        self: Arc<Self>,
        said: ipc::AnswerWaitingParts,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `free_slot`: the person gives back a slot a Job holds to the waiting Job whose call lists it.
    /// [`Refusal::IllegalMove`] where the call is gone, the holder is working or holds uncommitted work,
    /// or the slot is no longer the holder's.
    fn free_slot(
        self: Arc<Self>,
        free: ipc::FreeSlot,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `move_in_line`: the person moves a Job waiting to start, over the Operator's order.
    /// [`Refusal::Unacceptable`] while the Operator is off, where the Job is not waiting, or where
    /// the place names a Job not waiting in its line.
    fn move_in_line(
        self: Arc<Self>,
        moved: ipc::MoveInLine,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `talk_to_operator`: one message in the box. [`Refusal::Unacceptable`] while the Operator is
    /// off, at the day's ceiling, or where the call came back with nothing.
    fn talk_to_operator(
        self: Arc<Self>,
        talk: ipc::operator_talk::TalkToOperator,
    ) -> impl Future<Output = Result<ipc::operator_talk::OperatorSaid, Refusal>> + Send;

    /// `operator_talk`: the conversation so far.
    fn operator_talk(self: Arc<Self>) -> impl Future<Output = Result<ipc::operator_talk::OperatorTalk, Refusal>> + Send;

    /// `operator_in_front`: Bridge's window came to the front. Answers at once; coming back after time
    /// away sets when he last looked and rewrites the page's top line.
    fn operator_in_front(self: Arc<Self>) -> impl Future<Output = Result<ipc::operator_page::OperatorInFront, Refusal>> + Send;

    /// `operator_page`: Your turn and the digest beside it.
    fn operator_page(self: Arc<Self>) -> impl Future<Output = Result<ipc::operator_page::OperatorPage, Refusal>> + Send;

    /// `operator_answer`: one press on an item of Your turn. [`Refusal::NotFound`] where no open item has
    /// the id, [`Refusal::Unacceptable`] where the answer is not offered on it or the Operator is off.
    fn operator_answer(
        self: Arc<Self>,
        press: ipc::operator_page::OperatorAnswerPress,
    ) -> impl Future<Output = Result<ipc::operator_page::OperatorAnswered, Refusal>> + Send;

    /// `operator_undo_ruling`: take back a ruling the Operator made by following something he said.
    /// [`Refusal::NotFound`] where no precedent ruling has the id, [`Refusal::Unacceptable`] where it is taken
    /// back already or its Job has ended.
    fn operator_undo_ruling(
        self: Arc<Self>,
        undo: ipc::operator_page::OperatorUndoRuling,
    ) -> impl Future<Output = Result<ipc::operator_page::OperatorPage, Refusal>> + Send;

    /// `operator_turn_order`: Your turn as he dragged it.
    fn operator_turn_order(
        self: Arc<Self>,
        order: ipc::operator_page::OperatorTurnOrder,
    ) -> impl Future<Output = Result<ipc::operator_page::OperatorPage, Refusal>> + Send;

    /// `operator_later`: one item goes last for the rest of the local day. [`Refusal::NotFound`] where no
    /// open item has the id.
    fn operator_later(
        self: Arc<Self>,
        later: ipc::operator_page::OperatorLater,
    ) -> impl Future<Output = Result<ipc::operator_page::OperatorPage, Refusal>> + Send;

    /// `get_operator_board`: what the Operator is ruling on, what waits and what it settled.
    fn get_operator_board(self: Arc<Self>) -> impl Future<Output = Result<ipc::operator_board::OperatorBoard, Refusal>> + Send;

    /// `move_on_operator_board`: one press reorders what waits. [`Refusal::NotFound`] where the item,
    /// or the one it goes before, is not waiting. Answers the board.
    fn move_on_operator_board(
        self: Arc<Self>,
        moved: ipc::operator_board::MoveOnOperatorBoard,
    ) -> impl Future<Output = Result<ipc::operator_board::OperatorBoard, Refusal>> + Send;

    /// `report_bridge_error`: an error Bridge showed the person, handed to the Operator. Never refused for
    /// the Operator being off: it is let go and `repeated` is `false`.
    fn report_bridge_error(
        self: Arc<Self>,
        error: ipc::operator_errors::ReportBridgeError,
    ) -> impl Future<Output = Result<ipc::operator_errors::BridgeErrorTaken, Refusal>> + Send;

    /// `answer_operator_suggestion`: one press on a suggestion no Job holds. [`Refusal::NotFound`] where
    /// none has that id, [`Refusal::IllegalMove`] where it is settled already. Answers the board.
    fn answer_operator_suggestion(
        self: Arc<Self>,
        answer: ipc::operator_board::AnswerOperatorSuggestion,
    ) -> impl Future<Output = Result<ipc::operator_board::OperatorBoard, Refusal>> + Send;

    /// `confirm_operator_order`: the person presses to have an order stand. [`Refusal::Unacceptable`]
    /// while the Operator is off, or where the form names what Fleet does not know.
    fn confirm_operator_order(
        self: Arc<Self>,
        confirm: ipc::operator_talk::ConfirmOperatorOrder,
    ) -> impl Future<Output = Result<ipc::operator_talk::OperatorOrders, Refusal>> + Send;

    /// `operator_orders`: the standing orders in force now.
    fn operator_orders(self: Arc<Self>) -> impl Future<Output = Result<ipc::operator_talk::OperatorOrders, Refusal>> + Send;

    /// `remove_operator_order`: one press. [`Refusal::Unacceptable`] where it is not standing.
    fn remove_operator_order(
        self: Arc<Self>,
        remove: ipc::operator_talk::RemoveOperatorOrder,
    ) -> impl Future<Output = Result<ipc::operator_talk::OperatorOrders, Refusal>> + Send;

    /// `dismiss_waiting`: the person drops one item for good. [`Refusal::IllegalMove`] where
    /// nothing holds the item.
    fn dismiss_waiting(
        self: Arc<Self>,
        dismiss: ipc::DismissWaiting,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `answer_items`: his verdict on several items that ask for his approval, each settled as its own answer
    /// settles it. `refused` names the items the route refused itself, which are not settled. Never refused whole.
    fn answer_items(
        self: Arc<Self>,
        answer: ipc::AnswerItems,
        refused: Vec<Option<ipc::ItemSettled>>,
    ) -> impl Future<Output = ipc::ItemsSettled> + Send;

    /// `dismiss_items`: several items that ask for his approval off his list with no verdict, each Job told in one
    /// message. `refused` as for `answer_items`. Never refused whole.
    fn dismiss_items(
        self: Arc<Self>,
        dismiss: ipc::DismissItems,
        refused: Vec<Option<ipc::ItemSettled>>,
    ) -> impl Future<Output = ipc::ItemsSettled> + Send;

    /// `convert_draft_to_document`: the person turns an open draft into a Document in the job's worktree.
    /// [`Refusal::NotFound`] for a draft the job has no row for, [`Refusal::IllegalMove`] for one already
    /// settled and for a job with no worktree, [`Refusal::Unacceptable`] for a proposal's draft.
    fn convert_draft_to_document(
        self: Arc<Self>,
        convert: ipc::ConvertDraftToDocument,
    ) -> impl Future<Output = Result<ipc::DraftDocument, Refusal>> + Send;

    /// `reopen_waiting`: the person lets an item he closed wait again. [`Refusal::IllegalMove`] where no
    /// closed item has that id, [`Refusal::Unacceptable`] where it would not wait again.
    fn reopen_waiting(
        self: Arc<Self>,
        reopen: ipc::ReopenWaiting,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `done_waiting`: the person did what an agent's item asked of him; the job is sent `Done: <text>` and his note.
    /// [`Refusal::IllegalMove`] where nothing holds the item, [`Refusal::Unacceptable`] for an item the agent did not state.
    fn done_waiting(
        self: Arc<Self>,
        done: ipc::DoneWaiting,
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

    /// `view_session`: a Job came on screen in a visible window, or left it. Answers the Job, with
    /// `viewed_at` moved to now.
    fn view_session(
        &self,
        asked: ipc::ViewSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `get_calls`: every call standing on the person, oldest first. It never refuses.
    fn get_calls(&self) -> impl Future<Output = Result<ipc::StandingCalls, Refusal>> + Send;

    /// `open_call`: he opened a main or pull request call. Answers the calls as they stand. A Job's
    /// call is opened by `view_session`, and a key that names no call standing opens nothing.
    fn open_call(
        &self,
        asked: ipc::OpenStandingCall,
    ) -> impl Future<Output = Result<ipc::StandingCalls, Refusal>> + Send;

    /// `pause_session`: ends the turn, parks the slot's work on its branch and gives the slot back.
    fn pause_session(
        &self,
        asked: ipc::PauseSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `snooze_session`: snoozes a Job until an instant, or wakes it now with none.
    fn snooze_session(
        &self,
        asked: ipc::SnoozeSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `set_session_auto_land`: turns Auto land on or off and answers the Session. The forge is asked
    /// after the answer, so a pull request it refuses is a notice to the Job, not a refusal here.
    fn set_session_auto_land(
        self: std::sync::Arc<Self>,
        asked: ipc::SetSessionAutoLand,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `set_session_project`: moves each Job into a project of his, or out to none, and answers the Jobs
    /// as they are now. Every Job is checked first, and a refusal writes nothing.
    fn set_session_project(
        &self,
        asked: ipc::SetSessionProject,
    ) -> impl Future<Output = Result<ipc::SessionList, Refusal>> + Send;

    /// `get_session_mcp`: the MCP servers a hosted job's process has, asked of it now.
    fn get_session_mcp(
        &self,
        id: SessionId,
    ) -> impl Future<Output = Result<ipc::SessionMcp, Refusal>> + Send;

    /// `get_session_quick`: the replies a Job's quick bar offers that no setting holds.
    fn get_session_quick(
        &self,
        id: SessionId,
    ) -> impl Future<Output = Result<ipc::SessionQuick, Refusal>> + Send;

    /// `get_job_artifacts`: a Job's sketches, drafts and pages, newest first.
    fn get_job_artifacts(
        &self,
        id: SessionId,
    ) -> impl Future<Output = Result<Vec<ipc::JobArtifact>, Refusal>> + Send;

    /// `archive_artifact`: the agent, or the person where `by` is `Owner`, folds away a stale artifact, taking
    /// one that waits on him off his list with no verdict. `caller` places a hosted session as `withdraw_sketch`'s
    /// does. Answers with the artifact. [`Refusal::Unacceptable`] for an id the Job does not hold, or one archived already.
    fn archive_artifact(
        self: Arc<Self>,
        caller: Option<crate::Caller>,
        by: ipc::ArtifactBy,
        archive: ipc::ArtifactRef,
    ) -> impl Future<Output = Result<ipc::JobArtifact, Refusal>> + Send;

    /// `restore_artifact`: lifts an archive, and the dismissal it wrote. Placed as `archive_artifact` is.
    /// [`Refusal::Unacceptable`] for an id the Job does not hold, or one not archived.
    fn restore_artifact(
        &self,
        caller: Option<crate::Caller>,
        restore: ipc::ArtifactRef,
    ) -> impl Future<Output = Result<ipc::JobArtifact, Refusal>> + Send;

    /// `save_artifact_edit`: his Edit of a page's or a draft's words, kept as the next revision, his, with the Job sent
    /// the diff first. Answers with the artifact. [`Refusal::Unacceptable`] for a sketch, an edit that is not words
    /// alone, or an artifact archived, settled or not held; [`Refusal::IllegalMove`] for a revision not the newest.
    fn save_artifact_edit(
        self: Arc<Self>,
        edit: ipc::SaveArtifactEdit,
    ) -> impl Future<Output = Result<ipc::JobArtifact, Refusal>> + Send;

    /// `press_page_action`: his press of an action a page's kept revision declares, run as a message, a start or an
    /// answer, and kept. Answers with the press. [`Refusal::Unacceptable`] for an action the revision does not hold,
    /// an act off the list, an ask no longer open, or a page archived or not held.
    fn press_page_action(
        self: Arc<Self>,
        press: ipc::PressPageAction,
    ) -> impl Future<Output = Result<ipc::PagePress, Refusal>> + Send;

    /// `cast_page_vote`: his vote on a vote a page's kept revision declares, sent to the Job with every note and
    /// kept. Answers with the cast. [`Refusal::Unacceptable`] for a vote the revision does not hold, a result of
    /// another form, a take or part the vote does not hold, or a page archived or not held.
    fn cast_page_vote(
        self: Arc<Self>,
        cast: ipc::CastPageVote,
    ) -> impl Future<Output = Result<ipc::VoteCast, Refusal>> + Send;

    /// `act_on_session_mcp`: reconnects, turns on or off, or signs in to one server, then reads
    /// the servers again.
    fn act_on_session_mcp(
        &self,
        asked: ipc::ActOnSessionMcp,
    ) -> impl Future<Output = Result<ipc::SessionMcpActed, Refusal>> + Send;

    /// `mend_session_tools`: fills Fleet's copy of the mod and restarts the Job's process, which
    /// resumes the conversation with the lines it had not finished. Answers once the restart is
    /// under way.
    fn mend_session_tools(
        self: std::sync::Arc<Self>,
        asked: ipc::MendSessionTools,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `start_again`: sends again the turns a Job's process would not start for.
    fn start_again(
        self: std::sync::Arc<Self>,
        asked: ipc::StartAgain,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `resume_session`: takes a slot again at the branch tip, or waits for one.
    fn resume_session(
        self: std::sync::Arc<Self>,
        asked: ipc::ResumeSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `set_session_workflow`: the person picked the Session's workflow in a panel, or cleared
    /// it. A Session that has run is told at once.
    fn set_session_workflow(
        self: Arc<Self>,
        set: SetSessionWorkflow,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;
    /// `skip_session_step`: the person skipped a step of the Session's workflow, or undid the skip.
    /// A Session that has run is told at once.
    fn skip_session_step(
        self: Arc<Self>,
        skip: ipc::SkipSessionStep,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;
    /// `jump_session_step`: the person moved the Session straight to a step to come, skipping the steps
    /// between. A Session that has run is told once.
    fn jump_session_step(
        self: Arc<Self>,
        jump: ipc::JumpSessionStep,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;
    /// `change_session_steps`: a step added, moved, reworded or dropped in the Job's own copy of its
    /// workflow, by the Job through the door or by the person. Applies at once, to that Job only.
    fn change_session_steps(
        self: Arc<Self>,
        change: ipc::ChangeSessionSteps,
    ) -> impl Future<Output = Result<ipc::JobCopy, Refusal>> + Send;
    /// `get_affected_jobs`: every live Job that follows the workflow in a repository the place reaches, and where it
    /// stands against what a publish there takes. Read-only.
    fn get_affected_jobs(
        self: Arc<Self>,
        workflow_id: ipc::WorkflowId,
        source: String,
        project: Option<String>,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::AffectedJobs, Refusal>> + Send;
    /// `migrate_jobs`: the person moved the Jobs he picked to the published workflow, one result for each in order. A
    /// Job that cannot move is refused in its own result, and the others move.
    fn migrate_jobs(
        self: Arc<Self>,
        migrate: ipc::MigrateJobs,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::MigratedJobs, Refusal>> + Send;
    /// `change_job_copy`: a lifecycle or a rule put in the Job's own copy, or taken out of it.
    fn change_job_copy(
        self: Arc<Self>,
        change: ipc::ChangeJobCopy,
    ) -> impl Future<Output = Result<ipc::JobCopy, Refusal>> + Send;
    /// `get_job_copy`: what the Job holds of its workflow, lifecycles and rules, and what was changed in
    /// it. The Job is the one the connection places, else `session_id`.
    fn get_job_copy(
        self: Arc<Self>,
        session_id: Option<SessionId>,
    ) -> impl Future<Output = Result<ipc::JobCopy, Refusal>> + Send;
    /// `keep_job_workflow`: the Job's copy of its workflow proposed for every Job, as a change kept ready
    /// for the owner to approve.
    fn keep_job_workflow(
        self: Arc<Self>,
        keep: ipc::KeepJobWorkflow,
    ) -> impl Future<Output = Result<ipc::WorkflowChange, Refusal>> + Send;
    /// `keep_job_piece`: one rule or lifecycle the Job changed in its copy, proposed for every Job as a file in
    /// Kit, waiting on the owner's Approve.
    fn keep_job_piece(
        self: Arc<Self>,
        keep: ipc::KeepJobPiece,
    ) -> impl Future<Output = Result<ipc::JobPieceProposed, Refusal>> + Send;
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

    /// `get_job_thread`: the Job's thread folded, as whole items. [`Refusal::Unacceptable`] for a `before` no
    /// item holds.
    fn get_job_thread(
        self: Arc<Self>,
        read: ipc::JobThreadRead,
    ) -> impl Future<Output = Result<ipc::JobThread, Refusal>> + Send;

    /// `get_session_subagent`: a subagent's own thread.
    fn get_session_subagent(
        &self,
        id: SessionId,
        subagent: String,
    ) -> impl Future<Output = Result<SessionSubagent, Refusal>> + Send;

    /// `get_repository_file`: a file of the Job, as text or a picture.
    fn get_repository_file(
        &self,
        id: SessionId,
        path: String,
    ) -> impl Future<Output = Result<ipc::RepositoryFile, Refusal>> + Send;

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
