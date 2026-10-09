//! The Checks a Session's agent runs with `armada check` in the slot it holds. Since 23.74.
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
    End,
}

/// `report_session_check`: `POST /sessions/checks`. `armada check` is the caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCheckCall {
    pub act: SessionCheckAct,
    /// The directory the Check ran in, which places the Session by the slot it stands in.
    pub cwd: String,
    /// The Check's key, as `armada check` took it.
    pub name: String,
    /// The run `start` answered. Required on `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<u64>,
    /// `passed` or `failed`. Required on `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<SessionCheckState>,
    /// How long it ran, in milliseconds. Only on `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_ms: Option<u64>,
    /// What the Check printed. Only on `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
}

/// What `report_session_check` answers. **`run` is absent where no Session holds the slot**, which
/// is not a refusal: a person's own `armada check` is the common case.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCheckAnswer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<u64>,
}
