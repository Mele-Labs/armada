//! [`FakeVcs`] as the forge: every [`Forge`] method, recorded and answered as
//! scripted.

use adapter_traits::{
    Base, Forge, FromOutside, LandingOnTheForge, Merged, NotDelivered, NotMerged, Opened, Remark,
    Review, UnderReview, WhatBecameOfIt, Worktree,
};

use super::{Delivered, FakeVcs, Merging};

impl FakeVcs {
    /// What the script says became of a pull request, recorded by nobody:
    /// [`landed`](adapter_traits::Delivery::landed) records the ask itself.
    pub(super) fn what_became_of_it(&self) -> WhatBecameOfIt {
        let delivery = self.delivery.lock().expect("not poisoned");
        WhatBecameOfIt {
            landing: delivery.landed.clone(),
            base: delivery.base_on_the_forge.clone(),
            number: delivery.number,
            title: delivery.title.clone(),
            mergeable: delivery.mergeable,
            merged_at: delivery.merged_at.clone(),
        }
    }
}

impl Forge for FakeVcs {
    fn open_for_review(
        &self,
        _worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::OpenedForReview {
                base: base.name().to_string(),
                review: review.clone(),
            });
        Ok(self.delivery.lock().expect("not poisoned").review.clone())
    }

    fn under_review(&self, _in_repo: &str, pull_request: &str, gate: &str) -> UnderReview {
        self.main_ci.gated_by(gate);
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedWhatIsUnderReview {
                pull_request: pull_request.to_string(),
            });
        self.delivery
            .lock()
            .expect("not poisoned")
            .under_review
            .clone()
    }

    fn inline_remarks(&self, _in_repo: &str, pull_request: &str) -> Option<Vec<Remark>> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedForInlineRemarks {
                pull_request: pull_request.to_string(),
            });
        self.delivery
            .lock()
            .expect("not poisoned")
            .inline_remarks
            .clone()
    }

    fn base_head_on_the_forge(&self, _in_repo: &str, _base: &str) -> Option<String> {
        self.main_ci.head()
    }

    fn ci_runs_on(&self, _in_repo: &str, commit: &str) -> Option<adapter_traits::CiRuns> {
        self.main_ci.runs(commit)
    }

    fn queue_commit_of(&self, _in_repo: &str, _base: &str, number: u64) -> Option<String> {
        self.main_ci.queue_commit(number)
    }

    fn ci_log(&self, _in_repo: &str, run: &adapter_traits::CiRun) -> Option<FromOutside> {
        self.main_ci.log(run)
    }

    fn merged_by(&self, _in_repo: &str, commit: &str) -> Option<adapter_traits::MergedPull> {
        self.main_ci.merged(commit)
    }

    fn merge_queue(&self, _in_repo: &str, _base: &str) -> Option<adapter_traits::MergeQueue> {
        self.main_ci.merge_queue()
    }

    fn open_pull_requests(&self, _in_repo: &str, gate: &str) -> Option<adapter_traits::OpenPulls> {
        self.main_ci.gated_by(gate);
        self.main_ci.open_pulls()
    }

    fn pull_watch(&self, _in_repo: &str, gate: &str) -> Option<adapter_traits::WatchedPulls> {
        self.main_ci.gated_by(gate);
        self.main_ci.watched()
    }

    fn watched_pull(
        &self,
        _in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::WatchedPull> {
        self.main_ci.gated_by(gate);
        self.main_ci.watched_one(number)
    }

    fn open_pull_request(
        &self,
        _in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::OpenPull> {
        self.main_ci.gated_by(gate);
        self.main_ci.open_one(number)
    }

    fn forge_login(&self, _in_repo: &str) -> Option<String> {
        self.main_ci.login()
    }

    fn recently_merged_pull_requests(
        &self,
        _in_repo: &str,
        _base: &str,
        limit: usize,
    ) -> Option<adapter_traits::RecentlyMergedPulls> {
        self.main_ci.recently_merged(limit)
    }

    fn pull_request_diff(
        &self,
        _in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestDiff> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedForTheDiff {
                pull_request: pull_request.to_string(),
            });
        self.delivery
            .lock()
            .expect("not poisoned")
            .pull_request_diff
            .clone()
    }

    fn rerun_failed(
        &self,
        _in_repo: &str,
        pull_request: &str,
    ) -> Result<adapter_traits::Rerun, adapter_traits::NotRerun> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::RerunFailed {
                pull_request: pull_request.to_string(),
            });
        self.delivery.lock().expect("not poisoned").rerun.clone()
    }

    fn recorded_laps(
        &self,
        _in_repo: &str,
        commit: &str,
    ) -> Result<adapter_traits::LapsOnForge, adapter_traits::NotAsked> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedForLaps { commit: commit.to_string() });
        self.delivery.lock().expect("not poisoned").recorded_laps.clone()
    }

    fn file_issue(
        &self,
        _in_repo: &str,
        title: &str,
        _body: &str,
    ) -> Result<adapter_traits::FiledIssue, adapter_traits::NotFiled> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::FiledIssue {
                title: title.to_string(),
            });
        self.delivery.lock().expect("not poisoned").filed.clone()
    }

    fn enable_auto_merge(&self, _in_repo: &str, pull_request: &str) -> Result<(), String> {
        self.auto_merge_asked(pull_request)
    }

    fn disable_auto_merge(&self, _in_repo: &str, pull_request: &str) -> Result<(), String> {
        self.auto_merge_withdrawn(pull_request)
    }

    fn pull_request_facts(
        &self,
        _in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestFacts> {
        // A pull request the watch lists is open, as a forge read of it says, unless a test
        // scripted what the read answers.
        self.pull_request_read(pull_request).or_else(|| {
            let number: u64 = pull_request.trim_start_matches('#').parse().ok()?;
            let pull = self.main_ci.watched()?.into_iter().find(|pull| pull.number == number)?;
            Some(adapter_traits::PullRequestFacts {
                standing: adapter_traits::PullRequestStanding::Open,
                branch: pull.branch.as_written().to_string(),
                auto_merge: false,
                title: String::new(),
                url: pull.url.as_written().to_string(),
                body: None,
                closes: Vec::new(),
            })
        })
    }

    fn mark_ready(&self, _in_repo: &str, pull_request: &str) -> Result<(), String> {
        self.ready_asked(pull_request)
    }

    fn pull_request_page(
        &self,
        _in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestPageFacts> {
        self.page_read(pull_request)
    }

    fn pull_request_file_patch(
        &self,
        _in_repo: &str,
        pull_request: &str,
        path: &str,
    ) -> Option<adapter_traits::FilePatch> {
        self.file_patch_read(pull_request, path)
    }

    fn pull_request_patches(
        &self,
        _in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<adapter_traits::PatchedFile>> {
        self.patches_read(pull_request)
    }

    fn write_pull_request(
        &self,
        _in_repo: &str,
        pull_request: &str,
        write: &adapter_traits::PullRequestWrite,
    ) -> Result<(), String> {
        self.write_asked(pull_request, write)
    }

    fn pull_review_list(
        &self,
        _in_repo: &str,
        ask: &adapter_traits::ReviewListAsk<'_>,
    ) -> Option<Vec<adapter_traits::ListedPull>> {
        self.review_list_read(ask)
    }

    fn resolve_review_thread(
        &self,
        _in_repo: &str,
        thread: &str,
        resolved: bool,
    ) -> Result<bool, String> {
        self.thread_resolution_asked(thread, resolved)
    }

    fn merge(&self, _in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged> {
        // Recorded before the answer, because the forge is reached either way:
        // a refused merge is a call that happened and a test asserting nothing
        // was merged is asserting about the Job, not about the process.
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::Merged {
                pull_request: pull_request.to_string(),
            });
        match self.merging.lock().expect("not poisoned").clone() {
            Merging::Takes => {
                self.pull_request_merged();
                Ok(Merged::Taken)
            }
            Merging::AlreadyMerged => Ok(Merged::AlreadyMerged),
            Merging::Refuses(why) => Err(why),
        }
    }

    /// The scripted answer [`landed`](adapter_traits::Delivery::landed) gives,
    /// with no commits to read a rendering from: the fake scripts that whole.
    fn landing(&self, _in_repo: &str, _pull_request: &str) -> LandingOnTheForge {
        LandingOnTheForge {
            became: self.what_became_of_it(),
            compared: None,
        }
    }

    /// Refused: the fake opens pull requests only through its scripted sends,
    /// [`send_manifest_edit`](adapter_traits::Delivery::send_manifest_edit) and
    /// [`send_checkout_as_pull_request`](adapter_traits::Delivery::send_checkout_as_pull_request).
    fn open_branch_for_review(
        &self,
        _in_repo: &str,
        _base: &str,
        _branch: &str,
        _title: &str,
        _body: &str,
    ) -> Result<String, NotDelivered> {
        Err(NotDelivered::of(
            "the pull request",
            "the fake opens one only through a scripted send",
        ))
    }
}
