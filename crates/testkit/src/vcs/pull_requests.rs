//! What the fake forge says about one pull request, and what it was asked to
//! write on one. Its own file for `merging`'s reason: it is scripted through
//! `&self`, because nobody reads or readies a pull request until the Fleet
//! holding this fake exists.

use adapter_traits::{PullRequestFacts, PullRequestStanding};

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
