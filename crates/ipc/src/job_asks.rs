//! `raise_conflict` and `ask_held`: the two things a Job may send the Operator, each a typed act
//! Fleet checks before anything reads it. `docs/concepts/operator.md`, *A Job's two asks*.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// `raise_conflict`: this Job's work and another Job's cannot both stand.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaiseConflict {
    /// The other Job's address, `s-` and eight characters. The sender is the first side.
    pub with: String,
    /// The paths the clash is in.
    #[serde(default)]
    pub files: Vec<String>,
    /// The wire type names the clash is in. At least one of `files` and `wire` is named.
    #[serde(default)]
    pub wire: Vec<String>,
    /// What this Job's work does, in one sentence.
    pub mine: String,
    /// What the other Job's work does, in one sentence.
    pub theirs: String,
    /// What fails if both land, in one sentence.
    pub breaks: String,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `raise_conflict` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictRaised {
    /// `conflict:<a>:<b>`, the two addresses sorted.
    pub id: String,
    /// An open conflict between the same two Jobs took this one as "also raised by", so nothing new is shown.
    pub folded: bool,
}

/// `ask_held`: should this pull request or branch land?
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskHeld {
    /// The pull request's number. Exactly one of `pull` and `branch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull: Option<u64>,
    /// The branch's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `ask_held` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldAsked {
    /// `held:<repository>:<pull or branch>`.
    pub id: String,
    /// Fleet's answer where it already knew it, such as `held? #12: merged at 14:05.`: the Job is sent the same words and
    /// nothing goes to the Operator. Absent where the question went on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
}

/// `withdraw_held`: the Job takes back a landing question it asked and nothing has answered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawHeld {
    /// The pull request's number. Exactly one of `pull` and `branch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull: Option<u64>,
    /// The branch's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `withdraw_held` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldWithdrawn {
    /// `held:<repository>:<pull or branch>`.
    pub id: String,
}

/// `settle_clash`: two Jobs that write the same files agreed which of them lands first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettleClash {
    /// The other Job's address, `s-` and eight characters, as the claims notice named it.
    pub with: String,
    /// The paths the two agreed on. A folder covers the files under it.
    pub files: Vec<String>,
    pub first: ClashFirst,
    /// Why, in one sentence.
    pub why: String,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// Which of the two lands first, from the point of view of the Job that calls `settle_clash`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClashFirst {
    Me,
    Them,
    BothGoOn,
}

/// How a clash between two Jobs stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClashStanding {
    /// The two agreed, with `settle_clash`.
    Agreed,
    /// One raised it with `raise_conflict` naming files, and the person chose which goes first.
    Ruled,
    /// One raised it with `raise_conflict` naming files, and the person has not chosen yet.
    Waiting,
}

/// Two Jobs that write the same files, and the order they settled. **The same on both Jobs' records.** Answered by
/// `settle_clash`, and carried on `SessionRecord::clashes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobClash {
    /// `clash:<a>:<b>`, the two addresses sorted.
    pub id: String,
    /// The Job that settled it or raised it, then the other.
    pub jobs: Vec<OperatorAskJob>,
    pub files: Vec<String>,
    pub standing: ClashStanding,
    /// The Job that lands first, the other waiting until it lands. Absent where both go on, and while it waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<SessionId>,
    /// Their one sentence on why, or for a raised clash what fails if both land. Empty where none was given.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub why: String,
    /// When it was last settled or raised.
    pub at: String,
}

/// A Job on a card of Your turn, as a chip: its title and its address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorAskJob {
    pub session_id: SessionId,
    /// `s-` and eight characters, as `raise_conflict` names a Job.
    pub address: String,
    /// The Job's title as it stood when the ask was made. Empty where it had none.
    pub title: String,
}

/// A conflict as Your turn's card draws it: it leads with `breaks`, shows the two Jobs as chips, and folds `files` and
/// `wire` behind a press.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorConflict {
    /// The Job that raised it, whose side `mine` is.
    pub raised_by: OperatorAskJob,
    /// The other Job, whose side `theirs` is.
    pub with: OperatorAskJob,
    pub files: Vec<String>,
    pub wire: Vec<String>,
    pub mine: String,
    pub theirs: String,
    pub breaks: String,
}

/// Where a pull request's checks stand, as the forge showed them when it was asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeldChecks {
    Passed,
    Pending,
    Failed,
}

/// A held? as Your turn's card draws it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorHeld {
    /// The pull request asked about. Exactly one of `pull` and `branch` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// The pull request's title, where one was read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Its checks, where the forge said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<HeldChecks>,
    /// The Job that asked.
    pub asked_by: OperatorAskJob,
}
