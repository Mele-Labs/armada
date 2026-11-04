//! `show_file`: a session puts a file in front of the person, to read or edit in Bridge's side panel.
//! `docs/concepts/session.md`, *Showing a file*.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// `show_file`: a session opens a file for the person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowFile {
    /// Absolute, or relative to the folder the session runs in. Fleet resolves it and keeps the
    /// absolute path.
    pub path: String,
    /// Which session shows it. **Read only where the connection places no hosted session**, as
    /// `ShowWindow`'s is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `get_shown_file`: a file a session showed, read as text for a reader that is not Bridge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShownText {
    /// The absolute path the session's `show_file` kept, as its attachment's `target` holds it.
    pub path: String,
    pub text: String,
}

/// `get_shown_file` for a video: the file is not sent. The Gateway streams it from the Mac's disk, so what
/// it needs is where it is, what it is, and how long.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShownVideo {
    /// The absolute path the session's `show_file` kept, as its attachment's `target` holds it.
    pub path: String,
    /// `video/mp4`, `video/quicktime` or `video/webm`.
    pub media_type: String,
    /// The file's size in bytes when it was read.
    pub bytes: u64,
}
