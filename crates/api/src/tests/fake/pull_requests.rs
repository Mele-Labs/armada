//! A pull request, as the fake answers one: the same open pull request for any
//! number, with its checks passed. What Fleet reads and writes on a forge is
//! `fleet::pull_requesting`'s, against a scripted one.

use ipc::{
    ForgeChecks, ManifestId, PullRequestMergeable, PullRequestPage, PullRequestReviewState,
    PullRequestStanding, PullRequestState, WritePullRequest,
};

use super::FakeDaemon;
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
        base: "main".into(),
        address: format!("https://forge.invalid/a/b/pull/{number}"),
        body: None,
        closes: Vec::new(),
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
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        Ok(open(manifest_id, number))
    }

    async fn merge_pull_request_by_number(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        Ok(PullRequestState {
            state: PullRequestStanding::Merged,
            ..open(manifest_id, number)
        })
    }

    async fn enable_auto_merge(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        Ok(PullRequestState {
            auto_merge: true,
            ..open(manifest_id, number)
        })
    }

    async fn refresh_pull_request(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        Ok(open(manifest_id, number))
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
        head: None,
        body: None,
        threads: Vec::new(),
        reviewers: Vec::new(),
        owner: None,
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

    async fn get_pull_request_file(
        &self,
        manifest_id: ManifestId,
        number: u64,
        path: String,
    ) -> Result<ipc::PullRequestFilePatch, Refusal> {
        Ok(ipc::PullRequestFilePatch {
            manifest_id,
            number,
            path,
            patch: None,
            said: None,
        })
    }

    async fn get_pull_request_patches(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<ipc::PullRequestFilePatches, Refusal> {
        Ok(ipc::PullRequestFilePatches {
            files: vec![ipc::PullRequestFilePatch {
                manifest_id,
                number,
                path: "a.rs".into(),
                patch: Some("@@ -1 +1 @@\n-a\n+b".into()),
                said: None,
            }],
        })
    }

    async fn write_pull_request(
        &self,
        manifest_id: ManifestId,
        write: WritePullRequest,
    ) -> Result<PullRequestPage, Refusal> {
        Ok(page(manifest_id, write.pull_request))
    }

    async fn get_review_lists(&self) -> Result<ipc::PullReviewLists, Refusal> {
        Ok(ipc::PullReviewLists {
            me: "octocat".into(),
            fetched_at: "2026-10-16T09:00:00Z".into(),
            requested: Vec::new(),
            team: Vec::new(),
            assigned: Vec::new(),
            mine: Vec::new(),
            silent: Vec::new(),
        })
    }

    async fn list_review_tools(&self, _manifest_id: ManifestId) -> Result<ipc::ReviewTools, Refusal> {
        Ok(ipc::ReviewTools {
            tools: Vec::new(),
            left_out: Vec::new(),
        })
    }

    async fn save_review_tool(
        &self,
        _manifest_id: ManifestId,
        _save: ipc::SaveReviewTool,
    ) -> Result<ipc::ReviewTools, Refusal> {
        Ok(ipc::ReviewTools {
            tools: Vec::new(),
            left_out: Vec::new(),
        })
    }

    async fn start_review_run(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        start: ipc::StartReviewRun,
    ) -> Result<ipc::ReviewRun, Refusal> {
        Ok(ipc::ReviewRun {
            manifest_id,
            number: start.number,
            kind: start.kind,
            name: start.name,
            head: "0".repeat(40),
            state: ipc::ReviewRunState::Queued,
            started_at: "2026-10-16T09:00:00Z".into(),
            finished_at: None,
            failed: None,
            result: None,
            cost_micros: None,
        })
    }

    async fn get_review_runs(&self, _manifest_id: ManifestId, _number: u64) -> Result<ipc::ReviewRuns, Refusal> {
        Ok(ipc::ReviewRuns { runs: Vec::new() })
    }

    async fn start_review_search(self: std::sync::Arc<Self>, start: ipc::StartReviewSearch) -> Result<ipc::ReviewRun, Refusal> {
        Ok(ipc::ReviewRun {
            manifest_id: ManifestId::carried(ipc::SEARCH_RUN_SCOPE),
            number: 0,
            kind: ipc::ReviewRunKind::Search,
            name: start.words,
            head: String::new(),
            state: ipc::ReviewRunState::Queued,
            started_at: "2026-10-16T09:00:00Z".into(),
            finished_at: None,
            failed: None,
            result: None,
            cost_micros: None,
        })
    }

    async fn get_review_searches(&self) -> Result<ipc::ReviewRuns, Refusal> {
        Ok(ipc::ReviewRuns { runs: Vec::new() })
    }

    async fn forget_review_search(&self, _forget: ipc::ForgetReviewSearch) -> Result<ipc::ReviewRuns, Refusal> {
        Ok(ipc::ReviewRuns { runs: Vec::new() })
    }

    async fn find_walk_place(&self, _manifest_id: ManifestId, _number: u64) -> Result<ipc::WalkPlaceFound, Refusal> {
        Ok(ipc::WalkPlaceFound { area: String::new(), tried: Vec::new(), place: None })
    }

    async fn list_walk_places(&self) -> Result<ipc::RememberedPlaces, Refusal> {
        Ok(ipc::RememberedPlaces { places: Vec::new() })
    }

    async fn save_walk_place(&self, manifest_id: ManifestId, save: ipc::SaveWalkPlace) -> Result<ipc::RememberedPlace, Refusal> {
        Ok(ipc::RememberedPlace {
            id: "01place".into(),
            manifest_id,
            repository: "armada/armada".into(),
            area: save.area,
            place: save.place,
        })
    }

    async fn forget_walk_place(&self, _manifest_id: ManifestId, _id: String) -> Result<ipc::RememberedPlaces, Refusal> {
        Ok(ipc::RememberedPlaces { places: Vec::new() })
    }

    async fn open_review_walk(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        open: ipc::OpenReviewWalk,
    ) -> Result<ipc::ReviewWalk, Refusal> {
        Ok(ipc::ReviewWalk {
            id: "01walk".into(),
            manifest_id,
            number: open.number,
            state: ipc::ReviewWalkState::Ready,
            slot: None,
            address: open.place.address,
            api: open.place.api,
            failed: None,
        })
    }

    async fn close_review_walk(&self, manifest_id: ManifestId, id: String) -> Result<ipc::ReviewWalk, Refusal> {
        Ok(ipc::ReviewWalk {
            id,
            manifest_id,
            number: 0,
            state: ipc::ReviewWalkState::Closed,
            slot: None,
            address: String::new(),
            api: String::new(),
            failed: None,
        })
    }

    async fn resolve_review_thread(
        &self,
        _manifest_id: ManifestId,
        resolve: ipc::ResolvePullThread,
    ) -> Result<ipc::PullThreadResolved, Refusal> {
        Ok(ipc::PullThreadResolved {
            thread: resolve.thread,
            resolved: resolve.resolved,
        })
    }
}
