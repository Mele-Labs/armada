//! What a fake repository looks like before a test says otherwise.

use adapter_traits::{
    Base, BranchMerged, KeptCurrent, Landing, Mergeable, Opened, Pushed, RepositoryStanding,
    Standing, UnderReview,
};

use super::Delivering;

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
