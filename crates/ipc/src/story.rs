//! What the owner said in a Job, sorted by kind, the asks that came of it, and a Job's story.
//! `docs/concepts/job-story.md` has the rules, `crates/ipc/operations/list_message_kinds.toml` and its neighbours the routes.
//!
//! **A message of his is one of four kinds**, sorted by a small model after it is delivered and changed by one press: a
//! `new_ask`, a `revision` of an ask, an `answer` to one, or `talk`. An ask keeps his first words and every revision.
//! Feature slugs are plain strings: the library that names them is another surface's.

use serde::{Deserialize, Serialize};

use crate::{ArtifactForm, ArtifactState, Instant, PagePress, SessionId, VoteCast};

/// What a message of his was sorted as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    /// Opens an ask.
    NewAsk,
    /// Changes an ask the Job already holds, which keeps what he first said.
    Revision,
    /// A pick, an approval or a reply, joined to the ask it answers.
    Answer,
    /// Stays in the thread and opens nothing.
    Talk,
}

/// What an ask is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskKind {
    Bug,
    ChangeRequest,
    Idea,
    Fix,
}

/// Where an ask stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskState {
    /// Decided and not started.
    Backlog,
    /// A Job is building it.
    Building,
    /// The Job that took it merged.
    Built,
    /// Passed to another Job, which builds it.
    Handed,
    Dropped,
}

/// Who sorted a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortedBy {
    Fleet,
    /// He changed it. **Kept**, and the newest few ride in the sorting prompt as examples.
    Owner,
}

/// How one message of his was sorted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageSorting {
    pub session_id: SessionId,
    /// The message's row id in the Job's thread.
    pub row_id: String,
    pub kind: MessageKind,
    /// The kind of the ask a `new_ask` opened. Absent for every other kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_kind: Option<AskKind>,
    /// The ask a `new_ask` opened, or a `revision` or an `answer` joined. Absent for `talk`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_id: Option<String>,
    /// The title of that ask.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub sorted_by: SortedBy,
    pub sorted_at: Instant,
}

/// What `list_message_kinds` answers: the messages of a Job that were sorted, oldest first. **A message missing here is not
/// sorted yet**, or the model failed on it, and the thread draws no chip for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageSortings {
    pub session_id: SessionId,
    pub messages: Vec<MessageSorting>,
}

/// `set_message_kind`: he changes a message's kind, and the ask it joins.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetMessageKind {
    pub session_id: SessionId,
    pub row_id: String,
    pub kind: MessageKind,
    /// For `new_ask`: the ask's kind. Absent keeps the one it has, else `change_request`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_kind: Option<AskKind>,
    /// For `revision` and `answer`: the ask it joins, which the Job holds. Required for both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_id: Option<String>,
}

/// One message that joined an ask after it opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskWords {
    pub session_id: SessionId,
    pub row_id: String,
    /// `revision` or `answer`.
    pub kind: MessageKind,
    /// His words as he last wrote them.
    pub words: String,
    pub at: Instant,
}

/// One ask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ask {
    pub ask_id: String,
    pub kind: AskKind,
    /// A short line a small model wrote, which he may change.
    pub title: String,
    /// His first words, as he last wrote them.
    pub first_words: String,
    /// The message row they are in, in `said_in`'s thread. For an ask filed outside a Job: `note:<id>` for a note, else the ask's id.
    pub first_row: String,
    /// The Job it was said in. **Empty for an ask filed outside a Job**, which opens in the backlog.
    pub said_in: SessionId,
    /// The Job building it. Absent in the backlog and once dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building: Option<SessionId>,
    /// The feature slugs it is filed under. An ask on two shows on both.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    pub state: AskState,
    /// When `state` last moved.
    pub state_at: Instant,
    pub opened_at: Instant,
    /// Each revision and answer, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub said: Vec<AskWords>,
}

/// What `list_job_asks` answers: the asks said in a Job, built by it, or handed through it, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobAsks {
    pub session_id: SessionId,
    pub asks: Vec<Ask>,
}

/// `change_ask`: he moves an ask. Each field present is set; absent is left.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeAsk {
    pub ask_id: String,
    /// `backlog` or `dropped` only: Fleet moves the others as it sees the work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<AskState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<AskKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The whole list of slugs, replacing the one the ask has. An empty list files it under none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
}

/// `file_ask`: the Operator files an ask said outside a Job: his words to it, a note on a screen, or dictation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileAsk {
    /// The project's id the ask is filed in.
    pub project: String,
    pub kind: AskKind,
    /// His words, as he said them.
    pub words: String,
    /// A short line. Absent takes his first line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The slugs of the features it is filed under, each held by the project's library.
    #[serde(default)]
    pub features: Vec<String>,
    /// The id of the note the words are, where they are one. A note filed twice is one ask.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// What `list_feature_asks` answers: a project's asks, narrowed to one feature slug where one is given, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureAsks {
    pub project: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    pub asks: Vec<Ask>,
}

/// What he was shown and what he decided about it: one artifact of the Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryShown {
    /// The artifact's id, as `JobArtifact::id`.
    pub artifact: String,
    pub title: String,
    pub form: ArtifactForm,
    /// Where its newest revision stands: `approved`, `sent_back`, `dismissed`, `asking`, `reading` or `withdrawn`.
    pub state: ArtifactState,
    /// His note on a send back, or his words with an approval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub votes: Vec<VoteCast>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presses: Vec<PagePress>,
    /// When its first revision was kept: what puts it under the ask he last spoke on.
    pub shown_at: Instant,
}

/// A pull request that built an ask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryBuilt {
    /// The pull request's number as the ledger holds it.
    pub pull_request: String,
    pub merged_at: Instant,
}

/// An ask passed to another Job, or taken from one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryHanded {
    pub from: SessionId,
    pub to: SessionId,
    /// The receiving Job's title, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_title: Option<String>,
    pub at: Instant,
}

/// One ask in a Job's story.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryAsk {
    /// How it started, its revisions and where it stands.
    pub ask: Ask,
    /// What he was shown while it was the one he last spoke on, and decided. Oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shown: Vec<StoryShown>,
    /// What was built: the merges, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub built: Vec<StoryBuilt>,
    /// What was handed to other Jobs and what came from them, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handed: Vec<StoryHanded>,
    /// Still open: not built, not dropped.
    pub open: bool,
}

/// What `get_job_story` answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobStory {
    pub session_id: SessionId,
    /// Open asks first, each group oldest first.
    pub asks: Vec<StoryAsk>,
    /// What he was shown before he spoke on any ask of the Job, or after the last ask was dropped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shown_otherwise: Vec<StoryShown>,
    /// One line an ask, open ones first, for the done card to lead with beside the Job's own summary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lead: Vec<String>,
}

/// `session.story`: a Job's sorting, asks or story moved. A client reads `list_message_kinds`, `list_job_asks` and
/// `get_job_story` again for the Job it shows. An ask handed on moves both Jobs, so both are published.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryChanged {
    pub session_id: SessionId,
}
