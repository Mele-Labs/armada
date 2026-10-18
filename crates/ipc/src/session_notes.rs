//! A note the person pinned on a page, sent to a Session whole: his words, where he pointed, what
//! the page was showing and a crop of the spot. `send_session_note`.
//!
//! **Fleet answers with the lines of code the spot was drawn from**, found by the words on screen,
//! so the Session starts at a file and a line rather than guessing one from a selector.

use serde::{Deserialize, Serialize};

use crate::sessions::{SessionId, SessionRecord};

/// `send_session_note`. It takes a turn, as `send_session_message` does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendSessionNote {
    pub session_id: SessionId,
    /// The person's words, trimmed. **The only part the thread keeps.**
    pub said: String,
    /// The page's url as loaded.
    pub address: String,
    /// The `.armada/annotations/<id>.json` note, where Bridge's own layer took it, so
    /// `mark_annotation_done` still finds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_id: Option<String>,
    pub place: NotePlace,
    pub state: NoteState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<NoteCrop>,
}

/// Where the person pointed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotePlace {
    /// The component that drew the element, where the page names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    /// The components around it, innermost first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<String>,
    pub selector: String,
    pub element: NoteElement,
    /// At most 8 accessible names of what encloses it, and the nearest heading above, innermost
    /// first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nearby: Vec<String>,
    /// `path:line` from the closest `[data-armada-source]`. **A claim, checked against the text.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stamp: Option<String>,
    /// Path, query and hash.
    pub location: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

/// The element itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteElement {
    pub tag: String,
    /// Its text, cut to 200 characters.
    pub text: String,
    /// Its accessible name, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// What the page was showing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteState {
    /// At most 12, each `role "name"`: `tab "Bash"`, `option "Session 3"`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk: Option<NoteWalk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take: Option<NoteTake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
}

/// The walk the page was on, and its step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteWalk {
    pub name: String,
    pub step: u32,
    pub of: u32,
}

/// The take of a walk with variations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteTake {
    pub letter: String,
    pub name: String,
}

/// A picture of the spot. `data` is base64.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteCrop {
    pub media_type: String,
    pub data: String,
}

/// `send_session_note`'s answer: the turn taken, and the lines the Session was handed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoteSent {
    pub session: SessionRecord,
    /// At most 5, best first.
    pub candidates: Vec<Candidate>,
}

/// A line of code the spot may have been drawn from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// Relative to the checkout searched.
    pub file: String,
    /// 1-based.
    pub line: u32,
    /// Why it matched, in one short line.
    pub why: String,
    /// The line, trimmed, at most 160 characters.
    pub text: String,
}
