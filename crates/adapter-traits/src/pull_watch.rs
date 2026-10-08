//! What Fleet watches on each open pull request to tell its owner when it fails: the checks on
//! its newest commit, whether it conflicts with the base, and where the forge's merge queue holds
//! it. `docs/concepts/fleet.md`, *Telling the owner of a pull request*.
//!
//! **The forge's facts and nothing else**, `crate::main_ci`'s rule: a check is named as the forge
//! names it, and which owner to tell is Fleet's question. Every word a forge wrote arrives as
//! [`FromOutside`].

use alloc::string::String;
use alloc::vec::Vec;

use crate::{CiState, FromOutside};

/// One check on a pull request's newest commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchedCheck {
    pub name: FromOutside,
    pub state: CiState,
    /// The ruleset gates the pull request on this one: the check named `ci` where there is one,
    /// else every check.
    pub required: bool,
    /// It ended cancelled after about the whole time a job is given, which is a runner that hung
    /// and not a failure of the change.
    pub hung: bool,
    /// Where a person reads its log, where the forge named one.
    pub log_url: Option<FromOutside>,
}

/// Where the forge's merge queue holds a pull request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullQueue {
    /// Not in the queue.
    Outside,
    /// In the queue and going on.
    Waiting,
    /// In the queue, and the forge marked it unmergeable.
    Unmergeable,
}

/// One open pull request, as watched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchedPull {
    pub number: u64,
    pub branch: FromOutside,
    pub url: FromOutside,
    /// The commit its branch is at, where the forge named one.
    pub head: Option<String>,
    /// The forge says it cannot merge into the base as it stands. An answer not yet computed is
    /// not a conflict.
    pub conflicting: bool,
    pub queue: PullQueue,
    pub checks: Vec<WatchedCheck>,
}

impl WatchedPull {
    /// A check that decides the pull request has failed.
    pub fn failed(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.required && check.state == CiState::Failed)
    }
}

/// The open pull requests, watched. `None` from the call that returns it is a forge that would
/// not answer.
pub type WatchedPulls = Vec<WatchedPull>;
