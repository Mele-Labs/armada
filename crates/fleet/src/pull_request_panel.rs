//! What a pull request's panel reads when it opens, and the write its comment
//! box makes. Since 23.83. Beside `crate::pull_requesting`, whose read and
//! ledger follow this reuses: the standing rides every `session.changed`, and
//! the conversation is read here, when somebody looks.
//!
//! **A write is checked before the forge is reached**: a comment or a request
//! for changes with no body, and a Session the ledger has never heard of, are
//! refused with nothing written. After the write the forge is read again and
//! every Session's `pr` row follows it.

use adapter_traits::{
    AgentHarness, CheckStanding, Delivery, DiffSide as FactSide, FileChange, FilePatch, Mergeability, PatchedFile,
    PullRequestPageFacts, PullRequestWrite, ReviewComment as FactComment, ReviewSaid, ReviewStanding,
    ReviewerKind, Said, Vcs, WorkProduct,
};
use api::{PullRequestPanel, Refusal};
use ipc::{
    DiffSide, ManifestId, PullRequestCheckRow, PullRequestCheckState, PullRequestCommit, PullRequestEntry,
    PullRequestFile, PullRequestFilePatch, PullRequestFilePatches, PullRequestFileStatus, PullRequestMergeable, PullRequestPage,
    PullRequestReviewState, PullRequestReviewer, PullRequestReviewerKind, PullRequestVerdict, PullRequestWriteKind, PullReviewComment, PullReviewThread,
    PullReviewThreadComment, PullReviewLists, PullThreadResolved, ResolvePullThread, WritePullRequest,
};

use crate::daemon::Fleet;
use crate::pull_requesting::{SESSION_UNKNOWN, UNREADABLE};

/// A comment, a request for changes, or a line, file or reply comment with nothing in it. A 422.
const WRITE_BLANK: &str = "fleet.pull_request_write_blank";
/// The forge would not take the write, in its own sentence. A 409.
const WRITE_REFUSED: &str = "fleet.pull_request_write_refused";
/// A file the pull request does not list. A 422.
const FILE_UNKNOWN: &str = "fleet.pull_request_file_unknown";
/// What a file says where the forge sent no patch for it.
const NO_PATCH: &str = "The forge sent no patch for this file. It is binary, or the change is too large for the forge to show.";

/// A file's `patch` and `said`: one of them, the sentence where the forge sent no patch.
fn patch_or_said(patch: Option<String>) -> (Option<String>, Option<String>) {
    match patch {
        Some(patch) => (Some(patch), None),
        None => (None, Some(NO_PATCH.to_string())),
    }
}

/// The write a request makes, or why it cannot be made. **With comments it is one review and a
/// body may be empty for all three kinds**: the comments are what it says.
fn write_of(write: &WritePullRequest) -> Result<PullRequestWrite, &'static str> {
    let body = write.body.clone().filter(|body| !body.trim().is_empty());
    if write.comments.iter().any(|comment| comment_body(comment).trim().is_empty()) {
        return Err("a comment on a line, a file or a thread needs something to say");
    }
    if !write.comments.is_empty() {
        return Ok(PullRequestWrite::Review {
            verdict: match write.kind {
                PullRequestWriteKind::Comment => ReviewSaid::Commented,
                PullRequestWriteKind::Approve => ReviewSaid::Approved,
                PullRequestWriteKind::RequestChanges => ReviewSaid::ChangesRequested,
            },
            body,
            comments: write.comments.iter().map(comment_of).collect(),
            commit: write.commit.clone().filter(|commit| !commit.trim().is_empty()),
        });
    }
    match write.kind {
        PullRequestWriteKind::Comment => body
            .map(|body| PullRequestWrite::Comment { body })
            .ok_or("a comment needs something to say"),
        PullRequestWriteKind::Approve => Ok(PullRequestWrite::Approve { body }),
        PullRequestWriteKind::RequestChanges => body
            .map(|body| PullRequestWrite::RequestChanges { body })
            .ok_or("a request for changes needs a reason"),
    }
}

fn comment_body(comment: &PullReviewComment) -> &str {
    match comment {
        PullReviewComment::Line { body, .. } | PullReviewComment::File { body, .. } | PullReviewComment::Reply { body, .. } => body,
    }
}

fn side_of(side: DiffSide) -> FactSide {
    match side {
        DiffSide::Left => FactSide::Left,
        DiffSide::Right => FactSide::Right,
    }
}

fn comment_of(comment: &PullReviewComment) -> FactComment {
    match comment.clone() {
        PullReviewComment::Line { path, line, side, body } => FactComment::Line {
            path,
            line,
            side: side_of(side),
            body,
        },
        PullReviewComment::File { path, body } => FactComment::File { path, body },
        PullReviewComment::Reply { thread, body } => FactComment::Reply { thread, body },
    }
}

/// What the reviews have come to, in the wire's words.
pub(crate) fn review_state(review: ReviewStanding) -> PullRequestReviewState {
    match review {
        ReviewStanding::Approved => PullRequestReviewState::Approved,
        ReviewStanding::ChangesRequested => PullRequestReviewState::ChangesRequested,
        ReviewStanding::Required => PullRequestReviewState::Required,
        ReviewStanding::None => PullRequestReviewState::None,
    }
}

fn verdict_of(said: ReviewSaid) -> PullRequestVerdict {
    match said {
        ReviewSaid::Approved => PullRequestVerdict::Approved,
        ReviewSaid::ChangesRequested => PullRequestVerdict::ChangesRequested,
        ReviewSaid::Commented => PullRequestVerdict::Commented,
    }
}

/// The forge's page as the wire carries it.
fn page_of(manifest_id: ManifestId, number: u64, facts: PullRequestPageFacts) -> PullRequestPage {
    PullRequestPage {
        manifest_id,
        number,
        base: facts.base,
        author: facts.author,
        created_at: facts.created_at,
        additions: facts.additions,
        deletions: facts.deletions,
        mergeable: match facts.mergeable {
            Mergeability::Mergeable => PullRequestMergeable::Mergeable,
            Mergeability::Conflicting => PullRequestMergeable::Conflicting,
            Mergeability::Unknown => PullRequestMergeable::Unknown,
        },
        review: review_state(facts.review),
        approvals: facts.approvals,
        conversation: facts
            .conversation
            .into_iter()
            .map(|said| match said {
                Said::Comment { author, at, body } => PullRequestEntry::Comment { author, at, body },
                Said::Review {
                    author,
                    at,
                    verdict,
                    body,
                } => PullRequestEntry::Review {
                    author,
                    at,
                    verdict: verdict_of(verdict),
                    body,
                },
            })
            .collect(),
        checks: facts
            .checks
            .into_iter()
            .map(|row| PullRequestCheckRow {
                name: row.name,
                state: match row.standing {
                    CheckStanding::Passed => PullRequestCheckState::Passed,
                    CheckStanding::Failed => PullRequestCheckState::Failed,
                    CheckStanding::Running => PullRequestCheckState::Running,
                    CheckStanding::Skipped => PullRequestCheckState::Skipped,
                },
                seconds: row.seconds,
                address: row.address,
            })
            .collect(),
        files: facts
            .files
            .into_iter()
            .map(|file| PullRequestFile {
                path: file.path,
                additions: file.additions,
                deletions: file.deletions,
                status: match file.change {
                    FileChange::Added => PullRequestFileStatus::Added,
                    FileChange::Modified => PullRequestFileStatus::Modified,
                    FileChange::Removed => PullRequestFileStatus::Removed,
                    FileChange::Renamed => PullRequestFileStatus::Renamed,
                },
            })
            .collect(),
        commits: facts
            .commits
            .into_iter()
            .map(|commit| PullRequestCommit {
                sha: commit.sha,
                headline: commit.headline,
                author: commit.author,
                at: commit.at,
            })
            .collect(),
        head: facts.head,
        body: facts.body,
        threads: facts
            .threads
            .into_iter()
            .map(|thread| PullReviewThread {
                id: thread.id,
                path: thread.path,
                line: thread.line,
                side: match thread.side {
                    FactSide::Left => DiffSide::Left,
                    FactSide::Right => DiffSide::Right,
                },
                resolved: thread.resolved,
                comments: thread
                    .comments
                    .into_iter()
                    .map(|said| PullReviewThreadComment {
                        id: said.id,
                        author: said.author,
                        at: said.at,
                        body: said.body,
                    })
                    .collect(),
            })
            .collect(),
        reviewers: facts
            .reviewers
            .into_iter()
            .map(|who| PullRequestReviewer {
                login: who.login,
                kind: match who.kind {
                    ReviewerKind::Person => PullRequestReviewerKind::Person,
                    ReviewerKind::Team => PullRequestReviewerKind::Team,
                },
                verdict: who.verdict.map(verdict_of),
            })
            .collect(),
        owner: None,
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The page as the forge shows it. What the reader capped is counted in
    /// the facts' `omitted`; Fleet has no log outside a Job to say it in.
    fn page_read(
        &self,
        manifest_id: &ManifestId,
        root: &str,
        number: u64,
    ) -> Result<PullRequestPage, Refusal> {
        let facts = self
            .vcs()
            .pull_request_page(root, &number.to_string())
            .ok_or_else(|| {
                Refusal::Fault(self.pr_refusal(
                    UNREADABLE,
                    format!("the forge would not say what is on pull request {number}"),
                ))
            })?;
        Ok(page_of(manifest_id.clone(), number, facts))
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The page, with the owner the last reading of the open pull requests found. **No call to
    /// the store**: opening the panel waits on the forge already, and not on a second service.
    async fn page_with_owner(
        &self,
        manifest_id: &ManifestId,
        root: &str,
        number: u64,
    ) -> Result<PullRequestPage, Refusal> {
        let mut page = self.page_read(manifest_id, root, number)?;
        page.owner = self.owner_known(root, number).await;
        Ok(page)
    }
}

impl<H, V, W> PullRequestPanel for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_pull_request_detail(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestPage, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        self.page_with_owner(&manifest_id, served.root(), number).await
    }

    async fn get_pull_request_file(
        &self,
        manifest_id: ManifestId,
        number: u64,
        path: String,
    ) -> Result<PullRequestFilePatch, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let found = self
            .vcs()
            .pull_request_file_patch(served.root(), &number.to_string(), &path)
            .ok_or_else(|| {
                Refusal::Fault(self.pr_refusal(
                    UNREADABLE,
                    format!("the forge would not say what changed in {path} on pull request {number}"),
                ))
            })?;
        let (patch, said) = match found {
            FilePatch::Patch(patch) => patch_or_said(Some(patch)),
            FilePatch::NoPatch => patch_or_said(None),
            FilePatch::NoSuchFile => {
                return Err(Refusal::Unacceptable(self.pr_refusal(
                    FILE_UNKNOWN,
                    format!("pull request {number} has no file {path}"),
                )))
            }
        };
        Ok(PullRequestFilePatch {
            manifest_id,
            number,
            path,
            patch,
            said,
        })
    }

    async fn get_pull_request_patches(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestFilePatches, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let found = self
            .vcs()
            .pull_request_patches(served.root(), &number.to_string())
            .ok_or_else(|| {
                Refusal::Fault(self.pr_refusal(
                    UNREADABLE,
                    format!("the forge would not say what changed on pull request {number}"),
                ))
            })?;
        let files = found
            .into_iter()
            .map(|PatchedFile { path, patch }| {
                let (patch, said) = patch_or_said(patch);
                PullRequestFilePatch {
                    manifest_id: manifest_id.clone(),
                    number,
                    path,
                    patch,
                    said,
                }
            })
            .collect();
        Ok(PullRequestFilePatches { files })
    }

    async fn write_pull_request(
        &self,
        manifest_id: ManifestId,
        write: WritePullRequest,
    ) -> Result<PullRequestPage, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let root = served.root();
        let number = write.pull_request;
        let to_write = write_of(&write)
            .map_err(|said| Refusal::Unacceptable(self.pr_refusal(WRITE_BLANK, said)))?;
        if let Some(id) = &write.session_id {
            let kept = self
                .store()
                .lock()
                .await
                .session(id.as_str())
                .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?;
            if kept.is_none() {
                return Err(Refusal::Unacceptable(self.pr_refusal(
                    SESSION_UNKNOWN,
                    format!("no job `{}` has reported to Fleet", id.as_str()),
                )));
            }
        }
        self.vcs()
            .write_pull_request(root, &number.to_string(), &to_write)
            .map_err(|said| Refusal::IllegalMove(self.pr_refusal(WRITE_REFUSED, said)))?;
        // **The write is taken whatever follows**: the ledger is kept current on
        // a best effort, and a standing the forge would not read leaves the rows
        // as they were.
        if let Ok(state) = self.pull_request_read(&manifest_id, root, number) {
            self.ledger_follows(&state).await;
        }
        self.page_with_owner(&manifest_id, root, number).await
    }

    async fn get_review_lists(&self) -> Result<PullReviewLists, Refusal> {
        self.review_lists_read().await
    }

    async fn list_review_tools(&self, manifest_id: ManifestId) -> Result<ipc::ReviewTools, Refusal> {
        self.review_tools_read(manifest_id).await
    }

    async fn save_review_tool(
        &self,
        manifest_id: ManifestId,
        save: ipc::SaveReviewTool,
    ) -> Result<ipc::ReviewTools, Refusal> {
        self.review_tool_saved(manifest_id, save).await
    }

    async fn start_review_run(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        start: ipc::StartReviewRun,
    ) -> Result<ipc::ReviewRun, Refusal> {
        self.review_run_started(manifest_id, start).await
    }

    async fn get_review_runs(&self, manifest_id: ManifestId, number: u64) -> Result<ipc::ReviewRuns, Refusal> {
        self.review_runs_read(manifest_id, number).await
    }

    async fn start_review_search(
        self: std::sync::Arc<Self>,
        start: ipc::StartReviewSearch,
    ) -> Result<ipc::ReviewRun, Refusal> {
        self.review_search_started(start).await
    }

    async fn get_review_searches(&self) -> Result<ipc::ReviewRuns, Refusal> {
        self.review_searches_read().await
    }

    async fn forget_review_search(&self, forget: ipc::ForgetReviewSearch) -> Result<ipc::ReviewRuns, Refusal> {
        self.review_search_forgotten(forget).await
    }

    async fn find_walk_place(&self, manifest_id: ManifestId, number: u64) -> Result<ipc::WalkPlaceFound, Refusal> {
        self.walk_place_found(manifest_id, number).await
    }

    async fn list_walk_places(&self) -> Result<ipc::RememberedPlaces, Refusal> {
        self.walk_places_listed().await
    }

    async fn save_walk_place(
        &self,
        manifest_id: ManifestId,
        save: ipc::SaveWalkPlace,
    ) -> Result<ipc::RememberedPlace, Refusal> {
        self.walk_place_saved(manifest_id, save).await
    }

    async fn forget_walk_place(&self, manifest_id: ManifestId, id: String) -> Result<ipc::RememberedPlaces, Refusal> {
        self.walk_place_forgotten(manifest_id, id).await
    }

    async fn open_review_walk(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        open: ipc::OpenReviewWalk,
    ) -> Result<ipc::ReviewWalk, Refusal> {
        self.review_walk_opened(manifest_id, open).await
    }

    async fn close_review_walk(&self, manifest_id: ManifestId, id: String) -> Result<ipc::ReviewWalk, Refusal> {
        self.review_walk_closed(manifest_id, id).await
    }

    async fn resolve_review_thread(
        &self,
        manifest_id: ManifestId,
        resolve: ResolvePullThread,
    ) -> Result<PullThreadResolved, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        self.review_thread_resolved(served.root(), resolve)
    }
}
