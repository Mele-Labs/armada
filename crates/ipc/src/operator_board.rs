//! The Operator's board: what it is ruling on, what waits for it and what it settled.
//! `crates/ipc/operations/get_operator_board.toml`.

use serde::{Deserialize, Serialize};

/// What kind of thing an item is. A plan and a Job's start are both a `start`, and work from outside a
/// Job's own run (an issue, a review request, a red main, a Later item) is a `report`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorBoardKind {
    Start,
    Question,
    Card,
    Stall,
    Report,
}

/// One arrival the Operator has not ruled on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorBoardItem {
    /// The arrival's id, which `move_on_operator_board` and `TalkToOperator.about` name it by.
    pub id: String,
    pub kind: OperatorBoardKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// What a person reads: the Job's title, or the first line of the question.
    pub title: String,
    /// The repository's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// How long it has been with the Operator, such as `12s` or `3m`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waits: Option<String>,
}

/// One ruling the Operator made, kept in the Log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorBoardSettled {
    pub id: String,
    pub kind: OperatorBoardKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Written `Kind: what`, a one or two word kind and a short what, or the kind alone.
    pub outcome: String,
    /// RFC 3339.
    pub at: String,
    /// Who settled it: the Operator's ruling, or the person's press on a suggestion.
    #[serde(default)]
    pub by: OperatorDecider,
    /// Why, in the words kept with the ruling. Empty where none were.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub because: String,
}

/// Who settled a row of the Log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorDecider {
    #[default]
    Operator,
    Person,
}

/// One thing a suggestion offers to do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorBoardOption {
    pub label: String,
}

/// What the Operator would do with something that is no Job's, such as a note, waiting on one press.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorBoardSuggestion {
    /// `suggestion:<n>`, the same id the row has on a Job's Needs you list. `answer_operator_suggestion` names it.
    pub id: String,
    /// What the row says, as it was kept.
    pub text: String,
    /// `Do it` (choice 0) and `Skip` (choice 1).
    pub options: Vec<OperatorBoardOption>,
    /// When it was raised, RFC 3339.
    pub at: String,
}

/// `get_operator_board`: whether the Operator is on, the arrival a call is out on, those waiting
/// for it in the order they will be taken, the suggestions no Job holds, oldest first, and its last
/// twenty rulings, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorBoard {
    pub on: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now: Option<OperatorBoardItem>,
    #[serde(default)]
    pub next: Vec<OperatorBoardItem>,
    #[serde(default)]
    pub suggestions: Vec<OperatorBoardSuggestion>,
    #[serde(default)]
    pub done: Vec<OperatorBoardSettled>,
    /// Present only while the week's plan is used to the share where the Operator pauses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused: Option<OperatorPaused>,
}

/// The week's plan used and the share where the Operator pauses, both in percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorPaused {
    pub week: u32,
    pub at: u32,
}

/// `move_on_operator_board`: `id` goes before `before` in `next`, or last where `before` is absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveOnOperatorBoard {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
}

/// `answer_operator_suggestion`: one press on a suggestion no Job holds. `choice` is 0 for Do it or 1 for Skip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerOperatorSuggestion {
    pub id: String,
    pub choice: u32,
}

/// `operator.board_changed`: the board is not what it was, so read it again. Nothing else rides.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorBoardChanged {}
