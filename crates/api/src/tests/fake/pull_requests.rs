//! A pull request, as the fake answers one: the same open pull request for any
//! number, with its checks passed. What Fleet reads and writes on a forge is
//! `fleet::pull_requesting`'s, against a scripted one.

use ipc::{
    ForgeChecks, ManifestId, PullRequestMergeable, PullRequestPage, PullRequestReviewState,
    PullRequestStanding, PullRequestState, ReviewDispatched, ReviewPullRequest, WritePullRequest,
};

use super::FakeDaemon;
use crate::daemon::Redirector;
use crate::{PullRequestPanel, PullRequests, Refusal};

fn open(manifest_id: ManifestId, number: u64) -> PullRequestState {
    PullRequestState {
        manifest_id,
        number,
        state: PullRequestStanding::Open,
        auto_merge: false,
        queued: false,
        checks: ForgeChecks::Passed,
        title: "A pull request".into(),
        branch: "a/branch".into(),
        address: format!("https://forge.invalid/a/b/pull/{number}"),
        body: None,
    }
}

impl PullRequests for FakeDaemon {
    async fn get_pull_request(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        Ok(open(manifest_id, number))
    }

    async fn ready_pull_request(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        Ok(open(manifest_id, number))
    }

    async fn merge_pull_request_by_number(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        Ok(PullRequestState {
            state: PullRequestStanding::Merged,
            ..open(manifest_id, number)
        })
    }

    async fn enable_auto_merge(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        Ok(PullRequestState {
            auto_merge: true,
            ..open(manifest_id, number)
        })
    }

    async fn review_pull_request(
        &self,
        _manifest_id: ManifestId,
        review: ReviewPullRequest,
        _by: Redirector,
    ) -> Result<ReviewDispatched, Refusal> {
        Ok(ReviewDispatched {
            job_id: ipc::JobId::carried("01FAKEREVIEWJOB"),
            address: review.pull_request,
            session_id: review.session_id,
        })
    }
}

fn page(manifest_id: ManifestId, number: u64) -> PullRequestPage {
    PullRequestPage {
        manifest_id,
        number,
        base: "main".into(),
        author: None,
        created_at: None,
        additions: 0,
        deletions: 0,
        mergeable: PullRequestMergeable::Unknown,
        review: PullRequestReviewState::None,
        approvals: 0,
        conversation: Vec::new(),
        checks: Vec::new(),
        files: Vec::new(),
        commits: Vec::new(),
    }
}

impl PullRequestPanel for FakeDaemon {
    async fn get_pull_request_detail(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestPage, Refusal> {
        Ok(page(manifest_id, number))
    }

    async fn write_pull_request(
        &self,
        manifest_id: ManifestId,
        write: WritePullRequest,
    ) -> Result<PullRequestPage, Refusal> {
        Ok(page(manifest_id, write.pull_request))
    }
}
