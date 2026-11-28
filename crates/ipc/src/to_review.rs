//! `list_to_review`: everything waiting on the person across a project's Jobs, in one list he answers in place. It
//! replaces Needs you as the place he looks. `crates/ipc/operations/list_to_review.toml` is the contract.
//!
//! **Each item is a row of its Job's Needs you**, read off the same `waiting_for`, so the two never disagree. The list
//! adds what tells one Job's item from another's: the Job, its sub project, its features, and the act that answers it.

use serde::{Deserialize, Serialize};

use crate::needs_you::NeedsYouRow;
use crate::sessions::{SessionFeature, SessionId};

/// Every item waiting on him, grouped: each sub project with one, oldest started first, then each feature in its
/// library's order, then the items of Jobs in neither. No group is empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToReview {
    /// The project asked, by id. Absent where every project's Jobs are listed, and the Jobs in no project with them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub groups: Vec<ToReviewGroup>,
}

/// One heading and its items, each in the order its Job set with `order_waiting`, then oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToReviewGroup {
    pub head: ToReviewHead,
    pub items: Vec<ToReviewItem>,
}

/// What a group stands for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToReviewHead {
    /// The Jobs of one sub project: the ones he picked, and the live ones that write to one of its features.
    SubProject(ToReviewSubProject),
    /// The Jobs in no sub project whose first feature, in the library's order, is this one.
    Feature(SessionFeature),
    /// The Jobs in no sub project that write to no feature. Last.
    Neither,
}

/// A sub project as an item names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToReviewSubProject {
    /// The sub project's id, as `list_sub_projects` answers it.
    pub id: String,
    /// What he named it.
    pub name: String,
    /// His words for what it is for, which head its group. A sub project kept before goals has its name.
    pub goal: String,
}

/// One thing waiting on him.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToReviewItem {
    pub kind: ToReviewKind,
    /// The item as the Job's Needs you draws it: its id, text, since, glyph, what pressing it does and how it is put
    /// away. `get_needs_you` for the Job answers the same row.
    pub row: NeedsYouRow,
    /// Its place among its Job's open asks, 1 first, as `waiting_for` carries it.
    pub order: u32,
    /// Why it stands there, one sentence: the Job's own, else Fleet's.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub why: String,
    pub job: ToReviewJob,
    /// The project the Job is filed in, by id. Absent for a Job in none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// The sub project the Job is one of. Absent for a Job in none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_project: Option<ToReviewSubProject>,
    /// The features the Job writes to now, as `SessionRecord.features` names them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<SessionFeature>,
    /// The operation that answers it here, with no Job to open.
    pub answer: ToReviewAnswer,
}

/// The Job an item comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToReviewJob {
    pub id: SessionId,
    /// Its title, else its id.
    pub title: String,
}

/// What an item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToReviewKind {
    /// A plan awaiting his approval.
    Plan,
    /// A sketch that asks.
    Sketch,
    /// A page under `.armada/documents/` that asks.
    Document,
    /// A draft the Job proposed.
    Draft,
    /// Branches offered to try: the Job's open Ready to try cards.
    ReadyToTry,
    /// Two Jobs write the same files and have not agreed which lands first.
    Clash,
    /// A sub project's orchestrator says its goal is met, and asks him to agree.
    GoalMet,
    /// Anything else with choices or an answer, the Job's own or Fleet's.
    Question,
    /// Something the Job asked him to do, then mark done.
    ToDo,
    /// A page to walk.
    Walk,
    /// A command to run, or steps to follow.
    Command,
    /// A pull request to open.
    PullRequest,
    /// A restart the Job asked for.
    Restart,
    /// The Job said it is done.
    Done,
}

/// The operation that answers an item, tagged by `op`. Each is an existing one; the list adds none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ToReviewAnswer {
    /// Approve, or send back with notes: `answer_items` with the Job's id and the row's id. A plan, a sketch, a page, a
    /// draft, a walk, or an item of the Job's that asks for his approval.
    AnswerItems,
    /// A choice, his words, or both: `answer_waiting` with the row's id. The row's act carries the choices.
    AnswerWaiting,
    /// Each open card, answered with `answer_ready_to_try` by its branch and tip.
    AnswerReadyToTry { cards: Vec<ToReviewCard> },
    /// He did it: `done_waiting` with the row's id.
    DoneWaiting,
    /// Press Start: `begin_session` on this Job, which may be the item's own or one it started.
    BeginSession { session_id: SessionId },
    /// Put away: `dismiss_waiting` with the row's id.
    DismissWaiting,
    /// No operation answers it: the row's act is the press, such as a command to copy, a restart or a pull request to
    /// open.
    Press,
}

/// A Ready to try card, as `answer_ready_to_try` takes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToReviewCard {
    pub branch: String,
    /// In full.
    pub tip: String,
    /// What the Job said the branch holds.
    pub about: String,
}
