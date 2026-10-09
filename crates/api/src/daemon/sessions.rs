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

    /// `claim_pull_request` — the caller takes an open pull request no live Session or Job holds,
    /// or whose holder has ended. [`Refusal::Unacceptable`] with the reason for a pull request
    /// that is not open, a live holder, or a call that places no holder.
    fn claim_pull_request(
        &self,
        caller: Option<crate::Caller>,
        claim: ipc::ClaimPullRequest,
    ) -> impl Future<Output = Result<ipc::PullRequestClaimed, Refusal>> + Send;
    /// `waiting_for` — the agent sets the whole list of what it needs from the person. `caller`
    /// places a hosted session as `show_window`'s does. Answers with the row.
    /// [`Refusal::Unacceptable`] for a call that places no session or an item with no id or text.
    fn waiting_for(
        &self,
        caller: Option<crate::Caller>,
        set: ipc::SetWaitingFor,
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
