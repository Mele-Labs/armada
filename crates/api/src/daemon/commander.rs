//! What a Job's Commander may call through the door. `#2164`.
//!
//! **An allowlist, and the opposite shape to Helm's.** Helm may call any act it
//! is offered except those named in `fleet::helm::RESERVED`; a new operation is
//! therefore Helm's the day it is written. A Commander acts with nobody
//! watching, so a new operation is **refused** until a person adds it to
//! [`ALLOWED`] here, and a test fails until it is either there or on the
//! reviewed list beside it (`crate::tests::commander_reach`).
//!
//! The rule needs nothing from the daemon, unlike Helm's, which reads a machine
//! setting, so it is stated here with the door and not in `fleet`. Fleet only
//! says which connection is a Commander's (`Admitting::commander_at`).

use ipc::door::Reachable;

/// The acts a Commander may take, beside every query.
///
/// **Each steers a Job and none passes a gate**: a cap, a plan, a restart or a
/// Drone's next words. Checks and the Judge still decide whether work advances
/// (the decision record `2026-10-11-a-job-has-a-commander`).
pub const ALLOWED: &[&str] = &[
    // The caps. A raise stops at a ceiling of three times the workflow's: `#2165`.
    "raise_turn_cap",
    "raise_cost_cap",
    // A Drone's next words.
    "redirect_drone",
    "redirect_one_drone",
    // A step or a task started again.
    "restart_step",
    "restart_task",
    // The plan.
    "edit_task",
    "add_task",
    "drop_task",
    "move_plan",
    // The one way it reaches the owner: a card that outlives the turn.
    "ask_the_owner",
    // Not here until its issue lands, adding its own line:
    //   the repair Drone onto the Job's branch   `#2170`
];

/// Whether a Commander may call one operation the door offers.
///
/// **Every query, and the acts named in [`ALLOWED`], and nothing else.** A
/// query reads; the door answers it the same whoever asks.
pub fn may(offered: &Reachable) -> bool {
    offered.kind == "query" || ALLOWED.contains(&offered.operation)
}

/// What the Commander of one Job may call through the door.
///
/// **The allowlist is not a setting, so no machine can widen it and no caller
/// can choose it; the Job is the one argument, and it is the Job the process
/// was started for.** A reach with no Job cannot be made, so a Commander that
/// is not some Job's is not a value: the door narrows every act on a Job to
/// this one (`crate::reference::Resolved`), and an act naming another is
/// refused there, by the type that resolves the Job and not by a check each
/// handler remembers (`#2166`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommanderReach {
    may: Vec<&'static str>,
    job: ipc::JobId,
}

impl CommanderReach {
    pub fn of_the_allowlist(job: ipc::JobId) -> CommanderReach {
        CommanderReach {
            may: super::offerable_to_a_commander()
                .filter(|row| may(row))
                .map(|row| row.operation)
                .collect(),
            job,
        }
    }

    /// The one Job this Commander steers.
    pub fn job(&self) -> &ipc::JobId {
        &self.job
    }

    pub fn may(&self, operation: &str) -> bool {
        self.may.contains(&operation)
    }

    /// The refusal a call outside the reach is answered with, **naming the
    /// operation** so the Commander reads which act was not its own.
    pub fn refusing(&self, operation: &str) -> String {
        format!(
            "`{operation}` is not the Commander's to call: it steers a Job, and passing a gate, \
             attesting, merging, pushing and editing a Check or the Manifest are the person's"
        )
    }
}
