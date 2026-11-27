//! A note on a screen sent to the features that own the screen's files, and the Job that claims it.
//! `docs/concepts/feature-catalog.md`, *A note on a screen*.

use serde::{Deserialize, Serialize};

use crate::{FeatureJob, ManifestId, SessionId, SessionNote};

/// `send_notes_to_features`: Bridge's notes on one screen, with the files that draw it. Fleet sends them to every live
/// Job on the features those files map to, or starts a Job with `words` where none is live.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendNotesToFeatures {
    /// The repository the screen's files are in, and a new Job would start in.
    pub manifest_id: ManifestId,
    /// The project whose features are read. Absent, every project that holds the repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// The files and folders that draw the screen, from the repository root.
    pub paths: Vec<String>,
    /// His words, the request of a Job started for the notes.
    pub words: String,
    /// The notes, each with its `note_id`.
    pub notes: Vec<SessionNote>,
}

/// Where `send_notes_to_features` sent the notes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesRouted {
    /// The features the paths map to, by title, the longest prefix first.
    pub features: Vec<String>,
    /// The live Jobs the notes went to. The first to claim a note takes it; one Job alone was given them at once.
    /// Empty where a Job was started.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offered: Vec<FeatureJob>,
    /// The Job started for the notes, where no Job was live on the features. It holds them from the start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<FeatureJob>,
}

/// `claim_note`: the Job takes a note Fleet sent it and other Jobs, as its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimNote {
    /// The note's id, as the line Fleet sent with it names it.
    pub note_id: String,
    /// Which session. **Read only where the connection places no hosted session**, as `FinishUp`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// The Job a note went to: the first to claim it, or the one Fleet gave it to when none did. `claim_note`'s answer, and
/// the body of the event `note.claimed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteClaimed {
    pub note_id: String,
    pub session_id: SessionId,
    /// The Job's title.
    pub title: String,
    /// Fleet gave it the note: no Job claimed it in time, or Fleet started the Job for it because none was live.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub given: bool,
}
