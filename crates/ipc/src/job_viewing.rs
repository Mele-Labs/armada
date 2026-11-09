//! When the person was last in a Job, told by the client that has it on screen. `view_session`, and
//! `SessionRecord::viewed_at`. `docs/practices/protocol/a-jobs-row-mark.md`.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// `view_session`: the Job came on screen in a visible window (`on`), or left it, or the window hid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewSession {
    pub session_id: SessionId,
    pub on: bool,
}
