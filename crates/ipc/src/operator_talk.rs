//! The box at the foot of the Operator panel: what the person types, what it says back, and the
//! standing orders he confirms. `crates/ipc/operations/talk_to_operator.toml`.
//!
//! **An order is only ever one of these forms**, which Fleet enforces. No prose reaches a call.

use serde::{Deserialize, Serialize};

use crate::operator_positions::OperatorRole;
use crate::SessionId;

/// What lifts a hold on paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorHoldUntil {
    Pushes,
    Ends,
}

/// A standing order, in one of the forms Fleet enforces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperatorOrderForm {
    /// A role acts on nothing in one repository, until the day ends or he removes it.
    RoleOff {
        role: OperatorRole,
        manifest_id: String,
        #[serde(default)]
        today: bool,
    },
    /// Only `job` writes under `paths` in its repository, until it pushes or ends. A Job that ends
    /// releases the hold whichever `until` says.
    HoldPaths { manifest_id: String, paths: Vec<String>, job: SessionId, until: OperatorHoldUntil },
    /// Arrivals of the kind `what`, as the Log names them, go to him with no call.
    AlwaysRaise { what: String },
}

impl OperatorOrderForm {
    /// The form as the store keeps it.
    pub fn kept(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// What the store kept. `None` where it is not a form, which then stands for nothing.
    pub fn read(text: &str) -> Option<OperatorOrderForm> {
        serde_json::from_str(text).ok()
    }
}

/// One standing order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorOrder {
    pub id: i64,
    pub form: OperatorOrderForm,
    /// The form in Fleet's words.
    pub said: String,
    /// When an order for today stops standing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    pub at: String,
}

/// `operator_orders`: the orders standing now, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorOrders {
    pub orders: Vec<OperatorOrder>,
}

/// `confirm_operator_order`: the person saw this form and presses to have it stand.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmOperatorOrder {
    pub form: OperatorOrderForm,
}

/// `remove_operator_order`: one press removes an order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveOperatorOrder {
    pub id: i64,
}

/// `talk_to_operator`: one message from the person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TalkToOperator {
    pub words: String,
    /// A board item's id or a Needs you item's id: the person is talking about that one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
}

/// The Log row an answer rests on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorCite {
    pub arrival: String,
    pub session_id: String,
    pub subject: String,
    /// As the Log names it, such as `merge` or `raise`.
    pub outcome: String,
    pub because: String,
    pub at: String,
}

/// A form the Operator read in what he typed, standing only once he confirms it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedOperatorOrder {
    pub form: OperatorOrderForm,
    pub said: String,
}

/// What the Operator said back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSaid {
    pub said: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cites: Vec<OperatorCite>,
    /// The brief of a Job it offers to start, where an answer needs the code read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    /// The Job he told it to start, which has been started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<ProposedOperatorOrder>,
    /// Every form there is, where he asked for an order that fits none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forms: Vec<String>,
    /// The board item it put first in `next`, as he asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moved: Option<String>,
}

/// One message of the conversation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTalkLine {
    pub from_person: bool,
    pub text: String,
    pub at: String,
    /// The item the message was about, where it named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
}

/// `operator_talk`: the conversation so far, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTalk {
    pub lines: Vec<OperatorTalkLine>,
}
