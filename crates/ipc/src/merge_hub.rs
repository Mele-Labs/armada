//! What the forge says about a repository, beside the queue `armada land`
//! keeps: where main's CI stands and which pull requests are open. It rides on
//! [`MergeLine`](crate::MergeLine) as `hub`, since 23.41.
//!
//! **Fleet's reading of the forge, redacted to what a person is shown.** A job
//! is named as the forge names it, and a Manifest Check is added only where one
//! maps. Nothing here is a log: a failed job's log is read on the merge line's
//! Check log socket, under the branch `main`.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, JobId};

/// One repository's main and open pull requests.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeLineHub {
    /// Absent where Fleet has not read main yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main: Option<MainStanding>,
    /// Every open pull request, newest first as the forge lists them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pull_requests: Vec<HubPullRequest>,
    /// The Job working on main's red. **Never set yet**: a Job picking the red
    /// up is a later change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing: Option<HubJob>,
}

/// A Job, by its id and what it is called.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubJob {
    pub id: JobId,
    pub title: String,
}

/// Where main's newest commit stands on the forge's CI.
///
/// **Strict, because Bridge branches on it**: green draws a mark, red a band.
/// A red a fix is still running for stays `red`; `running` is a commit nothing
/// has failed on yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainCiState {
    Green,
    Red,
    Running,
    /// The forge ran nothing on it. **Not green**: nothing was proved.
    NothingRan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainStanding {
    pub state: MainCiState,
    /// The commit read, whole.
    pub commit: String,
    pub read_at: Instant,
    /// When main first read red, kept while it stays red. `red` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub red_since: Option<Instant>,
    /// Each CI job that failed, in the forge's order. `red` only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<MainFailedJob>,
    /// The pull request that turned main red, where the forge named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge: Option<MainMerge>,
}

/// One CI job that failed on main.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainFailedJob {
    /// The job's name as the forge reports it.
    pub name: String,
    /// The Manifest Check it maps to. **Absent is ordinary**: CI Armada has no
    /// Check for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_url: Option<String>,
    /// The tests its log names, in the order printed. **Absent where none could
    /// be read**, and never a guess.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tests: Vec<String>,
}

/// The pull request that put the red commit on main.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainMerge {
    pub number: u64,
    /// Absent where only the merge commit's own message named it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// The Job whose pull request that is. Absent for a person's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<HubJob>,
}

/// One open pull request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubPullRequest {
    pub number: u64,
    pub title: String,
    pub branch: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Absent where nothing has run on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci: Option<HubPullCi>,
    /// The Job that opened it. Absent for a person's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<HubJob>,
}

/// How a pull request's `ci` stands.
///
/// **Strict**: Bridge picks a mark from it. `waiting_on_main` is a `ci` that
/// failed only on the jobs main is failing, so it is the fix's to wait for and
/// not the branch's failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HubPullCi {
    Passed,
    Running,
    Failed,
    WaitingOnMain,
}
