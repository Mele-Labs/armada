//! A Job's shell: the one terminal the person and the Job's agent both type into. Typing into it,
//! resizing it, watching it and stopping it are the run operations, on its `run_id`.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;

/// `open_terminal`: the person opens the Job's shell in Bridge. `POST /sessions/terminal/open`.
/// Answered 200 with `SessionRunUnderway`: the shell already open, or one opened now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenTerminal {
    pub session_id: SessionId,
    /// The terminal's size in cells where it opens now. Absent is 80 by 24.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
}
