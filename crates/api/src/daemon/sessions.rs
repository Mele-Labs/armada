//! The session ledger: what a harness reports, and who holds what. Since 23.43.
//! `docs/concepts/session.md`.
//!
//! **Its own surface**, for [`Authoring`](super::Authoring)'s reason: it is whole
//! on its own, and a handler that takes only this cannot reach for a Job. The
//! intake and its two reads share one trait because they share one store of
//! rows.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    ManifestId, Owners, RenameSession, SessionList, SessionRecord, SessionReport, SessionState,
};

use crate::daemon::Refusal;

/// What `get_shown_file` reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShownFile {
    Text(ipc::ShownText),
    Picture { media_type: String, bytes: Vec<u8> },
    /// Over what the phone is sent: answered 413 with its size.
    TooLarge { bytes: u64 },
}

pub trait Sessions: Send + Sync + 'static {
    /// `report_session` — record one fact about a session, creating its row if
    /// this is the first Fleet has heard of it. Answers with the row as it
    /// stands. [`Refusal::Unacceptable`] for a fact that names nothing: a blank
    /// session id or an attachment with no kind or target.
    fn report_session(
        &self,
        report: SessionReport,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `rename_session` — a person's name for a session, hosted or terminal.
    /// It stands until the next one. [`Refusal::Unacceptable`] for a blank
    /// title or a session Fleet does not know.
    fn rename_session(
        &self,
        rename: RenameSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `name_session` — Armada names a hosted session from its first message, as the
    /// proposer names one at the start. Not kept as the person's rename.
    /// [`Refusal::Unacceptable`] for a session Fleet does not know, one that is not hosted,
    /// one with no message, or a proposer that is missing or failed.
    fn name_session(
        &self,
        name: ipc::NameSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `start_session_run` — run a shell block from a Session's thread, in a terminal of its own.
    /// Answers at once with the run underway. [`Refusal::Unacceptable`] for a blank command, a
    /// Session Fleet has no row for, or one that has ended. **Takes the `Arc`** because the Session
    /// is told how the run ended, long after this call has answered.
    fn start_session_run(
        self: std::sync::Arc<Self>,
        start: ipc::StartSessionRun,
    ) -> impl Future<Output = Result<ipc::SessionRunUnderway, Refusal>> + Send;

    /// `run_session_shell` — run a shell action in the worktree slot a Session holds and answer
    /// when it has ended. [`Refusal::Unacceptable`] for a blank command, a Session Fleet has no row
    /// for, one that has ended, or one that holds no slot. **Takes the `Arc`** for
    /// `start_session_run`'s reason: the Session is told how it ended.
    fn run_session_shell(
        self: std::sync::Arc<Self>,
        run: ipc::RunSessionShell,
    ) -> impl Future<Output = Result<ipc::SessionShellRan, Refusal>> + Send;

    /// `stop_session_run` — end a run's process group. [`Refusal::Unacceptable`] for a run that
    /// is not in flight.
    fn stop_session_run(
        &self,
        stop: ipc::StopSessionRun,
    ) -> impl Future<Output = Result<ipc::SessionRunUnderway, Refusal>> + Send;

    /// `write_session_run` — keys typed into a run's terminal. [`Refusal::Unacceptable`] for a run
    /// that has ended or Fleet does not hold, as a stop is.
    fn write_session_run(
        &self,
        write: ipc::WriteSessionRun,
    ) -> impl Future<Output = Result<ipc::SessionRunUnderway, Refusal>> + Send;

    /// `resize_session_run` — the run's terminal changed size. [`Refusal::Unacceptable`] as
    /// `write_session_run`'s, and for a size of no columns or no rows.
    fn resize_session_run(
        &self,
        resize: ipc::ResizeSessionRun,
    ) -> impl Future<Output = Result<ipc::SessionRunUnderway, Refusal>> + Send;

    /// `observe_session_run` — a run's output on a socket of its own. [`Refusal::Unacceptable`]
    /// for a run Fleet does not hold.
    fn observe_session_run(
        &self,
        run_id: String,
    ) -> impl Future<Output = Result<crate::ObservedSessionRun, Refusal>> + Send;

    /// `open_terminal` — the Job's shell, opened where it has none: one a Job, shared by the
    /// person and its agent. [`Refusal::Unacceptable`] as `start_session_run`'s. **Takes the `Arc`**
    /// because the shell's end is seen long after this call has answered.
    fn open_terminal(
        self: std::sync::Arc<Self>,
        open: ipc::OpenTerminal,
    ) -> impl Future<Output = Result<ipc::SessionRunUnderway, Refusal>> + Send;

    /// `start_terminal_run` — a command run in a directory with no Session behind it. Nobody is
    /// told how it ended. [`Refusal::Unacceptable`] for a blank command, a directory that does not
    /// exist, or a size with no cells.
    fn start_terminal_run(
        self: std::sync::Arc<Self>,
        start: ipc::StartTerminalRun,
    ) -> impl Future<Output = Result<ipc::SessionRunUnderway, Refusal>> + Send;

    /// `type_in_terminal` — the agent types into its Job's shell, opening it where it has none, and
    /// is answered what it printed once it went quiet. `caller` places a hosted Job as
    /// `show_window`'s does. [`Refusal::Unacceptable`] for a call that places no Job, and as
    /// `open_terminal`'s.
    fn type_in_terminal(
        self: std::sync::Arc<Self>,
        caller: Option<crate::Caller>,
        typed: ipc::TypeInTerminal,
    ) -> impl Future<Output = Result<ipc::TerminalTyped, Refusal>> + Send;

    /// `read_terminal` — the last `lines` of the Job's shell as its screen drew them, once `until`
    /// was printed or its wait ran out. `caller` places the Job as `type_in_terminal`'s does.
    /// [`Refusal::Unacceptable`] for a call that places no Job, a Job with no shell open, or an
    /// `until` that is no regular expression.
    fn read_terminal(
        &self,
        caller: Option<crate::Caller>,
        asked: ipc::ReadTerminal,
    ) -> impl Future<Output = Result<ipc::TerminalScreen, Refusal>> + Send;

    /// `show_window` — a session shows the person a page. `caller` is the connection the call
    /// arrived on, where it came through the agent's door: it places a hosted session. Answers with
    /// the row. [`Refusal::Unacceptable`] for an address that is not `http` or `https`, or a call
    /// that places no session.
    fn show_window(
        &self,
        caller: Option<crate::Caller>,
        show: ipc::ShowWindow,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `withdraw_window` — a session takes back a walk it showed, so he is not asked to walk it and it
    /// holds no plan. `caller` places a hosted session as `show_window`'s does. Answers with the row.
    /// [`Refusal::Unacceptable`] for an address this session never showed a window on, or a call that
    /// places no session.
    fn withdraw_window(
        &self,
        caller: Option<crate::Caller>,
        withdraw: ipc::WithdrawWindow,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `show_sketch` — a session draws the person a diagram, kept on its ledger. `caller` places a
    /// hosted session as `show_window`'s does. Answers with the row. [`Refusal::Unacceptable`]
    /// with every rule the scene breaks, or for a call that places no session.
    fn show_sketch(
        &self,
        caller: Option<crate::Caller>,
        show: ipc::ShowSketch,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `record_plan` — a session records its plan, as a new version where it has one. `caller` places a hosted
    /// session as `show_window`'s does. `seen` is the agent's word that the person would see the change, so a
    /// walk follows approval; the route reads it beside the plan, and it holds nothing. Answers with the row.
    /// [`Refusal::Unacceptable`] with every rule the plan breaks, or for a call that places no session.
    fn record_plan(
        &self,
        caller: Option<crate::Caller>,
        plan: ipc::RecordPlan,
        seen: bool,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `approve_plan` — the person approves the plan at the version he read, and the agent is told once.
    /// [`Refusal::IllegalMove`] for a version that is no longer the current one, or a session with no plan.
    fn approve_plan(
        self: std::sync::Arc<Self>,
        approve: ipc::ApprovePlan,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `send_plan_changes` — the marks the person made on the plan go to its agent as one message; the plan
    /// itself does not change. [`Refusal::IllegalMove`] for a version that is no longer current,
    /// [`Refusal::Unacceptable`] for a mark that names nothing the plan holds.
    fn send_plan_changes(
        self: std::sync::Arc<Self>,
        changes: ipc::SendPlanChanges,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `judge_session` — the person asks for a judge to read the Session's work now. **Answers at once**, `judging`
    /// set; the reading runs in a task. [`Refusal::IllegalMove`] for a Session that ended,
    /// [`Refusal::Unacceptable`] for a terminal one.
    fn judge_session(
        self: std::sync::Arc<Self>,
        judge: ipc::JudgeSession,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `send_concern` — a judge's concern goes to the agent as the person's message, and is settled.
    /// [`Refusal::IllegalMove`] for an item that does not wait.
    fn send_concern(
        self: std::sync::Arc<Self>,
        send: ipc::SendConcern,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `dismiss_concern` — a judge's concern is settled and the agent is told nothing.
    /// [`Refusal::IllegalMove`] for an item that does not wait.
    fn dismiss_concern(
        self: std::sync::Arc<Self>,
        dismiss: ipc::DismissConcern,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `show_file` — a session puts a file in front of the person, kept on its ledger. `caller` places
    /// a hosted session as `show_window`'s does. Answers with the row. [`Refusal::Unacceptable`]
    /// for a path that is not a readable file of the size Bridge opens, or a call that places no session.
    fn show_file(
        &self,
        caller: Option<crate::Caller>,
        show: ipc::ShowFile,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `get_shown_file` — a file the session showed, read for the phone: text, or a picture's bytes.
    /// [`Refusal::NotFound`] for a session Fleet does not know, a path it never showed, or a file
    /// gone since; [`Refusal::Unacceptable`] for one that is neither text nor a picture.
    fn get_shown_file(
        &self,
        id: ipc::SessionId,
        path: String,
    ) -> impl Future<Output = Result<ShownFile, Refusal>> + Send;

    /// `report_session_check` — `armada check` tells Fleet a run began or ended in a directory.
    /// Answers the run, or none where no Session holds the slot that directory is in.
    /// [`Refusal::Unacceptable`] for a start with no name or an end with no run or outcome.
    fn report_session_check(
        &self,
        call: ipc::SessionCheckCall,
    ) -> impl Future<Output = Result<ipc::SessionCheckAnswer, Refusal>> + Send;

    /// `ask_for_slot` — a Session's `armada worktree lease` found every slot held. Raises the
    /// Session's `slot_choice` call, or keeps the one it has, and answers where the person's choice
    /// stands. Never a refusal for a Session Fleet does not host: that answers `unasked`.
    fn ask_for_slot(
        &self,
        call: ipc::AskForSlot,
    ) -> impl Future<Output = Result<ipc::SlotAsked, Refusal>> + Send;

    /// `get_session_check_output` — one Session run's log. [`Refusal::Unacceptable`] for a run no
    /// Session kept.
    fn get_session_check_output(
        &self,
        run: u64,
    ) -> impl Future<Output = Result<ipc::CheckOutput, Refusal>> + Send;
    /// `observe_session_check` — one Session run's log as it is written. Resolved before the socket
    /// opens: [`Refusal::Unacceptable`] for a run no Session kept.
    fn observe_session_check(
        &self,
        run: u64,
    ) -> impl Future<Output = Result<crate::SessionCheckLive, Refusal>> + Send;

    /// `stop_session_check` and `force_stop_session_check` — a person asks a running Session Check
    /// to stop: `SIGTERM` to its group, or `SIGKILL` where `force`. Answers the stop now standing.
    /// [`Refusal::Unacceptable`] for a run that is not running.
    fn stop_session_check(
        &self,
        run: u64,
        force: bool,
    ) -> impl Future<Output = Result<ipc::SessionCheckAnswer, Refusal>> + Send;

    /// `ask_in_window` — a session asks the person about a place on the page its walk window shows.
    /// `caller` places a hosted session as `show_window`'s does. Answers with the row at once.
    /// [`Refusal::Unacceptable`] for an address on no window the session showed, a blank question
    /// or selector, or a call that places no session.
    fn ask_in_window(
        &self,
        caller: Option<crate::Caller>,
        ask: ipc::AskInWindow,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `propose_draft` — a session puts a draft to the person for approval. `caller` places a hosted
    /// session as `show_window`'s does. Answers with the row at once, its `draft:<row id>` item
    /// waiting. [`Refusal::Unacceptable`] for a blank title or text, or a call that places no session.
    fn propose_draft(
        &self,
        caller: Option<crate::Caller>,
        draft: ipc::ProposeDraft,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `withdraw_draft` — a session takes back a draft it proposed, settling its row `withdrawn` so it
    /// waits no more. `caller` places a hosted session as `show_window`'s does. Answers with the row.
    /// [`Refusal::Unacceptable`] for a draft that is not this session's or not open, or a call that
    /// places no session.
    fn withdraw_draft(
        &self,
        caller: Option<crate::Caller>,
        withdraw: ipc::WithdrawDraft,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `window_closed` — the person closed a session's window, which ends its walk. Answers with
    /// the row. [`Refusal::Unacceptable`] for a session Fleet does not know.
    fn window_closed(
        &self,
        closed: ipc::WindowClosed,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `take_window_sign_in` — Bridge, opening a session's window, takes the sign-in addresses held for it.
    /// Once: the next take finds none. Answers empty where none are held.
    fn take_window_sign_in(
        &self,
        take: ipc::TakeWindowSignIn,
    ) -> impl Future<Output = Result<ipc::WindowSignIn, Refusal>> + Send;

    /// `window_failed` — a session's window could not show its page, and the session is told in
    /// Fleet's voice. [`Refusal::Unacceptable`] for a session Fleet does not know.
    fn window_failed(
        self: std::sync::Arc<Self>,
        failed: ipc::WindowFailed,
    ) -> impl Future<Output = Result<ipc::WalkTold, Refusal>> + Send;

    /// `claim_pull_request` — the caller takes an open pull request no live Session or Job holds,
    /// or whose holder has ended. [`Refusal::Unacceptable`] with the reason for a pull request
    /// that is not open, a live holder, or a call that places no holder.
    fn claim_pull_request(
        &self,
        caller: Option<crate::Caller>,
        claim: ipc::ClaimPullRequest,
    ) -> impl Future<Output = Result<ipc::PullRequestClaimed, Refusal>> + Send;
    /// `claim_issue` — the caller works an issue, named by the forge's number or a tracker's key,
    /// held as an `issue` row on its ledger with the pull requests that work it.
    /// [`Refusal::Unacceptable`] for a reference that is neither, or a call that places no holder.
    fn claim_issue(
        &self,
        caller: Option<crate::Caller>,
        claim: ipc::ClaimIssue,
    ) -> impl Future<Output = Result<ipc::IssueClaimed, Refusal>> + Send;
    /// `take_main_red` — a Session takes its repository's red main in the central store, so no
    /// Job on another machine is handed it. [`Refusal::Unacceptable`] for a call that places no
    /// Session; [`Refusal::IllegalMove`] with the holder when the red is held elsewhere, main is
    /// not red or a Job here is already working on it; [`Refusal::Fault`] when the store would not answer.
    fn take_main_red(
        &self,
        caller: Option<crate::Caller>,
        take: ipc::TakeMainRed,
    ) -> impl Future<Output = Result<ipc::MainRedTaken, Refusal>> + Send;
    /// `start_another_session` — a hosted Session's agent starts a Session with a first message held for the
    /// person. `caller` places the Session, which the body cannot name. No process starts.
    /// [`Refusal::Unacceptable`] for a call that places no hosted Session, a blank title or message, or
    /// a repository Fleet does not serve.
    fn start_another_session(
        self: Arc<Self>,
        caller: Option<crate::Caller>,
        start: ipc::StartAnotherSession,
    ) -> impl Future<Output = Result<ipc::SessionHeld, Refusal>> + Send;

    /// `change_waiting` — when a waiting Session starts, or where from. **The agent of the Session
    /// waited on changes a Session that waits on it; the person (no `caller`) changes any.**
    /// [`Refusal::Unacceptable`] for a call that places no Session, a Session that does not wait on the
    /// caller, or a plan part the Session waited on does not hold; [`Refusal::IllegalMove`] where the
    /// Session has started.
    fn change_waiting(
        self: Arc<Self>,
        caller: Option<crate::Caller>,
        change: ipc::ChangeWaiting,
    ) -> impl Future<Output = Result<ipc::SessionWait, Refusal>> + Send;

    /// `fork_this_session` — a hosted Session's agent forks its own conversation, whole or at a message,
    /// with a first message held for the person. [`Refusal::Unacceptable`] as `start_another_session`'s,
    /// and for a message the conversation does not hold.
    fn fork_this_session(
        &self,
        caller: Option<crate::Caller>,
        fork: ipc::ForkThisSession,
    ) -> impl Future<Output = Result<ipc::SessionHeld, Refusal>> + Send;

    /// `waiting_for` — the agent sets the whole list of what it needs from the person. `caller`
    /// places a hosted session as `show_window`'s does. Answers with the row.
    /// [`Refusal::Unacceptable`] for a call that places no session or an item with no id or text.
    fn waiting_for(
        &self,
        caller: Option<crate::Caller>,
        set: ipc::SetWaitingFor,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `all_done` — the agent says it has nothing left to work on. `caller` places a hosted session as
    /// `waiting_for`'s does. Answers with the row. [`Refusal::Unacceptable`] for a call that places no session.
    fn all_done(
        &self,
        caller: Option<crate::Caller>,
        done: ipc::AllDone,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `defer_later` — the agent keeps a follow-up for itself, as a sentence. `caller` places a hosted
    /// session as `all_done`'s does. [`Refusal::Unacceptable`] for an empty sentence or a call that places no
    /// session; [`Refusal::IllegalMove`] for a session that has ended.
    fn defer_later(
        &self,
        caller: Option<crate::Caller>,
        defer: ipc::DeferLater,
    ) -> impl Future<Output = Result<ipc::LaterDeferred, Refusal>> + Send;

    /// `settle_later` — a follow-up the session kept for later is settled as done, filed or dropped,
    /// by the agent or by the person. `caller` places a hosted session as `all_done`'s does.
    /// [`Refusal::Unacceptable`] for a filed with no issue, an agent's drop with no reason, or a call
    /// that places no session; [`Refusal::IllegalMove`] for a follow-up not open.
    fn settle_later(
        &self,
        caller: Option<crate::Caller>,
        settle: ipc::SettleLater,
    ) -> impl Future<Output = Result<ipc::LaterSettled, Refusal>> + Send;

    /// `dispute_fleet` — the agent says the session's latest Fleet notice was wrong. `caller` places a
    /// hosted session as `all_done`'s does. [`Refusal::Unacceptable`] for a call that places no hosted
    /// session, an empty `why`, or a session Fleet has told nothing.
    fn dispute_fleet(
        self: std::sync::Arc<Self>,
        caller: Option<crate::Caller>,
        dispute: ipc::DisputeFleet,
    ) -> impl Future<Output = Result<ipc::FleetDisputed, Refusal>> + Send;

    /// `finish_up` — the agent gives back the slots, merged branches and servers it is finished with.
    /// `caller` places a hosted session as `all_done`'s does. [`Refusal::Unacceptable`] for a call that places
    /// no session, and a refusal for a session in a terminal.
    fn finish_up(
        &self,
        caller: Option<crate::Caller>,
        finish: ipc::FinishUp,
    ) -> impl Future<Output = Result<ipc::FinishedUp, Refusal>> + Send;

    /// `mark_annotation_done` — the agent marks done a note Bridge sent it. `caller` places a hosted
    /// session as `finish_up`'s does. [`Refusal::Unacceptable`] for a note id that is not one, a note
    /// not found, or one sent to another session.
    fn mark_annotation_done(
        &self,
        caller: Option<crate::Caller>,
        mark: ipc::MarkAnnotationDone,
    ) -> impl Future<Output = Result<ipc::AnnotationMarked, Refusal>> + Send;

    /// `resolve_pull_request_note` — the agent marks done a pull request note Bridge sent it.
    /// `caller` places a hosted session as `finish_up`'s does. [`Refusal::Unacceptable`] for a note
    /// not found, not sent to this session, or already resolved.
    fn resolve_pull_request_note(
        &self,
        caller: Option<crate::Caller>,
        resolve: ipc::ResolvePullRequestNote,
    ) -> impl Future<Output = Result<ipc::PullRequestNote, Refusal>> + Send;

    /// `list_sessions` — the most recently seen first. `manifest_id` absent is
    /// every session on the machine, `text` finds one by title, branch, pull
    /// request, Job or slot, and `state` absent is both.
    fn list_sessions(
        &self,
        manifest_id: Option<ManifestId>,
        text: Option<String>,
        state: Option<SessionState>,
    ) -> impl Future<Output = Result<SessionList, Refusal>> + Send;

    /// `who_owns` — every holder of `kind` at `target`, the standing ones first.
    /// [`Refusal::Unacceptable`] for a blank `kind` or `target`.
    fn who_owns(
        &self,
        kind: String,
        target: String,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<Owners, Refusal>> + Send;

    /// `recall` — the Logbook entries for `manifest_id`'s repository (absent is the first served),
    /// the habits and this Mac's that match `query`, faded ones included, best first. `section` is
    /// `repo`, `habits` or `machine`; any other is [`Refusal::Unacceptable`], as is a blank `query`.
    fn recall(
        &self,
        manifest_id: Option<ManifestId>,
        query: String,
        section: Option<String>,
    ) -> impl Future<Output = Result<ipc::Recalled, Refusal>> + Send;

    /// `get_logbook_for_subagent` — what a subagent starting under `session_id` is handed: the
    /// habits and the standing entries whose paths match those `prompt` names. Empty where there
    /// is none, and for a Session Fleet does not know.
    fn get_logbook_for_subagent(
        &self,
        session_id: String,
        prompt: String,
    ) -> impl Future<Output = Result<ipc::ForSubagent, Refusal>> + Send;

    /// `get_session_usage` — a session's turns from `since` to before `until` (RFC3339, either may
    /// be left out), oldest first. [`Refusal::Unacceptable`] for a session Fleet does not know.
    fn get_session_usage(
        &self,
        session_id: ipc::SessionId,
        since: Option<String>,
        until: Option<String>,
    ) -> impl Future<Output = Result<ipc::SessionTurnUsage, Refusal>> + Send;

    /// `get_usage_summary` — the turns in `[since, until)` summed in cost units. `until` is now and
    /// `since` a week before it where left out; `tz_offset_minutes` cuts the local days.
    /// [`Refusal::Unacceptable`] for a bound that is not a time.
    fn get_usage_summary(
        &self,
        since: Option<String>,
        until: Option<String>,
        tz_offset_minutes: i32,
    ) -> impl Future<Output = Result<ipc::UsageSummary, Refusal>> + Send;

    /// `get_usage_rules` — the token rules with their limits and the trips in `[since, until)`.
    /// `until` is now and `since` a week before it where left out. [`Refusal::Unacceptable`] for a
    /// bound that is not a time.
    fn get_usage_rules(
        &self,
        since: Option<String>,
        until: Option<String>,
    ) -> impl Future<Output = Result<ipc::UsageRules, Refusal>> + Send;

    /// `get_session_loadout` — what a session's agent knows, was told and used, the last fifty turns.
    /// [`Refusal::Unacceptable`] for a session Fleet does not know.
    fn get_session_loadout(
        &self,
        session_id: ipc::SessionId,
    ) -> impl Future<Output = Result<ipc::SessionLoadout, Refusal>> + Send;

    /// `get_attachment_history` — every version of everything a session holds, oldest first.
    /// [`Refusal::Unacceptable`] for a session Fleet does not know.
    fn get_attachment_history(
        &self,
        session_id: ipc::SessionId,
    ) -> impl Future<Output = Result<ipc::AttachmentHistory, Refusal>> + Send;

    /// `read_loadout_item` — what one row of a session's Loadout says, read from disk where Fleet
    /// runs. A row the Loadout does not list, or with no file behind it, is `not_read` with why.
    /// [`Refusal::Unacceptable`] for a session Fleet does not know.
    fn read_loadout_item(
        &self,
        asked: ipc::LoadoutItemAsked,
    ) -> impl Future<Output = Result<ipc::LoadoutItemBody, Refusal>> + Send;
}
