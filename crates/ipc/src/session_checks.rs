//! The Checks a Session's agent runs with `armada check` in the slot it holds. Since 23.75.
//!
//! **`armada check` runs in the caller's own process and writes no record**, so it tells Fleet
//! twice: when a run begins and when it ends, with the log. Fleet places the run by the directory
//! it ran in, never by anything the call claims, and answers no run where no Session holds that
//! slot. The run is then a thread row on the Session and a row on the Checks page.

use serde::{Deserialize, Serialize};

/// How a Session's run stands. The thread row and the Checks page both spell it so.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionCheckState {
    Running,
    Passed,
    Failed,
}

impl SessionCheckState {
    pub fn as_wire(self) -> &'static str {
        match self {
            SessionCheckState::Running => "running",
            SessionCheckState::Passed => "passed",
            SessionCheckState::Failed => "failed",
        }
    }
}

/// Which end of a run is being told.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionCheckAct {
    Start,
    /// What the run has printed since the last one. **Also the run's heartbeat**: Fleet answers it
    /// with a stop where a person pressed one, and `armada check` has no other way to be told.
    Chunk,
    End,
}

/// How a person asked a running Check to stop. `Force` outranks `Graceful`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionCheckStop {
    /// `SIGTERM` to the Check's process group, then the usual `end`.
    Graceful,
    /// `SIGKILL` to the group.
    Force,
}

/// `report_session_check`: `POST /sessions/checks`. `armada check` is the caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCheckCall {
    pub act: SessionCheckAct,
    /// The directory the Check ran in, which places the Session by the slot it stands in.
    pub cwd: String,
    /// The Check's key, as `armada check` took it.
    pub name: String,
    /// What the run was narrowed to: the test filter verbatim, or `--changed ` and the packages
    /// the change reached. Absent where it ran whole. **`end` may tell it when `start` could not**,
    /// since a `--changed` run only knows what it reached once it has resolved it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The run `start` answered. Required on `chunk` and `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<u64>,
    /// `passed` or `failed`. Required on `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<SessionCheckState>,
    /// How long it ran, in milliseconds. Only on `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_ms: Option<u64>,
    /// What the Check printed: all of it on `end`, what is new since the last one on `chunk`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
}

/// Where a failed run's log says it failed: the first location the failure names, relative to the
/// slot. Fleet reads it once, as the run ends, and never guesses one from the run's `target`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckBlame {
    pub path: String,
    /// 1-based. Absent where the log names the file and no line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// What `report_session_check` answers. **`run` is absent where no Session holds the slot**, which
/// is not a refusal: a person's own `armada check` is the common case.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCheckAnswer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<u64>,
    /// A stop a person asked for, answered to `chunk` and to the stop routes. Sticky until `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<SessionCheckStop>,
}

/// One message on a Session run's log socket, `observe_session_check`.
///
/// **`LandOutputMessage`'s shape**: the lines and the end are `OutputLines` and `OutputClosed`, so
/// one reader draws a Job's Check, a merge line's and a Session's. An ended run is served too.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum SessionCheckOutputMessage {
    Opened(SessionCheckOutputOpened),
    Lines(crate::OutputLines),
    Closed(crate::OutputClosed),
}

/// The first message: whose log this is, and what the first read left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCheckOutputOpened {
    pub protocol_id: crate::ProtocolId,
    pub run: u64,
    pub name: String,
    /// Older lines the first read left out, because the window is bounded.
    pub skipped: u64,
}
