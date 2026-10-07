//! What the fake forge and remote answer, scripted as one value.

use adapter_traits::{
    Base, BranchMerged, BroughtUpToDate, KeptCurrent, Landing, Mergeable, Opened, Pushed, Remark,
    RepositoryStanding, Standing, UnderReview,
};

/// What the fake's version control looks like from the delivery side.
///
/// Public fields and no `Default` for the reason `Fittings` has neither: a test
/// writes out every one of the four, so the repository it is describing is
/// visible at the call site rather than inherited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivering {
    /// What the repository says its base is, or `None` for one that names none.
    pub base: Option<Base>,
    /// How far behind the base the branch is.
    pub standing: Standing,
    /// What bringing it up to date comes to. `None` where it is not behind.
    pub rebase: Option<BroughtUpToDate>,
    pub push: Pushed,
    pub review: Opened,
    /// The forge's reply to turning auto-merge on: `Err` is its own sentence.
    pub auto_merge: Result<(), String>,
    /// What the forge says became of the pull request afterwards. `Unknown` by
    /// default, which is the answer on a machine with no forge and the one a
    /// test gets unless it says a merge happened.
    pub landed: Landing,
    /// The branch the forge says the pull request merges into. `None` is a
    /// forge that answered nothing, which is what [`Landing::Unknown`] means.
    pub base_on_the_forge: Option<String>,
    /// The forge's own number for the pull request, riding the same read as
    /// `landed` and `base_on_the_forge`. `None` on the same grounds as
    /// `base_on_the_forge`.
    pub number: Option<u64>,
    /// The pull request's title, as the forge holds it right now.
    pub title: Option<String>,
    /// Whether the forge can merge it as it stands. `Yes` by default, the
    /// shape a Job that goes the whole way runs against.
    pub mergeable: Mergeable,
    /// When it merged, as the forge says. `None` by default, which is also
    /// what the forge says about one that has not.
    pub merged_at: Option<String>,
    /// What the forge says about the pull request while it is still open.
    /// Unreadable by default, which is the answer on a machine with no forge —
    /// and the one every case that is not about reviews should get, so that
    /// nothing reads an approval nobody scripted.
    pub under_review: UnderReview,
    /// The forge's answer for inline diff comments: answered and empty by
    /// default, so a case that scripts only `under_review` counts what it
    /// scripted. `None` is a forge that would not answer. Never inferred from
    /// `under_review` — a real forge does not either.
    pub inline_remarks: Option<Vec<Remark>>,
    /// The forge's answer for a pull request's diff. `None` by default, the forge's silence.
    pub pull_request_diff: Option<adapter_traits::PullRequestDiff>,
    /// What re-running failed CI runs comes to. One run started again by default.
    pub rerun: Result<adapter_traits::Rerun, adapter_traits::NotRerun>,
    /// What filing an issue comes to. Filed by default.
    pub filed: Result<adapter_traits::FiledIssue, adapter_traits::NotFiled>,
    /// What keeping the branch current comes to. A clean rebase by default,
    /// because the case a test has to write out is the one where it conflicted
    /// or found no branch.
    pub kept_current: KeptCurrent,
    /// What catching the repository up comes to.
    pub repository: RepositoryStanding,
    /// What merging one local branch into another comes to.
    pub branch_merge: BranchMerged,
}

impl Default for Delivering {
    /// A repository on `main`, up to date, with a remote and a forge — the
    /// shape a Job that goes the whole way runs against.
    fn default() -> Delivering {
        Delivering {
            base: Some(Base::Inferred(String::from("main"))),
            standing: Standing::UpToDate,
            rebase: None,
            push: Pushed::ToTheRemote {
                remote: String::from("origin"),
                branch: String::from("armada/a-job"),
            },
            review: Opened::PullRequest {
                url: String::from("https://forge.invalid/armada/pull/1"),
            },
            auto_merge: Ok(()),
            // Nobody has merged it. A default that said `Merged` would have
            // every existing test's Job land the moment anything asked.
            landed: Landing::Unknown,
            base_on_the_forge: Some(String::from("main")),
            number: Some(1),
            title: Some(String::from("a job's pull request")),
            mergeable: Mergeable::Yes,
            merged_at: None,
            under_review: UnderReview::unreadable(),
            inline_remarks: Some(Vec::new()),
            pull_request_diff: None,
            rerun: Ok(adapter_traits::Rerun { runs: 1 }),
            filed: Ok(adapter_traits::FiledIssue {
                url: String::from("https://forge.invalid/armada/issues/1"),
            }),
            kept_current: KeptCurrent::Rebased {
                onto: String::from("5b4ec82700000000000000000000000000000000"),
                commits: 1,
            },
            branch_merge: BranchMerged::Merged,
            repository: RepositoryStanding::AlreadyHadIt {
                base: String::from("main"),
                // A commit-shaped string, because `#474` keys a proof by it and
                // a fixture that handed back an empty one would key every
                // fake's proof the same way.
                head: String::from("0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f"),
            },
        }
    }
}
