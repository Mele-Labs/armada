//! What a full pool's call carries so the person can free a slot instead of waiting or adding one.
//! `WaitingSource::SlotChoice`, `docs/concepts/session.md`, *A full pool asks*.
//!
//! **Fleet reads these and the person acts on them**: nothing here is sent by an agent, and no path
//! of Fleet's frees a slot by itself. A slot is freed by `free_slot`, a press.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::SessionId;

/// Why a held slot cannot be freed now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotKeeps {
    /// Its holder's turn is running, and a free would end it.
    Running,
    /// Its checkout holds files no commit has.
    Uncommitted,
}

/// The Job that holds a slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotHolderJob {
    pub id: String,
    pub title: String,
}

/// One slot a Job Fleet hosts holds, offered on a `slot_choice` call. Slots a terminal or a person
/// holds are not listed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotHolder {
    pub slot: u32,
    pub job: SlotHolderJob,
    /// When the holder last finished a turn, and its lease time where it has not.
    pub quiet_since: Instant,
    /// Why Free is off, where it is. Absent where the slot can be freed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keeps: Option<SlotKeeps>,
}

/// Where a Job goes in Waiting to start, against another Job waiting there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LinePlace {
    Ahead { session_id: SessionId },
    Behind { session_id: SessionId },
    Last,
}

/// `move_in_line`: the person drags a Job waiting to start to another place, over the Operator's
/// order. Kept in the Operator's Log as a correction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveInLine {
    pub session_id: SessionId,
    pub place: LinePlace,
}

/// `free_slot`: the person gives back the slot a waiting Job's call lists, to that Job. `session_id`
/// is the waiting Job and `item_id` its `slot_choice` item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeSlot {
    pub session_id: SessionId,
    pub item_id: String,
    pub slot: u32,
}
