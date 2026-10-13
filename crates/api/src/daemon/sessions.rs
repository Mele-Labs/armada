//! The session ledger: what a harness reports, and who holds what. Since 23.43.
//! `docs/concepts/session.md`.
//!
//! **Its own surface**, for [`Authoring`](super::Authoring)'s reason: it is whole
//! on its own, and a handler that takes only this cannot reach for a Job. The
//! intake and its two reads share one trait because they share one store of
//! rows.

use std::future::Future;

use ipc::{
    ManifestId, Owners, RenameSession, SessionList, SessionRecord, SessionReport, SessionState,
};

use crate::daemon::Refusal;

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

    /// `start_session_run` — run a shell block from a Session's thread. Answers at once with the
    /// run underway. [`Refusal::Unacceptable`] for a blank command, a Session Fleet has no row
    /// for, or one that has ended.
    fn start_session_run(
        &self,
        start: ipc::StartSessionRun,
    ) -> impl Future<Output = Result<ipc::SessionRunUnderway, Refusal>> + Send;

    /// `stop_session_run` — end a run's process group. [`Refusal::Unacceptable`] for a run that
    /// is not in flight.
    fn stop_session_run(
        &self,
        stop: ipc::StopSessionRun,
    ) -> impl Future<Output = Result<ipc::SessionRunUnderway, Refusal>> + Send;

    /// `observe_session_run` — a run's output on a socket of its own. [`Refusal::Unacceptable`]
    /// for a run Fleet does not hold.
    fn observe_session_run(
        &self,
        run_id: String,
    ) -> impl Future<Output = Result<crate::ObservedSessionRun, Refusal>> + Send;

    /// `show_window` — a session shows the person a page. `caller` is the connection the call
    /// arrived on, where it came through the agent's door: it places a hosted session. Answers with
    /// the row. [`Refusal::Unacceptable`] for an address that is not `http` or `https`, or a call
    /// that places no session.
    fn show_window(
        &self,
        caller: Option<crate::Caller>,
        show: ipc::ShowWindow,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `report_session_check` — `armada check` tells Fleet a run began or ended in a directory.
    /// Answers the run, or none where no Session holds the slot that directory is in.
    /// [`Refusal::Unacceptable`] for a start with no name or an end with no run or outcome.
    fn report_session_check(
        &self,
        call: ipc::SessionCheckCall,
    ) -> impl Future<Output = Result<ipc::SessionCheckAnswer, Refusal>> + Send;

    /// `get_session_check_output` — one Session run's log. [`Refusal::Unacceptable`] for a run no
    /// Session kept.
    fn get_session_check_output(
        &self,
        run: u64,
    ) -> impl Future<Output = Result<ipc::CheckOutput, Refusal>> + Send;
    /// `ask_in_window` — a session asks the person about a place on the page its walk window shows.
    /// `caller` places a hosted session as `show_window`'s does. Answers with the row at once.
    /// [`Refusal::Unacceptable`] for an address on no window the session showed, a blank question
    /// or selector, or a call that places no session.
    fn ask_in_window(
        &self,
        caller: Option<crate::Caller>,
        ask: ipc::AskInWindow,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `window_closed` — the person closed a session's window, which ends its walk. Answers with
    /// the row. [`Refusal::Unacceptable`] for a session Fleet does not know.
    fn window_closed(
        &self,
        closed: ipc::WindowClosed,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `claim_pull_request` — the caller takes an open pull request no live Session or Job holds,
    /// or whose holder has ended. [`Refusal::Unacceptable`] with the reason for a pull request
    /// that is not open, a live holder, or a call that places no holder.
    fn claim_pull_request(
        &self,
        caller: Option<crate::Caller>,
        claim: ipc::ClaimPullRequest,
    ) -> impl Future<Output = Result<ipc::PullRequestClaimed, Refusal>> + Send;
    /// `start_another_session` — a hosted Session's agent starts a Session with a first message held for the
    /// person. `caller` places the Session, which the body cannot name. No process starts.
    /// [`Refusal::Unacceptable`] for a call that places no hosted Session, a blank title or message, or
    /// a repository Fleet does not serve.
    fn start_another_session(
        &self,
        caller: Option<crate::Caller>,
        start: ipc::StartAnotherSession,
    ) -> impl Future<Output = Result<ipc::SessionHeld, Refusal>> + Send;

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
}
