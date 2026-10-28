//! What a pull request's panel reads when it opens, and the one write its
//! comment box makes. Since 23.83.
//!
//! Beside [`PullRequests`](super::PullRequests), which answers the pull
//! request's standing and rides every `session.changed`: a conversation is
//! read when somebody looks, not carried.

use std::future::Future;

use ipc::{
    ForgetReviewSearch, ManifestId, OpenReviewWalk, PullRequestFilePatch, PullRequestFilePatches, PullRequestPage, PullReviewLists,
    PullThreadResolved, RememberedPlace, RememberedPlaces, ResolvePullThread, ReviewRun, ReviewRuns, ReviewTools,
    ReviewWalk, SaveReviewTool, SaveWalkPlace, StartReviewRun, StartReviewSearch, WalkPlaceFound, WritePullRequest,
    PullRequestNote, PullRequestNotes, SavePullRequestNote, SendPullRequestNotes,
};

use crate::daemon::Refusal;

pub trait PullRequestPanel: Send + Sync + 'static {
    /// `get_pull_request_detail` — the conversation, checks, files and commits
    /// as the forge shows them. [`Refusal::Fault`] where it would not answer.
    fn get_pull_request_detail(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<PullRequestPage, Refusal>> + Send;

    /// `get_pull_request_file` — one file's patch, read when a person opens it.
    /// [`Refusal::Fault`] where the forge would not answer,
    /// [`Refusal::Unacceptable`] for a path the pull request does not list.
    fn get_pull_request_file(
        &self,
        manifest_id: ManifestId,
        number: u64,
        path: String,
    ) -> impl Future<Output = Result<PullRequestFilePatch, Refusal>> + Send;

    /// `get_pull_request_patches` — every file's patch, in the forge's order.
    /// [`Refusal::Fault`] where the forge would not answer.
    fn get_pull_request_patches(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<PullRequestFilePatches, Refusal>> + Send;

    /// `write_pull_request` — a comment, an approval or a request for changes.
    /// **A write to a shared place, a person's ask.** Answers the page read
    /// again afterwards. [`Refusal::Unacceptable`] for a comment or a request
    /// with no body, and for a Session the ledger does not know.
    fn write_pull_request(
        &self,
        manifest_id: ManifestId,
        write: WritePullRequest,
    ) -> impl Future<Output = Result<PullRequestPage, Refusal>> + Send;

    /// `get_review_lists` — the pull requests waiting on the person, as four lists read from
    /// the forge across every served repository. **Read when somebody looks, never polled.**
    /// [`Refusal::Fault`] where no list could be read, or no login is known.
    fn get_review_lists(&self) -> impl Future<Output = Result<PullReviewLists, Refusal>> + Send;

    /// `resolve_review_thread` — resolve a conversation on a pull request's diff, or open it
    /// again. **A write to a shared place, taken at once**, so a person's press.
    /// [`Refusal::IllegalMove`] with the forge's own sentence where it would not take it.
    fn resolve_review_thread(
        &self,
        manifest_id: ManifestId,
        resolve: ResolvePullThread,
    ) -> impl Future<Output = Result<PullThreadResolved, Refusal>> + Send;

    /// `list_review_tools` — the review tools and explanations a repository has, Kit's
    /// over the repository's, which is over Armada's, and the files left out and why. Read from the folders each time.
    fn list_review_tools(
        &self,
        manifest_id: ManifestId,
    ) -> impl Future<Output = Result<ReviewTools, Refusal>> + Send;

    /// `save_review_tool` — write one definition in Kit's folder or the repository's.
    /// [`Refusal::Unacceptable`] for a blank or wordless name, a blank prompt, and an id the
    /// scope already defines unless `overwrite`; [`Refusal::Fault`] where it cannot be written.
    fn save_review_tool(
        &self,
        manifest_id: ManifestId,
        save: SaveReviewTool,
    ) -> impl Future<Output = Result<ReviewTools, Refusal>> + Send;

    /// `start_review_run` — ask an agent, read-only, about a pull request. **Answers at once**:
    /// a kept result, the run already going, or a new one queued whose work goes on a task of
    /// its own. [`Refusal::Unacceptable`] for a name no definition has or a blank question;
    /// [`Refusal::Fault`] where the forge would not give the pull request.
    fn start_review_run(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        start: StartReviewRun,
    ) -> impl Future<Output = Result<ReviewRun, Refusal>> + Send;

    /// `get_review_runs` — every run kept or going for a pull request, newest first.
    fn get_review_runs(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<ReviewRuns, Refusal>> + Send;

    /// `start_review_search` — Natural Search: the person's words put to every open pull request
    /// of the served repositories. **Answers at once**, as `start_review_run` does.
    /// [`Refusal::Unacceptable`] for blank or over-long words; [`Refusal::Fault`] where the forge
    /// would not list the open pull requests.
    fn start_review_search(
        self: std::sync::Arc<Self>,
        start: StartReviewSearch,
    ) -> impl Future<Output = Result<ReviewRun, Refusal>> + Send;

    /// `get_review_searches` — every search kept or going, newest first.
    fn get_review_searches(&self) -> impl Future<Output = Result<ReviewRuns, Refusal>> + Send;

    /// `forget_review_search` — drop a kept search. Answers the searches kept now.
    fn forget_review_search(
        &self,
        forget: ForgetReviewSearch,
    ) -> impl Future<Output = Result<ReviewRuns, Refusal>> + Send;

    /// `find_walk_place` — where a pull request's change runs, from the pull request and then from
    /// what Fleet remembers. **Reads and spends nothing.**
    fn find_walk_place(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<WalkPlaceFound, Refusal>> + Send;

    /// `list_walk_places` — every place Fleet remembers a change runs.
    fn list_walk_places(&self) -> impl Future<Output = Result<RememberedPlaces, Refusal>> + Send;

    /// `save_walk_place` — remember where changes to an area run. [`Refusal::Unacceptable`] for a
    /// place that cannot be walked or an area that is not a directory of the repository.
    fn save_walk_place(
        &self,
        manifest_id: ManifestId,
        save: SaveWalkPlace,
    ) -> impl Future<Output = Result<RememberedPlace, Refusal>> + Send;

    /// `forget_walk_place` — stop remembering a place. Answers what is kept now.
    fn forget_walk_place(
        &self,
        manifest_id: ManifestId,
        id: String,
    ) -> impl Future<Output = Result<RememberedPlaces, Refusal>> + Send;

    /// `list_pull_request_notes` — the owner's notes on one pull request, oldest first.
    fn list_pull_request_notes(
        &self,
        manifest_id: ManifestId,
        number: u32,
    ) -> impl Future<Output = Result<PullRequestNotes, Refusal>> + Send;

    /// `save_pull_request_note` — make a note, or edit the body of one open and unsent.
    /// [`Refusal::Unacceptable`] for a blank body or a note that cannot be edited.
    fn save_pull_request_note(
        &self,
        manifest_id: ManifestId,
        number: u32,
        save: SavePullRequestNote,
    ) -> impl Future<Output = Result<PullRequestNote, Refusal>> + Send;

    /// `remove_pull_request_note` — answers the notes kept on the pull request now.
    fn remove_pull_request_note(
        &self,
        manifest_id: ManifestId,
        number: u32,
        id: String,
    ) -> impl Future<Output = Result<PullRequestNotes, Refusal>> + Send;

    /// `send_pull_request_notes` — record that notes were sent to a Job. **Sends nothing.**
    fn send_pull_request_notes(
        &self,
        manifest_id: ManifestId,
        number: u32,
        send: SendPullRequestNotes,
    ) -> impl Future<Output = Result<PullRequestNotes, Refusal>> + Send;

    /// `open_review_walk` — hold up the place a pull request's change runs. **Answers at once**;
    /// a pulled-down place's progress arrives as `review_walk.changed`.
    fn open_review_walk(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        open: OpenReviewWalk,
    ) -> impl Future<Output = Result<ReviewWalk, Refusal>> + Send;

    /// `close_review_walk` — stop a held walk's server and give its bay back. Idempotent.
    fn close_review_walk(
        &self,
        manifest_id: ManifestId,
        id: String,
    ) -> impl Future<Output = Result<ReviewWalk, Refusal>> + Send;
}
