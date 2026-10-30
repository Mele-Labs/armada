//! What the forge says about the newest commit on a repository's base branch:
//! which CI jobs ran on it, how each came out, and which pull request merged
//! it.
//!
//! **The forge's facts and nothing else.** A job is named as the forge names
//! it, and whether it maps to a Check in the Manifest is Fleet's question to
//! answer from the Manifest, never this vocabulary's. A repository whose CI
//! maps to no Check is the ordinary case.
//!
//! Every word a forge wrote arrives as [`FromOutside`], for
//! `crate::under_review`'s reason: a job's name comes from a workflow file on
//! whatever branch merged, and its log is whatever that code printed.

use alloc::string::String;
use alloc::vec::Vec;

use crate::FromOutside;

/// How one CI job on a commit stands. The forge's own words for this are the
/// adapter's; a conclusion it has no name for here is [`CiState::Failed`], the
/// direction `WhatTheForgeRan::SomeFailed` takes: nothing reports a pass it
/// cannot vouch for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CiState {
    Passed,
    Failed,
    /// Queued or running: no conclusion yet.
    Pending,
}

/// One CI job that ran on a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiRun {
    /// The job's name as the forge reports it.
    pub name: FromOutside,
    pub state: CiState,
    /// The forge's own handle for the job, which is what asks for its log.
    pub handle: FromOutside,
    /// Where a person reads the job's log. `None` where the forge named none.
    pub log_url: Option<FromOutside>,
}

/// The pull request that put a commit on the base branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedPull {
    pub number: u64,
    /// `None` where only the merge commit's own message named it.
    pub url: Option<FromOutside>,
    /// The branch it came from, where the forge said.
    pub branch: Option<FromOutside>,
}

/// The CI jobs on one commit. `None` from the call that returns it is a forge
/// that would not answer; an empty list is a commit nothing ran on.
pub type CiRuns = Vec<CiRun>;

/// One open pull request, as the forge lists it.
///
/// **`ci` is the check named `ci` where there is one, otherwise every check
/// together**: a repository's gate is the check people read, and the rest report
/// beside it. `None` is a pull request nothing has run on, which is not a pass.
/// [`failing`](OpenPull::failing) is every check that failed, whichever decided
/// `ci`, because whether a failure is the branch's own is a question about all
/// of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenPull {
    pub number: u64,
    pub title: FromOutside,
    pub branch: FromOutside,
    pub url: FromOutside,
    /// The login of whoever opened it, where the forge named one.
    pub author: Option<FromOutside>,
    pub ci: Option<CiState>,
    /// The commit its branch is at, where the forge named one: what a status
    /// is put on.
    pub head: Option<String>,
    pub failing: Vec<FromOutside>,
    /// The forge will merge it by itself once its checks pass.
    pub auto_merge: bool,
    /// What the same listing says beyond its checks.
    pub facts: PullFacts,
}

/// What the forge's listing of open pull requests says of each beyond its
/// checks, **read on the same call**: no field here costs a second ask. Every
/// one is absent where the forge did not name it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PullFacts {
    /// When it was opened, as the forge wrote it (RFC 3339).
    pub opened_at: Option<FromOutside>,
    /// When anything last happened on it, as the forge wrote it (RFC 3339).
    pub updated_at: Option<FromOutside>,
    /// Opened as a draft, and not yet marked ready.
    pub draft: bool,
    pub mergeable: Option<PullMergeable>,
    pub review: Option<PullReview>,
    pub additions: Option<u32>,
    pub deletions: Option<u32>,
}

/// Whether the forge can merge a pull request as it stands, as its listing says.
/// **`Unknown` is the forge still computing**, which it does for a while after
/// a push, and is not a conflict nobody has found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullMergeable {
    Clean,
    Conflicting,
    Unknown,
}

/// The forge's review decision on a pull request. `None` is a decision the
/// forge does not ask for: no review rule applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullReview {
    Approved,
    ChangesRequested,
    ReviewRequired,
    None,
}

/// Where the forge's merge queue holds a pull request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueState {
    /// A state the forge named that this build does not know: in the queue, nothing more.
    InQueue,
    Queued,
    AwaitingChecks,
    Mergeable,
    Unmergeable,
}

/// A whole commit id: 40 or 64 hex digits, and nothing else, so it is never read by git as an
/// option. `Copy`, so a queue entry stays so.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Oid {
    digits: [u8; 64],
    len: u8,
}

impl Oid {
    /// `None` for anything but a whole commit id.
    pub fn parse(text: &str) -> Option<Oid> {
        let whole = matches!(text.len(), 40 | 64) && text.bytes().all(|byte| byte.is_ascii_hexdigit());
        if !whole {
            return None;
        }
        let mut digits = [0; 64];
        digits[..text.len()].copy_from_slice(text.to_ascii_lowercase().as_bytes());
        Some(Oid { digits, len: text.len() as u8 })
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.digits[..usize::from(self.len)]).unwrap_or_default()
    }
}

impl core::fmt::Debug for Oid {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "Oid({})", self.as_str())
    }
}

/// One entry in the base branch's merge queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueEntry {
    pub number: u64,
    /// 1 is the next to merge.
    pub position: u32,
    pub state: QueueState,
    /// The pull request's head as the queue holds it. `None` where the forge named none.
    pub head: Option<Oid>,
}

/// The merge queue, in order. `None` from the call that returns it is a forge
/// that would not answer; an empty list is a queue nothing waits in, or a
/// repository with none.
pub type MergeQueue = Vec<QueueEntry>;

/// The open pull requests on a repository. `None` from the call that returns
/// it is a forge that would not answer.
pub type OpenPulls = Vec<OpenPull>;

/// One pull request recently merged into the base, as the forge lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentlyMerged {
    pub number: u64,
    pub title: FromOutside,
    pub branch: FromOutside,
    pub url: FromOutside,
    pub author: Option<FromOutside>,
    /// When the forge says it merged, as the forge wrote it (RFC 3339).
    pub merged_at: FromOutside,
    /// The merge commit, whole, where the forge named one.
    pub commit: Option<FromOutside>,
}

/// The newest merged pull requests, newest first. `None` from the call that
/// returns it is a forge that would not answer.
pub type RecentlyMergedPulls = Vec<RecentlyMerged>;
