//! A repository's open issues, read from its tracker. #91,
//! `docs/concepts/tracker.md`.
//!
//! **Its own surface**, for [`PullRequests`](super::PullRequests)' reason: a
//! handler that takes only this cannot reach for a Job.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    Autopilot, AutopilotList, AutopilotProposal, AutopilotSettings, IssueList, IssueQueries, IssueRead,
    InstallTool, InstalledTool, ManifestId, ProposeAutopilot, SaveIssueQuery, SavedIssueQuery, SearchIssues, WriteIssueQuery,
    WrittenIssueQuery,
};

use crate::daemon::Refusal;

pub trait Issues: Send + Sync + 'static {
    /// `list_issues` — the open issues Fleet last read, or a fresh read where
    /// `refresh` asks for one or what is held is old. [`Refusal::Fault`] where
    /// the tracker would not answer and nothing is held.
    fn list_issues(
        &self,
        manifest_id: ManifestId,
        refresh: bool,
    ) -> impl Future<Output = Result<IssueList, Refusal>> + Send;

    /// `get_issue` — one issue by reference (`#214` or `214` on a forge, `ABC-123` on Jira),
    /// closed ones included, read from the tracker each time.
    /// [`Refusal::NotFound`] where there is none or the number is a pull request's,
    /// [`Refusal::Fault`] where the tracker would not answer.
    fn get_issue(
        &self,
        manifest_id: ManifestId,
        reference: String,
    ) -> impl Future<Output = Result<IssueRead, Refusal>> + Send;

    /// `search_issues` — a query in the tracker's own language, run once and not held.
    /// [`Refusal::Unacceptable`] with the tracker's words where it refuses the query.
    fn search_issues(
        &self,
        manifest_id: ManifestId,
        search: SearchIssues,
    ) -> impl Future<Output = Result<IssueList, Refusal>> + Send;

    /// `install_tool` — install the tool a refusal's `missing_tool` named, when a person pressed
    /// it. [`Refusal::Unacceptable`] for a tool Fleet does not install. **Whether it worked is the
    /// answer**, not a refusal: no Homebrew is a thing a person is told.
    fn install_tool(
        &self,
        manifest_id: ManifestId,
        install: InstallTool,
    ) -> impl Future<Output = Result<InstalledTool, Refusal>> + Send;

    /// `list_issue_queries` — the tracker's guide summary, or why it cannot be read, and the
    /// queries saved for the repository.
    fn list_issue_queries(
        &self,
        manifest_id: ManifestId,
    ) -> impl Future<Output = Result<IssueQueries, Refusal>> + Send;

    /// `write_issue_query` — one model call turns the person's words into a query, which is run
    /// before it is answered. **Keeps nothing.** [`Refusal::Fault`] where the call or its answer
    /// failed, [`Refusal::Unacceptable`] where the tracker refuses the query it wrote.
    fn write_issue_query(
        &self,
        manifest_id: ManifestId,
        write: WriteIssueQuery,
    ) -> impl Future<Output = Result<WrittenIssueQuery, Refusal>> + Send;

    /// `save_issue_query` — keep a query, or with an `id` edit one. **No model call**: the query is
    /// run first and one the tracker refuses is refused with its words.
    fn save_issue_query(
        &self,
        manifest_id: ManifestId,
        save: SaveIssueQuery,
    ) -> impl Future<Output = Result<SavedIssueQuery, Refusal>> + Send;

    /// `forget_issue_query` — drop a saved query; answers those left. [`Refusal::NotFound`] for an
    /// id the repository has none of.
    fn forget_issue_query(
        &self,
        manifest_id: ManifestId,
        id: String,
    ) -> impl Future<Output = Result<IssueQueries, Refusal>> + Send;
}

/// A repository working its own backlog. #2092, `docs/concepts/autopilot.md`.
///
/// **Beside [`Issues`] because it picks from that list**, and like it takes no Job: a pick that
/// starts one goes through Fleet's own drafting, never through a handler.
pub trait Autopilots: Send + Sync + 'static {
    /// `propose_autopilot`: one sentence read as settings, with the words each came from.
    /// **Starts nothing.**
    fn propose_autopilot(
        &self,
        manifest_id: ManifestId,
        propose: ProposeAutopilot,
    ) -> impl Future<Output = Result<AutopilotProposal, Refusal>> + Send;

    /// `start_autopilot`, with what the person confirmed. [`Refusal::IllegalMove`] where one is
    /// running or paused, [`Refusal::Unacceptable`] for settings nothing could run under.
    fn start_autopilot(
        &self,
        manifest_id: ManifestId,
        settings: AutopilotSettings,
    ) -> impl Future<Output = Result<Autopilot, Refusal>> + Send;

    /// `pause_autopilot`: no new picks; the Sessions it started carry on.
    fn pause_autopilot(
        &self,
        manifest_id: ManifestId,
    ) -> impl Future<Output = Result<Autopilot, Refusal>> + Send;

    /// `resume_autopilot`. [`Refusal::IllegalMove`] while a cap it reached still holds.
    fn resume_autopilot(
        &self,
        manifest_id: ManifestId,
    ) -> impl Future<Output = Result<Autopilot, Refusal>> + Send;

    /// `update_autopilot`: new settings for one running or paused, in its state. It never resumes
    /// one: a raised cap is what lets a person's Resume through.
    fn update_autopilot(
        &self,
        manifest_id: ManifestId,
        settings: AutopilotSettings,
    ) -> impl Future<Output = Result<Autopilot, Refusal>> + Send;

    /// `stop_autopilot`: no new picks, picks whose Session has not run closed, and the end.
    fn stop_autopilot(
        &self,
        manifest_id: ManifestId,
    ) -> impl Future<Output = Result<Autopilot, Refusal>> + Send;

    /// `list_autopilots`.
    fn list_autopilots(&self) -> impl Future<Output = Result<AutopilotList, Refusal>> + Send;

    /// `get_autopilot`. [`Refusal::IllegalMove`] where the repository has never had one.
    fn get_autopilot(
        &self,
        manifest_id: ManifestId,
    ) -> impl Future<Output = Result<Autopilot, Refusal>> + Send;

    /// `dispatch_set_aside`: a person dispatches a set-aside pick, and a Session starts on it.
    /// **Takes the `Arc`** because the Session's first message is sent from a task of its own.
    fn dispatch_set_aside(
        self: Arc<Self>,
        manifest_id: ManifestId,
        pick_id: String,
    ) -> impl Future<Output = Result<Autopilot, Refusal>> + Send;

    /// `dismiss_set_aside`.
    fn dismiss_set_aside(
        &self,
        manifest_id: ManifestId,
        pick_id: String,
    ) -> impl Future<Output = Result<Autopilot, Refusal>> + Send;
}
