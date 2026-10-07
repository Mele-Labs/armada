//! Taking a Job over, and what a person is handed when they do.
//! `docs/concepts/pilot.md`. Since 23.50.
//!
//! **Assist is not a variant.** It is deferred, so a request naming it does not
//! decode, and nothing downstream has a branch for an outcome that cannot run.

use serde::{Deserialize, Serialize};

use crate::{ChangedFile, DeclaredPlan, Instant, JobDetail, JobEvidence, JobHistory, StepId};

/// What a person asked of the Job they are taking over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotOutcome {
    /// The Drone ends and the worktree is the person's.
    TakeOver,
    /// The Drone ends, and a fresh one takes the step when the person is done.
    RestartStep,
}

/// The request half of `take_over`. **The whole body is optional**, and an empty
/// one is a take over with no Session named.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakeOver {
    pub outcome: PilotOutcome,
    /// The Session the person pilots from. Named so the Board can say who is in
    /// the worktree; **Fleet does not start it**, which is the session host's
    /// act and takes the [`HandoffBundle`] and the worktree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// What a person says as they end a pilot by attesting or superseding. Never
/// required: the act is the record, and the words are for whoever reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PilotNote {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A Job's pilot on its row, **beside the status and never instead of it**.
///
/// Present on a `piloted` Job, and still present once the pilot ended, because
/// `exit` is what tells a Job a person attested from one that passed its gates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piloted {
    /// `take_over` or `restart_step`. A string, for [`crate::Paused::by`]'s
    /// reason: Bridge draws a chip and nothing matches on it.
    pub reason: String,
    /// The Session in the worktree, where one was named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub since: Instant,
    /// How it ended: `submitted`, `attested` or `superseded`. **Absent while
    /// the person is still working.** `attested` is never verified, and a
    /// surface draws it as what it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// The person's words on an attestation or a supersede.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// What the Drone said it was stuck on. **Not Evidence**: this states that no
/// proof is coming.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneNarrative {
    pub trying_to: String,
    pub blocked_by: String,
    pub tried: String,
}

/// The step a Job stopped on and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoppedOn {
    pub step_id: StepId,
    /// The trigger the step stopped under, where it stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
}

/// Where the person works.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffWorktree {
    /// The checkout, as a path on this machine.
    pub path: String,
    pub branch: String,
}

/// Everything a person, or a Session on their behalf, needs to pick a Job up.
///
/// **Fleet assembles it and the Drone supplies one field**, `narrative`, which
/// is by this system's own principle the least trustworthy thing in it. The
/// rest is read off the record, so nothing here is something a Drone could have
/// shaped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffBundle {
    /// The Job whole: its record, its steps, their Checks and the Judge's
    /// refusals, and the verdict each step carries.
    pub job: JobDetail,
    /// Every move the Job made, so every attempt of every step is in it.
    pub history: JobHistory,
    /// The evidence each step's Drone submitted.
    pub evidence: JobEvidence,
    /// `take_over` or `restart_step`.
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Absent where the Job holds no checkout, which a person is told.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<HandoffWorktree>,
    /// Absent where no step stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_on: Option<StoppedOn>,
    /// What each run of each step said its work would be.
    pub plans: Vec<DeclaredPlan>,
    /// Whether the step in hand declared a plan, for `changed`'s `outside_plan`
    /// to mean anything. **False is no plan, not nothing drifted.**
    pub plan_declared: bool,
    /// The worktree as it stands, each file marked where it is outside the plan.
    pub changed: Vec<ChangedFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrative: Option<DroneNarrative>,
}
