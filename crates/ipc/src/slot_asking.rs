//! `armada worktree lease` run by a hosted Session tells Fleet every slot is held, so Fleet asks the
//! person whether the lease waits or a slot is added. The call is the Session's `slot_choice`
//! waiting item, the one a queued write raises: one call, never two.
//!
//! **A record that expires.** The command asks again about once a second and Fleet forgets one it
//! has not heard from for ten seconds, so a command that dies takes its call with it.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// Which end of the wait is being told.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskForSlotAct {
    /// Every slot is held. Raises the call if none stands, and is the command's heartbeat.
    Ask,
    /// The command leased a slot or gave up. Takes the call off and ends a reservation.
    Done,
}

/// `ask_for_slot`: `POST /sessions/slot_wanted`. `armada worktree lease` is the caller, naming the
/// Session by the `ARMADA_SESSION` Fleet set in its process.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskForSlot {
    pub act: AskForSlotAct,
    pub session_id: SessionId,
}

/// Where the person's answer stands, as far as the command acts on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotAskStanding {
    /// The call is on the person's list and unanswered.
    Asking,
    /// He chose to wait: the command keeps looking for a free slot.
    Waiting,
    /// He chose to add one: Fleet made a slot and holds it for this command for about thirty seconds.
    Added,
    /// Fleet is not asking: it does not host the Session, or the Session has ended. The command
    /// behaves as it did before this existed.
    Unasked,
}

/// What `ask_for_slot` answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotAsked {
    pub standing: SlotAskStanding,
}
