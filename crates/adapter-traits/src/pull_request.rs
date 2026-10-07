//! What a forge says about one pull request that is the same whoever opened it:
//! whether it is a draft, which branch it is, and whether auto-merge is asked
//! for. `docs/concepts/session.md`, *Acts on a pull request*.
//!
//! **Beside [`Landing`](crate::Landing), not instead of it.** That one answers
//! what became of a pull request Armada opened and is read by a sweep; this is
//! read for a person looking at one, which may be nobody's Job, and a draft
//! is a state `Landing` has no word for.

use alloc::string::String;

/// Where a pull request stands, as a person reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullRequestStanding {
    /// Open and not yet ready for review. The forge will not merge one.
    Draft,
    Open,
    Merged,
    /// Closed and never merged.
    Closed,
}

/// One forge read of a pull request. **No checks**: those are
/// [`UnderReview::checks`](crate::UnderReview), which names what the forge
/// ran and is already guarded as text written outside this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullRequestFacts {
    pub standing: PullRequestStanding,
    /// The branch it is opened from.
    pub branch: String,
    /// Whether the forge will merge it when its required checks pass.
    pub auto_merge: bool,
    /// Its title as the forge holds it now.
    pub title: String,
    /// Where it is, as the forge prints it.
    pub url: String,
}
