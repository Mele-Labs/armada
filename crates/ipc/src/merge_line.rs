//! The merge line `armada land` keeps in each repository Fleet serves —
//! `get_merge_lines` and `merge_lines.changed` in `crates/ipc/operations.toml`.
//!
//! **A reading of files another process writes**, not Fleet's own state: the
//! runner is `armada land`, and Fleet reads its `armada-land/` directory and
//! starts nothing. `docs/capabilities/merge-line.md`, *In Bridge*.

use serde::{Deserialize, Serialize};

/// Every served repository that has a line, the one Fleet was started in first.
///
/// **A repository nobody has run `armada land` in is not here**, rather than
/// here with nothing in it: there is no line to draw and no rail row to offer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeLines {
    pub lines: Vec<MergeLine>,
}

/// One repository's line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeLine {
    /// The repository's root, as `list_repositories` names it.
    pub root: String,
    /// Waiting, or in a turn, in place order. Empty is a line with nobody in it.
    pub line: Vec<MergeLineEntry>,
    /// The newest few that left the line with an outcome, newest first, as
    /// one list. What a Bridge before 23.2 draws; `landed` and `sent_back`
    /// replace it.
    pub off: Vec<MergeLineEntry>,
    /// The newest few that left the line landed, newest first. Since 23.2.
    #[serde(default)]
    pub landed: Vec<MergeLineEntry>,
    /// Red, conflict or stopped and not back in line, written within the last
    /// three days, newest first. Since 23.2.
    #[serde(default)]
    pub sent_back: Vec<MergeLineEntry>,
}

/// One branch, in line or just off it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeLineEntry {
    pub branch: String,
    /// 1-based. Absent once the branch has left the line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<u32>,
    /// Absent where the branch has none, or `origin` is not on the forge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<MergeLinePullRequest>,
    pub state: LandState,
    /// What the runner says it is doing, in its own words, without the batch.
    /// Only while `gating` or `merging`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doing: Option<String>,
    /// The batch it gates in, by the branch first in place order. Absent alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch: Option<String>,
    /// `landed`: the merge commit, whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_commit: Option<String>,
    /// `red` and `stopped`: the Checks that failed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<String>,
    /// `conflict`: the files main did not merge into.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    /// `gating`, `red` and `stopped`: each Check the turn runs, as it stands,
    /// in the order they run. Since 23.2.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<MergeLineCheck>,
}

/// One Check a turn runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeLineCheck {
    /// Its name in `armada.yml`.
    pub name: String,
    pub state: LandCheckState,
}

/// Where one Check stands in a turn: `waiting` until it is run, then
/// `running`, then one of the three ends.
///
/// **Strict, for [`LandState`]'s reason**: Bridge draws each as a segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandCheckState {
    Waiting,
    Running,
    Passed,
    Failed,
    TimedOut,
}

/// A pull request, by number and address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeLinePullRequest {
    pub number: u64,
    pub url: String,
    /// `landed` only: how it ended, the Job's own `Settled`. Absent is
    /// nothing known, which is every pull request still in line. Since 23.2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<crate::Settled>,
}

/// Where one branch is on the line: `land_state` in `enum-verbs.toml`.
///
/// **Strict, because Bridge branches on it** — the live pair pulses and each
/// end state draws its own facts — so a new state is a major move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LandState {
    Waiting,
    Gating,
    Merging,
    Landed,
    Red,
    Conflict,
    Stopped,
}

/// One message on a merge line Check's log socket, `observe_land_check`.
///
/// **`OutputMessage`'s three, with its own first message.** A turn's Check
/// belongs to no Job, so the opening names the line instead: the repository,
/// the branch and the Check. The lines and the end are `observe_check_output`'s
/// own, so one reader draws both. Since 23.7.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum LandOutputMessage {
    Opened(LandOutputOpened),
    Lines(crate::OutputLines),
    Closed(crate::OutputClosed),
}

/// The first message: whose log this is, and what the first read left out.
///
/// **No path.** The runner's logs sit in the clone's common directory, and
/// `get_merge_lines` keeps every path in there off the wire; the three names
/// the reader asked by are what it is told.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandOutputOpened {
    pub protocol_version: crate::ProtocolVersion,
    /// The repository's root, as `list_repositories` names it.
    pub root: String,
    pub branch: String,
    /// The Check, as `MergeLineCheck::name` spells it.
    pub name: String,
    /// Older lines the first read left out, because the window is bounded.
    pub skipped: u64,
}
