//! The model's check of a feature's docs against its code, on the owner's press of Check, and his two answers to a doc
//! that disagrees: Start a Job, or Dismiss. `docs/concepts/feature-catalog.md`, *Librarian*.

use serde::{Deserialize, Serialize};

use crate::{ids::Instant, SessionId};

/// Where a doc and the feature's code disagree, in their own words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocDisagreement {
    /// What the doc says, quoted short.
    pub doc_quote: String,
    /// `path:line` in the code.
    pub code_at: String,
    /// What the code there does, quoted or said short.
    pub code_quote: String,
}

/// What the last check of a doc found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocCheck {
    pub at: Instant,
    /// Absent where the doc agrees with the code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disagrees: Option<DocDisagreement>,
    /// He dismissed `disagrees`: it stays quiet until the doc's text or the feature's code changes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dismissed: bool,
}

/// `check_docs`: put each doc of a feature to the model against the feature's code, now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckDocs {
    /// A project's id.
    pub project: String,
    pub slug: String,
}

/// One doc a press checked, and what the check found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckedDoc {
    pub target: String,
    pub check: DocCheck,
}

/// What `check_docs` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsChecked {
    pub project: String,
    pub slug: String,
    /// The docs the model answered for, each with what it found.
    pub checked: Vec<CheckedDoc>,
    /// The docs left unchecked because the night's share ran out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unchecked: Vec<String>,
    /// The model calls the press spent, failed ones too.
    pub calls: u32,
}

/// `dismiss_doc_flag` and `start_doc_job`: the doc of a feature whose disagreement he answered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocFlag {
    /// A project's id.
    pub project: String,
    pub slug: String,
    /// The doc, as the page's `docs` name it.
    pub target: String,
}

/// What `start_doc_job` started.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocJobStarted {
    pub project: String,
    pub slug: String,
    pub target: String,
    pub session_id: SessionId,
    /// `Doc or code: ` and the feature's title.
    pub title: String,
    /// The first message the Job was sent.
    pub brief: String,
    /// Why the first message was not sent, where it was not. The Job exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_failed: Option<String>,
}
