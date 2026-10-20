//! A Session that waits on another and starts on its own. `docs/concepts/session.md`, *A Session that
//! waits on another*.
//!
//! **The wait is a fact on the waiting Session's record until its condition is met.** It holds its
//! brief, takes no slot and runs no process; Fleet watches what it observes of the Session waited on
//! (its state, its plan, its pull requests) and starts it then. The Session waited on is told what the
//! waiting one needs and may change the condition and where it starts from.

use serde::{Deserialize, Serialize};

use crate::session_starts::SessionLink;
use crate::sessions::SessionId;

/// What starts a waiting Session. **`ends` is the default**: a wait that names nothing else.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StartCondition {
    /// The Session waited on ends.
    #[default]
    Ends,
    /// Every task of one group of its plan is done. `group` is the group's id in the plan.
    GroupDone { group: String },
    /// One task of its plan is done. `task` is the task's id in the plan.
    TaskDone { task: String },
    /// A pull request of the Session waited on is pushed.
    PrPushed,
    /// A pull request of the Session waited on merges.
    PrLands,
}

/// Where a waiting Session starts from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartBase {
    /// The repository's default branch.
    #[default]
    Main,
    /// The branch of the Session waited on's pull request.
    Branch,
}

/// Who last set a wait's condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitBy {
    You,
    Agent,
}

/// The Session a new Session waits on, and when it starts. On `start_session` and `start_another_session`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartAfter {
    /// The Session waited on. A Session Fleet hosts, in the same repository.
    pub session: SessionId,
    #[serde(default)]
    pub condition: StartCondition,
    #[serde(default)]
    pub base: StartBase,
    /// What the new Session needs of the one it waits on, in a line, read from the brief. The Session
    /// waited on is told it. Absent where the brief asks for nothing.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub needs: String,
}

/// The pull request a wait is on, once the Session waited on has one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitPull {
    pub number: u64,
    pub branch: String,
}

/// A Session dispatched to wait on another and not started: on its record until the condition is met.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWait {
    /// The Session waited on.
    pub on: SessionLink,
    pub condition: StartCondition,
    pub base: StartBase,
    /// As [`StartAfter::needs`].
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub needs: String,
    /// The words it was dispatched with, which it starts on.
    pub brief: String,
    /// The pull request a `pr_pushed` or `pr_lands` condition is on, once the Session waited on has
    /// one. Fleet picks the first and keeps it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull: Option<WaitPull>,
    /// The title of the group or task a `group_done` or `task_done` condition names, as the plan of the
    /// Session waited on has it now. Absent where the plan no longer holds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_title: Option<String>,
    pub by: WaitBy,
}

/// `read_session_wait`: the proposer reads a brief that tags a Session for the wait it names, before
/// anything is dispatched. Answered with [`WaitReading`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadSessionWait {
    /// The Session the brief tags with `@`.
    pub session_id: SessionId,
    /// The brief as typed.
    pub request: String,
}

/// What the proposer read of a brief that tags a Session. **A reading that cannot be used is the
/// defaults**: it ends, from main, asking nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitReading {
    pub condition: StartCondition,
    pub base: StartBase,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub needs: String,
    /// The brief tags the Session and names no condition, so the new one does not wait on it: the tag is
    /// a reference. Absent is a wait, which is every answer from before the flag and every reading that
    /// could not be had.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub plain: bool,
}

/// `change_waiting`: the condition a waiting Session starts on, or where it starts from. Answered with
/// the [`SessionWait`] as it now stands.
///
/// **From the agent of the Session waited on, or from the person.** An agent's call is placed by its
/// connection and may only change a Session that waits on it; the person's may change any.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeWaiting {
    pub waiting_session: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<StartCondition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<StartBase>,
}

/// `start_waiting_now`: the person starts a waiting Session at once, whatever its condition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartWaitingNow {
    pub session_id: SessionId,
}

/// What a row in a thread says of a wait.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaiterEvent {
    /// A Session was dispatched to wait on this one.
    Waits,
    /// The condition or the base was changed.
    Changed,
    /// The condition was met and the Session started.
    Started,
}

/// Which of the two threads a `waiter` row is written in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaiterSide {
    /// In the thread of the Session waited on: `session` is the one that waits.
    #[default]
    Waited,
    /// In the thread of the waiting Session: `session` is the one it waits on.
    Waiting,
}

impl WaiterSide {
    pub fn is_waited(&self) -> bool {
        *self == WaiterSide::Waited
    }
}
