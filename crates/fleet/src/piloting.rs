//! The three ways out of a taken-over Job, and what is read of a pilot that stands.
//! `docs/concepts/pilot.md`, *Evidence*.
//!
//! **Nothing takes a Job over any more.** The act, the Session that was started on the Job's
//! worktree and the handoff between them are gone; a Job that is `piloted` already is ended
//! through [`exits`], and the build sweep is handed [`Fleet::piloted_checkouts`] so `armada
//! clean` skips a piloted Job and names it.

mod exits;
mod served;

use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobStatus, StepId};

use crate::daemon::Fleet;

/// Why a pilot cannot be ended. **Each is checked before anything moves.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unpilotable {
    /// A run of this Job's Checks, or another exit, is out.
    Busy,
    /// An exit was asked of a Job that is not piloted.
    NotPiloted {
        status: JobStatus,
    },
    /// No step is stopped, held at a gate or being worked, so there are no
    /// gates to run.
    NothingToVerify,
    /// The step's evidence is written by its Drone, and none was written.
    NothingToSubmit {
        step: StepId,
    },
    /// Attesting is a verdict on finished work, not a way past a step.
    StepsNotAdvanced {
        step: StepId,
    },
}

impl Unpilotable {
    /// The refusal code. **All 409**: the Job is as it was.
    pub fn code(&self) -> &'static str {
        match self {
            Unpilotable::Busy => "fleet.pilot_busy",
            Unpilotable::NotPiloted { .. } => "fleet.not_piloted",
            Unpilotable::NothingToVerify => "fleet.nothing_to_verify",
            Unpilotable::NothingToSubmit { .. } => "fleet.nothing_to_submit",
            Unpilotable::StepsNotAdvanced { .. } => "fleet.steps_not_advanced",
        }
    }

    pub fn said(&self) -> String {
        match self {
            Unpilotable::Busy => String::from(
                "this Job's pilot is being ended, or its Checks are running. Wait for that to finish",
            ),
            Unpilotable::NotPiloted { status } => format!(
                "this Job is {}, and only a piloted Job has a pilot to end",
                status.as_wire()
            ),
            Unpilotable::NothingToVerify => String::from(
                "no step of this Job is stopped, at a gate or being worked, so there are no gates \
                 to run. Attest it complete or close it as superseded",
            ),
            Unpilotable::NothingToSubmit { step } => format!(
                "`{}` asks for written evidence that its Drone writes, and none was written. \
                 Restart the step to have it written, or attest the Job complete",
                step.as_str()
            ),
            Unpilotable::StepsNotAdvanced { step } => format!(
                "`{}` has not advanced. Attesting is a verdict on the work and not a way past a \
                 step: submit it for verification, or close the Job as superseded",
                step.as_str()
            ),
        }
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Every piloted Job's checkout, for the sweeps that walk checkouts without
    /// asking a Job.
    pub async fn piloted_checkouts(&self) -> Vec<PathBuf> {
        let Ok((loaded, _)) = self.every_job().await else {
            return Vec::new();
        };
        loaded
            .jobs
            .iter()
            .filter(|job| job.status() == JobStatus::Piloted)
            .filter_map(|job| self.worktree_of(job).ok().flatten())
            .map(|worktree| PathBuf::from(worktree.path()))
            .collect()
    }

    /// The Job's pilot as its row carries it, once the pilot began.
    pub(crate) fn piloted_on_row(&self, store: &store::Store, job: &Job) -> Option<ipc::Piloted> {
        let kept = store.pilot_of(job.id()).ok().flatten()?;
        let since = kept.piloted_at.as_ref()?;
        Some(ipc::Piloted {
            reason: kept.reason.as_wire().to_string(),
            since: ipc::Instant::from(since),
            exit: kept.exit.map(|exit| exit.as_wire().to_string()),
            ended_at: kept.ended_at.as_ref().map(ipc::Instant::from),
            note: kept.note.clone(),
        })
    }
}
