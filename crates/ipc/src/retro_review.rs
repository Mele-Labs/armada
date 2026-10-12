//! A guided review of the open retro items, and a question put about one.
//! Both are answered by one model call and keep nothing. Since 23.74.
//! `docs/concepts/retro.md`, *Reviewing*.

use serde::{Deserialize, Serialize};

/// `review_lessons` (`POST /lessons/review`). The body may be left out.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewLessons {
    /// Narrows the review to one repository's Jobs, as `list_lessons` does.
    /// Absent is every repository served.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<String>,
}

/// What the open items come to once a model has read them together.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonReview {
    /// The model that read them. Empty open set: no call, and this is empty.
    pub model: String,
    /// **Best first.** Every open item is in `entries` or in a `merged_ids`
    /// or in `set_aside`, once.
    pub entries: Vec<ReviewEntry>,
    pub set_aside: Vec<SetAside>,
}

/// One item worth the owner's time, with the duplicates it stands for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewEntry {
    /// `Lesson::id` of the best representative.
    pub lesson_id: String,
    /// `Lesson::id` of open items that say the same thing.
    pub merged_ids: Vec<String>,
    /// One plain sentence: why it is worth the owner's time. `not ranked` on
    /// an item the model left out.
    pub reason: String,
}

/// An item the review leaves out of the queue, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetAside {
    pub lesson_id: String,
    pub why: String,
}

/// `ask_lesson` (`POST /lessons/:lesson_id/ask`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonAsk {
    pub question: String,
    /// What was said before, oldest first. Absent is none.
    #[serde(default)]
    pub history: Vec<AskTurn>,
}

/// One earlier turn of the conversation about an item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskTurn {
    pub role: AskRole,
    pub text: String,
}

/// Who said a turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskRole {
    Person,
    Fleet,
}

/// What `ask_lesson` answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskLessonAnswer {
    pub answer: String,
}
