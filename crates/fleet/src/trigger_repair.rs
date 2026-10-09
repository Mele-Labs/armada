//! What a failed Trigger's repair decides, apart from the doing of it.
//! `docs/concepts/fleet.md`, *A failed Trigger's repair*.
//!
//! **Pure, so the acceptance test and Fleet read the same rules.** Fleet's
//! effects (a slot, a Drone, a push) are in `crate::repairing`; this is what
//! each of them is asked next, the brief the Drone is told, and the words of
//! the alert.

use std::collections::VecDeque;

use core_model::{FixChoice, JobId, RepairRecord, StepId, TriggerState, REPAIR_TRIES};

/// What follows a rerun of the Trigger's command on the repair branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterRerun {
    /// It passes. The fix waits for the owner, and Fleet does not choose.
    HoldTheFix,
    /// It still fails and a try is left, so another Drone goes on the same
    /// branch.
    TryAgain,
    /// It still fails and none is left.
    GiveUp,
}

impl AfterRerun {
    /// `tries` is how many repair Drones have been put on it, this one included.
    pub fn of(tries: u32, passed: bool) -> AfterRerun {
        match (passed, tries < REPAIR_TRIES) {
            (true, _) => AfterRerun::HoldTheFix,
            (false, true) => AfterRerun::TryAgain,
            (false, false) => AfterRerun::GiveUp,
        }
    }

    /// The state the firing is in once this is decided.
    pub fn state(self) -> TriggerState {
        match self {
            AfterRerun::HoldTheFix => TriggerState::FixReady,
            AfterRerun::TryAgain => TriggerState::Repairing,
            AfterRerun::GiveUp => TriggerState::Failed,
        }
    }
}

/// What the owner's choice sets going.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Merge the repair branch onto the Job's and push it, then run the
    /// Command on the Job's branch, which is what settles the Trigger.
    OntoTheJobsBranch,
    /// Push the repair branch and open a pull request from it against the
    /// Job's target. The Command has already passed on it.
    AsAPullRequest,
}

impl Delivery {
    pub fn of(choice: FixChoice) -> Delivery {
        match choice {
            FixChoice::ThisBranch => Delivery::OntoTheJobsBranch,
            FixChoice::NewPr => Delivery::AsAPullRequest,
        }
    }

    /// Where the firing stands once the delivery is made and nothing more has
    /// run. A new pull request is the end of it; the Job's own branch is run
    /// again first.
    pub fn state_once_made(self) -> TriggerState {
        match self {
            Delivery::OntoTheJobsBranch => TriggerState::Rerunning,
            Delivery::AsAPullRequest => TriggerState::Passed,
        }
    }
}

/// Whether the branch the repair Drone wrote on has done its work: its fix is
/// merged onto the Job's branch, or the repair did not fix it. **A new pull
/// request's branch is that pull request's head**, so it is never done with
/// here, and a fix still waiting on a choice is not placed.
pub fn branch_is_done_with(state: TriggerState, choice: Option<FixChoice>) -> bool {
    match state {
        TriggerState::Failed | TriggerState::Held => choice != Some(FixChoice::NewPr),
        // Passed with no choice is a side run that changed nothing.
        TriggerState::Passed => choice != Some(FixChoice::NewPr),
        _ => false,
    }
}

/// The repair Drone's one turn. The first line is what a Drone harness and a
/// fake both key on, as the worktree repair's is.
pub fn brief(
    trigger: &str,
    command: &str,
    exit: Option<i32>,
    stdout: &str,
    stderr: &str,
) -> String {
    let exited = match exit {
        Some(code) => format!("exited with code {code}"),
        None => String::from("did not exit on its own"),
    };
    let printed = match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => {
            String::from("It printed nothing, or its output was not kept: run it to see.")
        }
        _ => format!("stdout:\n{}\n\nstderr:\n{}", tail(stdout), tail(stderr)),
    };
    format!(
        "REPAIR THE TRIGGER `{trigger}`\n\n\
         The Trigger `{trigger}` runs `{command}`, and it failed: it {exited}.\n\n\
         {printed}\n\n\
         Make the command pass. Change what this branch needs changed, run `{command}` yourself \
         to check, and stop when it passes. Do not commit, push or open a pull request: Fleet \
         does that."
    )
}

/// The last of what a Command printed, which is where it says why.
fn tail(output: &str) -> String {
    const KEEP: usize = 4_000;
    match output.char_indices().rev().nth(KEEP - 1) {
        Some((at, _)) if at > 0 => format!("...{}", &output[at..]),
        _ => output.to_string(),
    }
}

/// Why a failed Trigger is on the Job's alerts.
pub fn alert(trigger: &str) -> String {
    format!("`{trigger}` failed and {REPAIR_TRIES} repairs did not fix it")
}

/// Why a held fix is on the Job's alerts: it waits on the owner's choice.
pub fn fix_waiting(trigger: &str) -> String {
    format!("`{trigger}` has a fix waiting on your choice")
}

/// What is being repaired: a firing of a Trigger, or a step added to one Job.
/// **Both go through the same repair**, so everything that asks "which" asks
/// this.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Subject {
    Firing(i64),
    /// The addition's id, `a1` and so on.
    Addition(String),
}

impl Subject {
    /// What a log line calls it.
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Subject::Firing(_) => "Trigger",
            Subject::Addition(_) => "Added step",
        }
    }
}

/// The part of a repair's names that tells one subject from another: the
/// firing's number, or the addition's id.
impl std::fmt::Display for Subject {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Subject::Firing(id) => write!(out, "{id}"),
            Subject::Addition(id) => write!(out, "{id}"),
        }
    }
}

/// A failed firing waiting for a repair Drone. Held in memory: a Fleet that
/// restarts forgets, and the firing stays `repairing` with nothing working it.
#[derive(Debug, Clone)]
pub(crate) struct Waiting {
    pub job: JobId,
    pub subject: Subject,
    /// The Trigger's name, or the added step's command.
    pub trigger: String,
    pub step: StepId,
    pub command: String,
    pub exit: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// What the repair had come to, for one a restart took up again.
    pub record: RepairRecord,
    /// The whole brief of a Skill or Drone run, where this is one and not a
    /// repair: `command` and the output are then empty, there is no Command
    /// to run again, and no second try.
    pub side: Option<String>,
}

#[derive(Default)]
pub(crate) struct Queue(VecDeque<Waiting>);

impl Queue {
    pub(crate) fn push(&mut self, one: Waiting) {
        self.0.push_back(one);
    }

    pub(crate) fn pop(&mut self) -> Option<Waiting> {
        self.0.pop_front()
    }
}
