//! What the fake forge says about one pull request, and what it was asked to
//! write on one. Its own file for `merging`'s reason: it is scripted through
//! `&self`, because nobody reads or readies a pull request until the Fleet
//! holding this fake exists.

use adapter_traits::{
    FilePatch, ListedPull, PatchedFile, PullRequestFacts, PullRequestPageFacts, PullRequestStanding, PullRequestWrite,
    Remark, ReviewList, ReviewListAsk, UnderReview,
};

use super::{commit, Delivered, FakeVcs};

/// What the forge answers, and what the next write to it comes to.
#[derive(Debug, Clone)]
pub(super) struct PullRequests {
    /// `None` is the forge's silence, which is also the default.
    pub(super) facts: Option<PullRequestFacts>,
    /// The forge's reply to taking a pull request out of draft.
    pub(super) ready: Result<(), String>,
    /// What the forge answers for a pull request's page. `None` is silence.
    pub(super) page: Option<PullRequestPageFacts>,
    /// A page for one pull request, by number, ahead of `page`. `None` is silence for that one.
    pub(super) pages: Vec<(String, Option<PullRequestPageFacts>)>,
    /// The paths the person's commits changed, and every history read asked: authors and count.
    pub(super) touched: Vec<String>,
    pub(super) touched_asked: Vec<(Vec<String>, usize)>,
    /// What the forge answers for one file's patch. `None` is silence.
    pub(super) file_patch: Option<FilePatch>,
    /// What the forge answers for every file's patch. `None` is silence.
    pub(super) patches: Option<Vec<PatchedFile>>,
    /// The forge's reply to a write on a pull request.
    pub(super) written: Result<(), String>,
    /// What the forge answers for each review list. A list with no entry is silent.
    pub(super) review_lists: Vec<(ReviewList, Vec<ListedPull>)>,
    /// The forge's reply to resolving or reopening a review thread.
    pub(super) thread_resolved: Result<(), String>,
    /// What the forge's facts become once a merge, a ready or an auto-merge is
    /// taken, as a real forge's would. A test that wants them to stay put
    /// scripts nothing.
    pub(super) follows_writes: bool,
}

impl Default for PullRequests {
    fn default() -> PullRequests {
        PullRequests {
            facts: None,
            ready: Ok(()),
            page: None,
            pages: Vec::new(),
            touched: Vec::new(),
            touched_asked: Vec::new(),
            file_patch: None,
            patches: None,
            written: Ok(()),
            review_lists: Vec::new(),
            thread_resolved: Ok(()),
            follows_writes: true,
        }
    }
}

impl PullRequests {
    /// A write the forge took, as it would show on the next read.
    pub(super) fn took_ready(&mut self) {
        if let (true, Some(facts)) = (self.follows_writes, self.facts.as_mut()) {
            if facts.standing == PullRequestStanding::Draft {
                facts.standing = PullRequestStanding::Open;
            }
        }
    }

    pub(super) fn took_auto_merge(&mut self) {
        if let (true, Some(facts)) = (self.follows_writes, self.facts.as_mut()) {
            facts.auto_merge = true;
        }
    }

    pub(super) fn took_merge(&mut self) {
        if let (true, Some(facts)) = (self.follows_writes, self.facts.as_mut()) {
            facts.standing = PullRequestStanding::Merged;
        }
    }
}

impl FakeVcs {
    /// `Delivery::enable_auto_merge`, and the forge showing it on the next read.
    pub(super) fn auto_merge_asked(&self, pull_request: &str) -> Result<(), String> {
        let answered = commit::auto_merge(self, pull_request);
        if answered.is_ok() {
            self.pull_requests
                .lock()
                .expect("not poisoned")
                .took_auto_merge();
        }
        answered
    }

    /// `Delivery::pull_request_facts`: recorded, then answered as scripted.
    pub(super) fn pull_request_read(&self, pull_request: &str) -> Option<PullRequestFacts> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::ReadPullRequest {
                pull_request: pull_request.to_string(),
            });
        self.pull_requests
            .lock()
            .expect("not poisoned")
            .facts
            .clone()
    }

    /// `Delivery::mark_ready`: recorded, then answered as scripted.
    pub(super) fn ready_asked(&self, pull_request: &str) -> Result<(), String> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::MarkedReady {
                pull_request: pull_request.to_string(),
            });
        let mut forge = self.pull_requests.lock().expect("not poisoned");
        let answered = forge.ready.clone();
        if answered.is_ok() {
            forge.took_ready();
        }
        answered
    }

    /// `Delivery::pull_request_page`: recorded, then answered as scripted.
    pub(super) fn page_read(&self, pull_request: &str) -> Option<PullRequestPageFacts> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::ReadPullRequestPage {
                pull_request: pull_request.to_string(),
            });
        let forge = self.pull_requests.lock().expect("not poisoned");
        match forge.pages.iter().find(|(number, _)| number == pull_request) {
            Some((_, page)) => page.clone(),
            None => forge.page.clone(),
        }
    }

    /// `Vcs::paths_touched_by`: recorded, then answered as scripted.
    pub(super) fn history_read(&self, authors: &[String], commits: usize) -> Vec<String> {
        let mut forge = self.pull_requests.lock().expect("not poisoned");
        forge.touched_asked.push((authors.to_vec(), commits));
        forge.touched.clone()
    }

    /// `Delivery::pull_request_file_patch`: recorded, then answered as scripted.
    pub(super) fn file_patch_read(&self, pull_request: &str, path: &str) -> Option<FilePatch> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::ReadPullRequestFilePatch {
                pull_request: pull_request.to_string(),
                path: path.to_string(),
            });
        self.pull_requests.lock().expect("not poisoned").file_patch.clone()
    }

    /// `Delivery::pull_request_patches`: recorded, then answered as scripted.
    pub(super) fn patches_read(&self, pull_request: &str) -> Option<Vec<PatchedFile>> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::ReadPullRequestPatches {
                pull_request: pull_request.to_string(),
            });
        self.pull_requests.lock().expect("not poisoned").patches.clone()
    }

    /// `Delivery::write_pull_request`: recorded, then answered as scripted.
    pub(super) fn write_asked(
        &self,
        pull_request: &str,
        write: &PullRequestWrite,
    ) -> Result<(), String> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::WroteOnPullRequest {
                pull_request: pull_request.to_string(),
                write: write.clone(),
            });
        self.pull_requests.lock().expect("not poisoned").written.clone()
    }

    /// `Delivery::pull_review_list`: recorded, then answered as scripted. A list nobody
    /// scripted is the forge's silence.
    pub(super) fn review_list_read(&self, ask: &ReviewListAsk<'_>) -> Option<Vec<ListedPull>> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedForReviewList {
                list: ask.list,
                login: ask.login.to_string(),
                repositories: ask.repositories.to_vec(),
            });
        let forge = self.pull_requests.lock().expect("not poisoned");
        let rows = forge.review_lists.iter().find(|(list, _)| *list == ask.list)?;
        Some(rows.1.iter().take(ask.most).cloned().collect())
    }

    /// `Delivery::resolve_review_thread`: recorded, then answered as scripted.
    pub(super) fn thread_resolution_asked(&self, thread: &str, resolved: bool) -> Result<bool, String> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::ResolvedReviewThread {
                thread: thread.to_string(),
                resolved,
            });
        self.pull_requests
            .lock()
            .expect("not poisoned")
            .thread_resolved
            .clone()
            .map(|()| resolved)
    }

    /// Say what the forge answers for one review list. Say nothing, and that list is silent.
    pub fn now_review_list(&self, list: ReviewList, rows: Vec<ListedPull>) {
        let mut forge = self.pull_requests.lock().expect("not poisoned");
        forge.review_lists.retain(|(held, _)| *held != list);
        forge.review_lists.push((list, rows));
    }

    /// Say that resolving or reopening a review thread is refused, in the forge's own sentence.
    pub fn resolving_a_thread_refuses(&self, said: &str) {
        self.pull_requests.lock().expect("not poisoned").thread_resolved = Err(said.to_string());
    }

    /// Every list the forge was asked for, in order, with the login and repositories asked.
    pub fn review_lists_asked(&self) -> Vec<(ReviewList, String, Vec<String>)> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter_map(|it| match it {
                Delivered::AskedForReviewList {
                    list,
                    login,
                    repositories,
                } => Some((*list, login.clone(), repositories.clone())),
                _ => None,
            })
            .collect()
    }

    /// Every thread resolved or reopened, in order, with whether it was resolved.
    pub fn review_thread_resolutions(&self) -> Vec<(String, bool)> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter_map(|it| match it {
                Delivered::ResolvedReviewThread { thread, resolved } => Some((thread.clone(), *resolved)),
                _ => None,
            })
            .collect()
    }

    /// Say what the forge shows on a pull request's page, or that it is silent.
    pub fn now_pull_request_page(&self, page: Option<PullRequestPageFacts>) {
        self.pull_requests.lock().expect("not poisoned").page = page;
    }

    /// Say what the forge shows on one pull request's page, ahead of the page said for the rest.
    pub fn now_page_of_pull_request(&self, number: u64, page: Option<PullRequestPageFacts>) {
        let mut forge = self.pull_requests.lock().expect("not poisoned");
        forge.pages.retain(|(held, _)| *held != number.to_string());
        forge.pages.push((number.to_string(), page));
    }

    /// Say which paths the person's commits changed.
    pub fn now_paths_touched(&self, paths: &[&str]) {
        self.pull_requests.lock().expect("not poisoned").touched = paths.iter().map(|path| path.to_string()).collect();
    }

    /// Every history read asked, in order, as the authors named and the commits counted.
    pub fn paths_touched_asked(&self) -> Vec<(Vec<String>, usize)> {
        self.pull_requests.lock().expect("not poisoned").touched_asked.clone()
    }

    /// Say what the forge holds for a file's patch, or that it is silent.
    pub fn now_pull_request_file_patch(&self, patch: Option<FilePatch>) {
        self.pull_requests.lock().expect("not poisoned").file_patch = patch;
    }

    /// Say what the forge holds for every file's patch, or that it is silent.
    pub fn now_pull_request_patches(&self, files: Option<Vec<PatchedFile>>) {
        self.pull_requests.lock().expect("not poisoned").patches = files;
    }

    /// Say that a write on a pull request is refused, in the forge's own sentence.
    pub fn write_on_pull_request_refuses(&self, said: &str) {
        self.pull_requests.lock().expect("not poisoned").written = Err(said.to_string());
    }

    /// Every write the forge was asked to take, in order, with the pull request's number.
    pub fn pull_request_writes(&self) -> Vec<(String, PullRequestWrite)> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter_map(|it| match it {
                Delivered::WroteOnPullRequest {
                    pull_request,
                    write,
                } => Some((pull_request.clone(), write.clone())),
                _ => None,
            })
            .collect()
    }

    /// A merge the forge took, as it shows on the next read.
    pub(super) fn pull_request_merged(&self) {
        self.pull_requests
            .lock()
            .expect("not poisoned")
            .took_merge();
    }

    /// Say what the forge shows of a pull request from now on, or that it is
    /// silent. **Writes taken afterwards show on the next read**, as a real
    /// forge's would: a merge makes it merged, a ready makes a draft open.
    pub fn now_pull_request(&self, facts: Option<adapter_traits::PullRequestFacts>) {
        self.pull_requests.lock().expect("not poisoned").facts = facts;
    }

    /// Say what the forge answers when auto-merge is asked for: `Err` is its
    /// own sentence. `&self`, as [`now_pull_request`](FakeVcs::now_pull_request).
    pub fn now_auto_merge(&self, answer: Result<(), String>) {
        self.delivery.lock().expect("not poisoned").auto_merge = answer;
    }

    /// Say that taking a pull request out of draft is refused, in the forge's
    /// own sentence.
    pub fn ready_refuses(&self, said: &str) {
        self.pull_requests.lock().expect("not poisoned").ready = Err(said.to_string());
    }

    /// How many times a pull request was taken out of draft.
    pub fn times_asked_to_ready(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::MarkedReady { .. }))
    }

    /// How many times auto-merge was asked for.
    pub fn times_asked_for_auto_merge(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::AutoMerge { .. }))
    }

    /// Say what the forge says about the open pull request: who has looked at
    /// it, what ran against it, what anybody wrote.
    ///
    /// `&self` for [`now_landed`](FakeVcs::now_landed)'s reason — nothing is
    /// reviewed until the Job that opened the pull request has finished, by
    /// which time the fake is inside a Fleet.
    pub fn now_under_review(&self, under_review: UnderReview) {
        self.delivery.lock().expect("not poisoned").under_review = under_review;
    }

    /// How many times the forge has been asked what is happening on an open
    /// pull request. **Counted apart from the merge question**, because the two
    /// ride one rotation and a test proving the sweep did not grow a second
    /// loop is counting these against those.
    pub fn times_asked_what_is_under_review(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::AskedWhatIsUnderReview { .. }))
    }

    /// Say what the forge answers for inline diff comments. `&self` for
    /// [`now_under_review`](FakeVcs::now_under_review)'s reason.
    pub fn now_inline_remarks(&self, remarks: Option<Vec<Remark>>) {
        self.delivery.lock().expect("not poisoned").inline_remarks = remarks;
    }

    /// How many times the forge has been asked for inline comments — the
    /// query the sweep makes once per turn on an open pull request.
    pub fn times_asked_for_inline_remarks(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::AskedForInlineRemarks { .. }))
    }
}
