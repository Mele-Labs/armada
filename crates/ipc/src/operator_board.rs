//! The Operator's events: the page is not what it was, and what became of a dispatch it was told of.
//! `crates/ipc/operations/operator.board_changed.toml`.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;

/// `operator.board_changed`: the board is not what it was, so read it again. Nothing else rides.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorBoardChanged {}

/// What became of a dispatch the Operator held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchWay {
    /// A Job was started on it.
    Started,
    /// It was folded into a live Job.
    Handed,
    /// It waits on the person's card.
    Asked,
    /// It was kept for later as an idea, and no Job was started.
    Parked,
}

/// `operator.dispatched`: what the Operator did with a dispatch it held (`dispatch_job`'s `operator`): started a
/// Job, handed it to a live one, kept it for later (`way` `parked`, `dispatch_id` then the idea's id) or asked him.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorDispatched {
    /// The `operator` `dispatch_job` answered with.
    pub dispatch_id: String,
    pub manifest_id: String,
    pub way: DispatchWay,
    /// The Job started, or the live Job the work was folded into. Absent while it is `asked`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}
