//! A Job's document put to the person, and his answer: `answer_document`.
//! `crates/ipc/operations/answer_document.toml` is the contract.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// The keys of a document's `detail` that put a revision to the person and say how he answered.
/// `detail` holds strings only: `asked` is `"true"`, `settled` a [`DocumentHow::word`].
pub const DOCUMENT_ASKED: &str = "asked";
/// The newest revision's number, `"1"` and on: how many `revisions` there are. `answer_document`
/// names it back.
pub const DOCUMENT_REVISION: &str = "revision";
/// When the newest revision was kept, as `settled_at` is written. `changed_at` moves on an answer, so
/// a card placed by it would jump.
pub const DOCUMENT_KEPT_AT: &str = "kept_at";
pub const DOCUMENT_SETTLED: &str = "settled";
pub const DOCUMENT_SETTLED_AT: &str = "settled_at";
/// His note on `sent_back`, or his words with an approval. Absent where he gave none.
pub const DOCUMENT_WORDS: &str = "words";
/// What a document's Needs you item id is its page's path behind.
pub const DOCUMENT_ITEM_PREFIX: &str = "document:";

/// How the person answered a document put to him.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentHow {
    Approved,
    /// Sent back with a note, to be revised and written again at the same path.
    SentBack,
}

impl DocumentHow {
    /// What `detail.settled` holds.
    pub fn word(self) -> &'static str {
        match self {
            DocumentHow::Approved => "approved",
            DocumentHow::SentBack => "sent_back",
        }
    }
}

/// `answer_document`: the person approves the revision of a document he read, or sends it back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerDocument {
    pub session_id: SessionId,
    /// The Needs you item id, `document:<page path>`.
    pub document: String,
    /// `detail.revision` as he read it. **A revision kept since is refused**, so an answer never
    /// lands on a page he did not see.
    pub revision: String,
    pub how: DocumentHow,
    /// The note, which `sent_back` needs. Words with an approval are passed on too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
}
