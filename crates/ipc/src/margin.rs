//! Margin threads: his comments on a passage of a Job's page, and the agent's answers and suggested edits on each.
//! An answer changes nothing on the page, and a suggested edit changes it only on his Accept.
//! `crates/ipc/operations/comment_margin.toml` and its neighbours are the contract.

use serde::{Deserialize, Serialize};

use crate::{Instant, SessionId};

/// Who wrote a message in a margin thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginVoice {
    You,
    Agent,
}

/// Where a margin thread stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginState {
    /// Sent to the agent, and not answered yet.
    Open,
    /// The agent answered, and the page is as it was.
    Answered,
    /// The agent suggested an edit, which waits on his Accept or Reject.
    Suggested,
    /// His comment is held, to go with the others he holds on Send held.
    Held,
    /// He settled it. Nothing more is sent on it.
    Settled,
}

/// The passage a thread is on, as he selected it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarginAnchor {
    /// The words he selected, as the page read then. **Kept when the passage is gone.**
    pub quote: String,
    /// The `data-block` of the element the words are in, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<String>,
    /// The `h2` the passage is under in the newest revision that holds it. Absent above the first heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

/// One message in a margin thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarginMessage {
    pub by: MarginVoice,
    pub text: String,
    pub at: Instant,
    /// His message held, not sent to the agent yet.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub held: bool,
}

/// How he decided a suggested edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionHow {
    Accepted,
    Rejected,
}

/// His decision on a suggested edit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestionDecided {
    pub how: SuggestionHow,
    pub at: Instant,
    /// The revision his Accept kept. Absent on a Reject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u32>,
}

/// An edit the agent suggested on a thread's passage: the words it would strike and the words it would add.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarginSuggestion {
    pub struck: String,
    pub added: String,
    pub at: Instant,
    /// Absent until he accepts or rejects it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided: Option<SuggestionDecided>,
}

/// One margin thread on a passage of a Job's page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarginThread {
    /// `m1`, `m2` and on, one count a Job.
    pub id: String,
    /// The page's file name under `.armada/documents/`.
    pub document: String,
    pub anchor: MarginAnchor,
    /// Oldest first.
    pub messages: Vec<MarginMessage>,
    pub state: MarginState,
    /// **The newest revision no longer holds the quote.** The thread keeps it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gone: bool,
    /// The newest edit the agent suggested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<MarginSuggestion>,
    /// What he settled it with, where he said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
}

/// What a comment is on: a passage, which starts a thread, or a thread he replies on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MarginOn {
    Passage { anchor: MarginAnchor },
    Thread { thread: String },
}

/// `comment_margin`: his comment on a passage of a page, or his reply on a thread, sent now or held.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentMargin {
    pub session_id: SessionId,
    /// The page's file name, or its path.
    pub document: String,
    pub on: MarginOn,
    pub text: String,
    /// Held to go with his other held comments on Send held.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hold: bool,
}

/// `send_held_margin`: every comment he holds on the page sent to the agent as one message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendHeldMargin {
    pub session_id: SessionId,
    pub document: String,
}

/// What he does on a thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MarginAct {
    /// Make this change: the agent is asked to turn its answer into a suggested edit.
    MakeChange,
    /// The suggested edit is written as the page's next revision.
    Accept,
    /// The suggested edit is dropped, and the page is as it was.
    Reject,
    /// The thread is closed, and the agent is asked to add the outcome to the page's Decisions.
    Settle {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcome: Option<String>,
    },
}

/// `act_on_margin_thread`: his Make this change, Accept, Reject or Settle on one thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActOnMargin {
    pub session_id: SessionId,
    pub thread: String,
    pub act: MarginAct,
}

/// `get_margin_threads`: a page's threads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetMarginThreads {
    pub session_id: SessionId,
    pub document: String,
}

/// A page's threads, oldest first: what each of his margin requests answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarginThreads {
    pub session_id: SessionId,
    pub document: String,
    pub threads: Vec<MarginThread>,
}

/// `answer_thread`: the agent's answer in a margin thread. It changes nothing on the page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerThread {
    /// Read only for a Job in a terminal; a hosted Job is the one whose process holds the connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub thread: String,
    pub text: String,
}

/// `suggest_edit`: the agent's edit on a thread's passage, which waits on his Accept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestEdit {
    /// As [`AnswerThread::session_id`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub thread: String,
    /// The words on the page to strike, exactly as they read.
    pub struck: String,
    /// The words that take their place.
    pub added: String,
}

/// What a round of margin talk was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginRoundKind {
    /// His comments sent, now or held.
    Comments,
    /// His Make this change.
    Change,
    Accepted,
    Rejected,
    Settled,
    /// A page sent back with his notes.
    SentBack,
}

/// One note of a round: the passage and his whole note on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadMarginNote {
    /// The margin thread it is on. Absent on a note sent back with the page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Absent on a note on the whole page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passage: Option<String>,
    pub note: String,
}

/// One round of margin talk in a Job's thread, in the place of the message it sent: built from the record,
/// never from the message's words. Since 21 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadMargin {
    pub id: String,
    pub at: Instant,
    /// The page's file name.
    pub page: String,
    /// The page's title.
    pub title: String,
    pub kind: MarginRoundKind,
    /// One line.
    pub summary: String,
    /// An answer or a suggested edit in the round waits on him.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub waits_on_you: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<ThreadMarginNote>,
}
