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
}

/// Where the forge's merge queue holds a pull request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueState {
    Queued,
    AwaitingChecks,
    Mergeable,
    Unmergeable,
}

/// One entry in the base branch's merge queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueEntry {
    pub number: u64,
    /// 1 is the next to merge.
    pub position: u32,
    pub state: QueueState,
}

/// The merge queue, in order. `None` from the call that returns it is a forge
/// that would not answer; an empty list is a queue nothing waits in, or a
/// repository with none.
pub type MergeQueue = Vec<QueueEntry>;

/// What a status Fleet publishes on a commit says. **Two words, not the
/// forge's four**: Fleet never fails a pull request on its own say, it makes
/// one wait or lets it through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusState {
    Pending,
    Success,
}

/// One status Fleet puts on a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitStatus {
    pub commit: String,
    pub context: String,
    pub state: StatusState,
    pub description: String,
}

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
