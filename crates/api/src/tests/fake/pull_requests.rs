//! A pull request, as the fake answers one: the same open pull request for any
//! number, with its checks passed. What Fleet reads and writes on a forge is
//! `fleet::pull_requesting`'s, against a scripted one.

use ipc::{
    ForgeChecks, ManifestId, PullRequestStanding, PullRequestState, ReviewDispatched,
    ReviewPullRequest,
};

use super::FakeDaemon;
use crate::daemon::Redirector;
use crate::{PullRequests, Refusal};

fn open(manifest_id: ManifestId, number: u64) -> PullRequestState {
    PullRequestState {
        manifest_id,
        number,
        state: PullRequestStanding::Open,
        auto_merge: false,
        checks: ForgeChecks::Passed,
        title: "A pull request".into(),
        branch: "a/branch".into(),
        address: format!("https://forge.invalid/a/b/pull/{number}"),
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
