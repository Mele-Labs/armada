//! What a Job's slot wait carries so the person can free a slot or add one instead of waiting.
//! `SessionRecord.slot_wait`, `docs/practices/protocol/a-full-pool-shows-a-wait-slot-wait.md`.
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

/// A Job waiting for a worktree slot, on its record. **Waiting is the default, so nobody is asked**:
/// the person may press Add (`answer_waiting` on `slot-choice`, choice 1) or Free (`free_slot`) from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotWait {
    pub since: Instant,
    /// How many Jobs are ahead of it in its repository's line.
    pub ahead: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holders: Vec<SlotHolder>,
    /// When the slot the person asked for was begun, while it is being made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creating_since: Option<Instant>,
    /// Why the last slot he asked for was not made, until the next Add.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_refused: Option<String>,
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
