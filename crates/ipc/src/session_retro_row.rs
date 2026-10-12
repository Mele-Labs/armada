//! The `retro` row of a hosted Session's thread: a retro being written, drawn as it goes.
//! Since 23.77. `docs/concepts/retro.md`, *A Session's retro*.
//!
//! **A progress report and no record.** The kept retro is `get_session_retro`'s, and the row
//! only says how far its writing has got, so losing every frame of it changes nothing that is
//! kept. The row is replaced by id and stays in the thread when it is done.

use serde::{Deserialize, Serialize};

use crate::enums::{LandsIn, Whose};

/// Where the writing of a Session's retro has got to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetroStage {
    /// Fleet put one question to the Session's live agent and is waiting, up to 90 seconds.
    /// **Absent from a retro written where no agent is live and idle.**
    AskingAgent,
    /// Fleet is gathering what it kept of the Session.
    ReadingRecord,
    /// The model is writing. `characters` and `items` move.
    Writing,
    /// Kept. `retro` is its number.
    Written,
    /// Not kept. `why` is Fleet's words.
    Failed,
}

impl RetroStage {
    /// Whether the writing is over, one way or the other.
    pub fn over(self) -> bool {
        matches!(self, RetroStage::Written | RetroStage::Failed)
    }
}

/// One item of the retro as it settles while the model writes. **A field appears once its value
/// is complete**, in the order the model writes them, so none is ever half a sentence. The
/// items are the model's before Fleet holds them to the record, so the kept retro may have fewer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialRetroItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub who: Option<Whose>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lands_in: Option<LandsIn>,
}
