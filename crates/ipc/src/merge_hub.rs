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
    /// The newest pull requests merged into the base, newest first, read from
    /// the forge on the same visit as the open ones. Since 23.42. **Absent from
    /// an older Fleet**, and a Bridge then draws the line's own `landed`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merged: Vec<HubMerged>,
    /// The Job working on main's red, since 23.42: the newest Job that took it
    /// and has not been stopped. Absent while nobody has it, and once main is
    /// green.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing: Option<HubJob>,
}

/// One pull request recently merged into the base. Since 23.42.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubMerged {
    pub number: u64,
    pub title: String,
    pub branch: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub merged_at: Instant,
    /// The merge commit, whole. Absent where the forge named none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// The Job that opened it. Absent for a person's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<HubJob>,
    /// The CI run on the merge commit, on main itself, distinct from the pull
    /// request's own checks. Since 23.44. Absent where nothing ran on it, where
    /// it was not asked yet, or from an older Fleet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_run: Option<MainRun>,
}

/// The CI run on one commit of main, as the forge reports it. Since 23.44.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainRun {
    pub state: MainRunState,
    /// Each job that failed, by the forge's name, in its order. `failed` only.
    /// A press on one opens its log on the Check log socket under the branch
    /// `main@<commit>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<String>,
}

/// How a commit's run on main stands. **Strict**: Bridge picks a mark from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainRunState {
    Passed,
    Running,
    Failed,
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
/// has failed on yet. **A red a newer commit's CI is running on is `red` too**,
/// with `checking` naming those commits since 23.44, so a Bridge before it still
/// reads the variants it knows.
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
    /// The commit the red was read at, whole. `red` only, since 23.44. It is
    /// `commit` until a newer one lands, and then the red's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub red_commit: Option<String>,
    /// The newer commits whose CI is still running, newest first. `red` only,
    /// since 23.44. **Non-empty is a held red**: a newer run may already have
    /// fixed it, so Fleet refuses to hand it to a Job and Bridge draws the
    /// band as caution with no buttons. Empty is a red to act on, and what an
    /// older Fleet always sends.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checking: Vec<MainChecking>,
}

/// A commit on main, newer than the red one, whose CI is still running.
/// Since 23.44.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainChecking {
    /// The commit, whole.
    pub commit: String,
    /// The pull request that merged it, where it is among the newest merged
    /// and the forge named one. Absent for a direct push.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<MainMerge>,
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

/// A Job's part in main's red, on its summary as `fixes_main`. Since 23.42.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixesMain {
    pub state: FixesMainState,
    /// The Manifest Check that was red, or the CI job's own name where none maps.
    pub check: String,
    /// The first failing test the log named. Absent where none could be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    /// The pull request that turned main red. Absent where none was named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge: Option<u64>,
    /// The pull request of this Job that put main green. `fixed` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_in: Option<u64>,
}

/// Where a Job's part in the red stands. **Strict**: Bridge picks a mark from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixesMainState {
    Fixing,
    Fixed,
}

/// `fix_main`'s body: hand a repository's red main to a Job. Since 23.42.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixMain {
    /// The repository, as `get_merge_lines` names it.
    pub root: String,
    /// An earlier Job to send the work back to. Absent dispatches a new one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<JobId>,
    /// What the Job is told. Absent is the facts Fleet read, as the band shows
    /// them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brief: Option<String>,
}
