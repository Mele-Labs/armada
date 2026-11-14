//! `answer_items` and `dismiss_items`: his verdict, or no verdict, on several waiting items at once.
//! `crates/ipc/operations/answer_items.toml` is the contract, `docs/concepts/artifact.md` the rules.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// `answer_items`: a verdict on each item, settled in order and each on its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerItems {
    pub items: Vec<ItemAnswer>,
}

/// One item and his verdict on it. `notes` go with that verdict only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemAnswer {
    pub session_id: SessionId,
    /// The `WaitingItem` id: `sketch:<title>`, `document:<path>`, `plan:<version>`, `draft:<row>`, `walk:<url>`, or an
    /// agent item's own.
    pub item: String,
    pub how: ItemHow,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<ItemNote>,
}

/// One note of a verdict. A note from a card's note box has no `on`; one made on words of a page, or on a part of a
/// sketch, names them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemNote {
    pub text: String,
    /// Where the note sits. A quote blank after trimming counts as no `on`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<NoteOn>,
}

/// The words a note points at, and the block they are in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteOn {
    /// The words he marked, as the page shows them.
    pub quote: String,
    /// The page block's id, or the sketch part's id, the note sits on.
    pub block: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemHow {
    Approve,
    SendBack,
}

/// `dismiss_items`: each item off his list with no verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DismissItems {
    pub items: Vec<ItemRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRef {
    pub session_id: SessionId,
    pub item: String,
}

/// The answer to both: one outcome per item asked, in the order asked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemsSettled {
    pub outcomes: Vec<ItemOutcome>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemOutcome {
    pub session_id: SessionId,
    pub item: String,
    pub outcome: ItemSettled,
}

/// How one item ended. A refusal leaves that item as it stood.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemSettled {
    Settled,
    Refused { code: String, message: String },
}
