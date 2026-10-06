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
