//! What the fake forge says about one pull request, and what it was asked to
//! write on one. Its own file for `merging`'s reason: it is scripted through
//! `&self`, because nobody reads or readies a pull request until the Fleet
//! holding this fake exists.

use adapter_traits::{PullRequestFacts, PullRequestStanding, Remark, UnderReview};

use super::{commit, Delivered, FakeVcs};

/// What the forge answers, and what the next write to it comes to.
#[derive(Debug, Clone)]
pub(super) struct PullRequests {
    /// `None` is the forge's silence, which is also the default.
    pub(super) facts: Option<PullRequestFacts>,
    /// The forge's reply to taking a pull request out of draft.
    pub(super) ready: Result<(), String>,
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
