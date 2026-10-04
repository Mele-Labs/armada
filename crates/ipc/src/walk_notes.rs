//! What a person said while walking a Prototype's mock, kept on the Job.
//!
//! **On the Job, not on the worktree.** A Prototype stops at Build with its mock
//! served and a person walks it in a Bridge window, pointing at what is wrong.
//! The worktree is throwaway; the notes are what the next Drone is told, so Fleet
//! keeps them beside the Job's record and keeps each frame under the machine
//! directory. Since 23.16.
//!
//! **Fixed at capture.** A note is never edited. It is removed while unsent, and
//! `sent` flips once, when a `request_changes` carrying `with_walk_notes` hands
//! it to a Drone.

use serde::{Deserialize, Serialize};

use crate::capturing::{CaptureServed, StagedFrame, StudioCapture};
use crate::ids::Instant;

/// `capture_walk_note`'s body. `CaptureStudioNote` without a position or a
/// node it was produced by, neither of which a Job has. **Since 23.16.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureWalkNote {
    /// What the person said, verbatim. Blank is refused at the Fleet boundary.
    pub said: String,
    /// Where they pointed. Its `frame` is ignored: a captured frame arrives
    /// staged, below.
    pub capture: StudioCapture,
    /// The PNG Bridge took, staged on disk. Absent where nothing could take one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<StagedFrame>,
}

/// One note, as Bridge draws it and as `get_job` carries it. **Since 23.16.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkNote {
    /// Minted by Fleet at capture.
    pub id: String,
    /// What the person said, verbatim.
    pub said: String,
    /// When Fleet kept it.
    pub at: Instant,
    /// One line naming what was pointed at, as a person would read it —
    /// `button “Save”`. Composed by Fleet from the capture's element.
    pub element: String,
    pub selector: String,
    /// The path within the page, the capture's own `location`.
    pub location: String,
    /// The server it was captured on, where the capture named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub served: Option<CaptureServed>,
    /// The kept PNG's absolute path, under the machine directory and never the
    /// worktree. Absent where no frame was taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    /// True once a `request_changes` carried it to a Drone.
    #[serde(default)]
    pub sent: bool,
}

/// Every note on one Job, oldest first: what `capture_walk_note` and
/// `remove_walk_note` answer with. **Since 23.16.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkNotes {
    pub notes: Vec<WalkNote>,
}

/// `remove_walk_note`'s body. A sent note is refused. **Since 23.16.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveWalkNote {
    pub id: String,
}
