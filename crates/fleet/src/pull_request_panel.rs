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
    AgentHarness, CheckStanding, Delivery, FileChange, Mergeability, PullRequestPageFacts,
    PullRequestWrite, ReviewSaid, ReviewStanding, Said, Vcs, WorkProduct,
};
use api::{PullRequestPanel, Refusal};
use ipc::{
    ManifestId, PullRequestCheckRow, PullRequestCheckState, PullRequestCommit, PullRequestEntry,
    PullRequestFile, PullRequestFileStatus, PullRequestMergeable, PullRequestPage,
    PullRequestReviewState, PullRequestVerdict, PullRequestWriteKind, WritePullRequest,
};

use crate::daemon::Fleet;
use crate::pull_requesting::{SESSION_UNKNOWN, UNREADABLE};

/// A comment or a request for changes with nothing in it. A 422.
const WRITE_BLANK: &str = "fleet.pull_request_write_blank";
/// The forge would not take the write, in its own sentence. A 409.
const WRITE_REFUSED: &str = "fleet.pull_request_write_refused";

/// The write a request makes, or why it cannot be made.
fn write_of(write: &WritePullRequest) -> Result<PullRequestWrite, &'static str> {
    let body = write.body.clone().filter(|body| !body.trim().is_empty());
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
        review: match facts.review {
            ReviewStanding::Approved => PullRequestReviewState::Approved,
            ReviewStanding::ChangesRequested => PullRequestReviewState::ChangesRequested,
            ReviewStanding::Required => PullRequestReviewState::Required,
            ReviewStanding::None => PullRequestReviewState::None,
        },
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
                    verdict: match verdict {
                        ReviewSaid::Approved => PullRequestVerdict::Approved,
                        ReviewSaid::ChangesRequested => PullRequestVerdict::ChangesRequested,
                        ReviewSaid::Commented => PullRequestVerdict::Commented,
                    },
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
        self.page_read(&manifest_id, served.root(), number)
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
                    format!("no session `{}` has reported to Fleet", id.as_str()),
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
        self.page_read(&manifest_id, root, number)
    }
}
