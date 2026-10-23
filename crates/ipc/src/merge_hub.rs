//! What the forge says about a repository, beside the queue lists: where main's CI stands and which pull requests are open. It rides on
//! [`MergeLine`](crate::MergeLine) as `hub`, since 23.41.
//!
//! **Fleet's reading of the forge, redacted to what a person is shown.** A job
//! is named as the forge names it, and a Manifest Check is added only where one
//! maps. Nothing here is a log: a failed job's log is read on the merge line's
//! Check log socket, under the branch `main`.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};

/// One repository's main and open pull requests.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    /// The Session working on main's red: the newest that took it and has not ended. Absent
    /// while nobody has it, and once main is green.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing_session: Option<HubSession>,
    /// Another machine's claim on main's red, read from the central store. Absent
    /// while nobody else holds it, where the repository's *Pull request owners*
    /// switch is off, the machine is signed out, and whenever the store could
    /// not be read: **an unreadable store yields no field and never an error**.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing_elsewhere: Option<FixingElsewhere>,
    /// The checkout Fleet serves, where main moved past it and Fleet would not
    /// fast-forward it. Absent while it has main's tip, and from an older Fleet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkout: Option<HeldCheckout>,
    /// When Fleet last got an answer from the forge for every call of its visit to this
    /// repository, main and pull requests alike: **how old everything above is**, as `main`'s
    /// own `read_at` moves only when the head does. Absent until a visit has fully answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forge_read_at: Option<Instant>,
    /// When the forge first failed to answer a call of the visit, since the last visit it
    /// answered whole. Absent while it answers, so what is above is no staler than a visit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_since: Option<Instant>,
}

/// What stops the served checkout catching up to main. Fleet never forces,
/// stashes or switches it; a person does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldCheckout {
    pub base: String,
    /// The branch checked out, where it is not `base`. `HEAD` when detached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<String>,
    /// Tracked files with uncommitted changes, as paths from the root.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uncommitted: Vec<String>,
    /// Those of `uncommitted` that main also changed, so an autostash could
    /// stop on a conflict.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clashing: Vec<String>,
    /// Commits main has that the checkout does not.
    pub behind: u32,
    /// Commits the checkout has that main does not. Above zero is diverged.
    pub ahead: u32,
    /// The pull request Fleet opened from the uncommitted files, on the person's
    /// press of `checkout_to_pull_request`. Absent until then, and once the hold
    /// clears.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<CheckoutPullRequest>,
}

/// The pull request a held checkout's uncommitted files went up as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutPullRequest {
    pub number: u32,
    /// Whether the forge took auto-merge. A pull request whose auto-merge it
    /// refused is open all the same, for a person to merge.
    pub auto_merge: bool,
}

/// `checkout_to_pull_request`'s body. **Names a repository and nothing more**,
/// for `CatchUpCheckout`'s reason: Fleet reads the hold again and sends the
/// files it lists, so no path crosses the wire.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutToPullRequest {
    /// The repository, by its Manifest.
    pub manifest_id: ManifestId,
}

/// `checkout_to_pull_request`'s answer: the send was accepted. **The pull request is not in it**:
/// the commit, the push and the pull request finish after the answer, and the hold carries the
/// pull request (`HeldCheckout.pull_request`) when they have. Where they fail, the stream says
/// `act.failed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutSentAsPullRequest {}

/// `catch_up_checkout`'s body: pull a held checkout level with its base, the one
/// way that cannot lose a person's files. **Names a repository and nothing more.**
/// There is no command in it: Fleet reads the hold again and runs the pull it
/// knows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatchUpCheckout {
    /// The repository, by its Manifest.
    pub manifest_id: ManifestId,
}

/// `catch_up_checkout`'s answer: what the pull came to. **A pull git refused or
/// left conflicted is still an answer**, with a non-zero `code`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutCaughtUp {
    /// git's exit code. `1` where git exited clean and the stashed work would not
    /// go back on; the work is in git's stash and nothing dropped it.
    pub code: i32,
    /// What git printed, for the person to read as it came.
    pub output: String,
}

/// `checkout_diff`'s query: one uncommitted file of a held checkout. **Only a path
/// the hold lists** is answered; any other is refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutDiffAsk {
    /// The repository, by its Manifest.
    pub manifest_id: ManifestId,
    /// The file, as the hold's `uncommitted` names it: from the repository's root.
    pub path: String,
}

/// `checkout_diff`'s answer: the file's change against `HEAD`, as git prints a
/// unified diff. Empty where the file no longer differs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutDiff {
    pub diff: String,
    /// Set where Fleet cut `diff` short of what git printed, at a line or byte
    /// bound. The text is still whole lines of the start of the change.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
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

/// A Session, by its short id.
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
    /// The Session that held the pull request or its branch. Since 23.80.
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
    /// The Session that claimed it, from the central store. **Absent** where nobody
    /// registered it, where the repository's switch is off (rows another machine wrote
    /// included), and whenever the store could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<PullRequestOwner>,
    /// The pull request of an `armada.yml` edit a form sent, which the forge would not turn
    /// auto-merge on for, so it waits for a person to merge it. Absent is any other pull request,
    /// and one the forge is going to merge itself.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub merge_by_hand: bool,
    /// Opened by the person using this Fleet: its author is the login Armada is signed in as for
    /// the repository or the one the forge's command line is signed in as, or either is unknown.
    /// **Fleet decides, and tells, checks and draws only the ones that are.** A pull request that
    /// is not is shown only in the merge queue, quietly. Absent is mine, and what an older Fleet
    /// always sends.
    #[serde(default = "mine", skip_serializing_if = "is_mine")]
    pub mine: bool,
}

fn mine() -> bool {
    true
}

fn is_mine(it: &bool) -> bool {
    *it
}

/// Who registered a pull request. **A kind, a short id and a machine's name, and nothing
/// about the pull request**: the store holds no title, repository or code, and only the
/// machine that holds the claim can say what it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestOwner {
    pub kind: crate::sessions::HolderKind,
    /// The Session's short id.
    pub id: String,
    /// The host's name, or `machine.name` where two hosts share one.
    pub machine: String,
    /// The row was written by this machine.
    pub this_machine: bool,
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
