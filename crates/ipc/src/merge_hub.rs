//! What the forge says about a repository, beside the queue lists: where main's CI stands and which pull requests are open. It rides on
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
    /// Another machine's claim on main's red, read from the central store. Absent
    /// while nobody else holds it, where the repository's *Pull request owners*
    /// switch is off, the machine is signed out, and whenever the store could
    /// not be read: **an unreadable store yields no field and never an error**.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing_elsewhere: Option<FixingElsewhere>,
}

/// Who holds main's red on another machine. **No title and no work**: the store
/// holds neither, and only the machine that holds a claim can say what it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixingElsewhere {
    /// The host's name, or `machine.name` where two hosts share one.
    pub machine: String,
    pub holder: ClaimHolder,
    /// When it took the red, so a stale claim reads as stale.
    pub since: Instant,
}

/// A Job or a Session, by its short id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimHolder {
    pub kind: crate::sessions::HolderKind,
    pub id: String,
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

/// A Session, by its id and the title the ledger holds for it. Since 23.80.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubSession {
    pub id: String,
    /// Absent where the Session has none; Bridge names it by the branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
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
    /// The newer commits whose CI is still running, newest first, since 23.44.
    /// **Main's state is the newest commit whose run has finished.** On a `red`,
    /// non-empty is a held red: a newer run may already have fixed it, so Fleet
    /// refuses to hand it to a Job and Bridge draws the band as caution with no
    /// buttons. On a `green` it is only checks running. Empty is main as `state`
    /// says, and what an older Fleet always sends.
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
    /// The Session that held the pull request or its branch, where no Job owns it. Since 23.80.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<HubSession>,
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
    /// Where it stands in the merge queue, since 23.70. Absent where it is not
    /// in the queue and has no auto-merge waiting on its checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<HubQueue>,
    /// When the forge says it was opened. Absent from an older Fleet, and where
    /// the forge named none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened_at: Option<Instant>,
    /// When anything last happened on it, the forge's reading. Absent as `opened_at` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<Instant>,
    /// Still a draft. Absent is ready, and what an older Fleet always sends.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub draft: bool,
    /// Whether the forge can merge it as it stands. Absent where the listing
    /// named none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mergeable: Option<HubMergeable>,
    /// The forge's review decision. Absent where the listing carried none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<HubReview>,
    /// Lines added and removed, as the forge counts them. Each absent where it did not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additions: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletions: Option<u32>,
}

/// Whether the forge can merge a pull request as it stands.
///
/// **Strict**: Bridge picks a mark from it. `unknown` is the forge still
/// computing, as it does for a while after a push, and is not a conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HubMergeable {
    Clean,
    Conflicting,
    Unknown,
}

/// The forge's review decision on a pull request. **Strict**: Bridge picks a
/// mark from it. `none` is a pull request no review rule applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HubReview {
    Approved,
    ChangesRequested,
    ReviewRequired,
    None,
}

/// One pull request's place in the merge queue. Since 23.70.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubQueue {
    pub state: HubQueueState,
    /// 1 is the next to merge. Absent for `waiting_for_ci`, which has no entry yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
}

/// What the queue says of a pull request. **Strict**: Bridge picks a mark from
/// it. `waiting_for_ci` is a pull request with auto-merge on whose own `ci` is
/// still running, so it has not joined the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HubQueueState {
    WaitingForCi,
    InQueue,
    Queued,
    AwaitingChecks,
    Mergeable,
    Unmergeable,
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
    /// Replace another machine's claim on this red with this one's. Asked for,
    /// never assumed: the press follows a refusal that named the holder.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub take_over: bool,
}
