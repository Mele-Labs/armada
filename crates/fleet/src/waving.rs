//! An Epic's wave: the Jobs its plan proposed, released by one press, and the
//! members a parent that finishes on their merges is still waiting for. Spike
//! 022, slice 6 (#1694, #1699, #1692).
//!
//! **The wave is real Jobs at `awaiting_approval`**, each dispatched by the
//! parent and stamped with the pass that made it, so a person reads, corrects
//! (`edit_job`) or drops (`kill_job`) each before anything runs. **One act
//! releases them**: `approve_wave`, at the parent's plan gate, refused unless
//! it names exactly the wave Fleet holds
//! (`.claude/decisions/2026-09-30-approving-an-epics-plan-releases-its-wave.md`).
//!
//! The decisions are free functions over the board, so the milestone's
//! acceptance test asks them the questions Fleet does; the methods below move
//! the record on their answers.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Landing, Vcs, WorkProduct};
use core_model::{Actor, CompleteWhen, Job, JobId, JobStatus, Origin, StepId, StepTarget, Target};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Why a press did not release a wave. **Nothing moved**: the parent stays at
/// its gate and every member where it was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotTheWave {
    /// The parent holds no wave: nothing it proposed is still waiting.
    NoneHeld,
    /// The press named a different set from the one held.
    Differs {
        /// Named, and not a Job the parent holds at its gate.
        not_held: Vec<String>,
        /// Held, and not named: a member the person did not read.
        not_named: Vec<String>,
    },
}

impl std::fmt::Display for NotTheWave {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotTheWave::NoneHeld => out.write_str("no Job of its plan is waiting to be released"),
            NotTheWave::Differs {
                not_held,
                not_named,
            } => {
                out.write_str(
                    "the press did not name the wave Fleet holds, so none of it was released",
                )?;
                if !not_named.is_empty() {
                    write!(out, "; waiting and not named: {}", not_named.join(", "))?;
                }
                if !not_held.is_empty() {
                    write!(out, "; named and not waiting: {}", not_held.join(", "))?;
                }
                Ok(())
            }
        }
    }
}

/// Why an act on an Epic or one of its members was refused. Every one is a
/// 409: what the press asked of is not where it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// `approve_wave` named another set, or there was none.
    NotTheWave(NotTheWave),
    /// The parent is not at the gate of a step that proposes Jobs.
    NotAtItsPlan,
    /// `approve_review` at a plan gate holding a wave: approving the plan
    /// alone would strand every Job it proposed.
    WaveHeld { wave: Vec<JobId> },
    /// `approve_dispatch` on one member: it is released with its wave.
    ReleasedWithItsWave { parent: JobId },
    /// The parent finishes on its members' merges, and these have not merged.
    MembersNotLanded { unlanded: Vec<JobId> },
}

impl std::fmt::Display for Refused {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ids = |jobs: &[JobId]| {
            jobs.iter()
                .map(JobId::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        };
        match self {
            Refused::NotTheWave(why) => write!(out, "{why}"),
            Refused::NotAtItsPlan => out.write_str(
                "it is not waiting at the gate of a step that proposes Jobs, so there is no \
                 wave to release",
            ),
            Refused::WaveHeld { wave } => write!(
                out,
                "its plan proposed Jobs that are waiting to be released, and approving the plan \
                 alone would leave them waiting: approve the wave instead ({})",
                ids(wave)
            ),
            Refused::ReleasedWithItsWave { parent } => write!(
                out,
                "it is one Job of a wave {} proposed, and a wave is released whole, by \
                 approving that Job's plan",
                parent.as_str()
            ),
            Refused::MembersNotLanded { unlanded } => write!(
                out,
                "it finishes when every Job it holds has landed, and these have not merged: {}",
                ids(unlanded)
            ),
        }
    }
}

/// Whether `job` is one `parent` dispatched, by any step on any pass.
fn member_of(job: &Job, parent: &JobId) -> bool {
    job.origin() == Origin::SubDispatched
        && job.dispatched_by().is_some_and(|by| &by.job_id == parent)
}

/// Whether `job` is a member some pass proposed, still at its gate.
fn waiting(job: &Job) -> bool {
    job.status() == JobStatus::AwaitingApproval
        && job.dispatched_by().is_some_and(|by| by.pass.is_some())
}

/// The wave `parent` holds: every Job its plan proposed that nobody has
/// released or dropped, in board order. **This pass's alone** — an earlier
/// wave was released, or dropped, before the loop came back to propose again.
pub fn held(parent: &JobId, board: &[Job]) -> Vec<JobId> {
    board
        .iter()
        .filter(|job| member_of(job, parent) && waiting(job))
        .map(|job| job.id().clone())
        .collect()
}

/// The Jobs one press releases, or why it releases none. **All or nothing**: a
/// press naming part of the wave, or a Job it does not hold, moves nothing.
pub fn released(
    parent: &JobId,
    board: &[Job],
    asked: &ipc::ApproveWave,
) -> Result<Vec<JobId>, NotTheWave> {
    let held = held(parent, board);
    if held.is_empty() {
        return Err(NotTheWave::NoneHeld);
    }
    let named: Vec<&str> = asked.jobs.iter().map(ipc::JobId::as_str).collect();
    let mut not_held: Vec<String> = named
        .iter()
        .filter(|one| !held.iter().any(|job| job.as_str() == **one))
        .map(|one| one.to_string())
        .collect();
    not_held.dedup();
    let not_named: Vec<String> = held
        .iter()
        .filter(|job| !named.contains(&job.as_str()))
        .map(|job| job.as_str().to_string())
        .collect();
    match not_held.is_empty() && not_named.is_empty() {
        true => Ok(held),
        false => Err(NotTheWave::Differs {
            not_held,
            not_named,
        }),
    }
}

/// The members of `parent` whose pull request has not merged. **A member
/// somebody dropped is not one** — `killed`, or `rejected` at its gate — and
/// a member is landed by the merge, never by a successful status
/// (`.claude/decisions/2026-09-21-a-parent-job-holds-members.md`).
pub fn unlanded(parent: &JobId, board: &[Job], landed: &BTreeMap<JobId, Landing>) -> Vec<JobId> {
    board
        .iter()
        .filter(|job| member_of(job, parent))
        .filter(|job| !matches!(job.status(), JobStatus::Killed | JobStatus::Rejected))
        .filter(|job| !matches!(landed.get(job.id()), Some(Landing::Merged { .. })))
        .map(|job| job.id().clone())
        .collect()
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
    /// Approve an Epic's plan and release its wave, in one act.
    ///
    /// **Every refusal is answered before anything moves.** Then each member
    /// takes `awaiting_approval -> queued` as a person's act, as edited, and
    /// the parent's plan step advances and the parent is queued **without
    /// entering the step after it**: it stands after the step that proposed,
    /// which is what holds it until its members are done (`crate::sub_dispatch`).
    pub async fn approve_wave(
        &self,
        parent: &JobId,
        asked: &ipc::ApproveWave,
    ) -> Result<Job, Adrift> {
        let slot = self.slot_for(parent).await;
        let _working = slot.lock().await;
        let job = self.load(parent).await?;
        let step = self.at_the_gate(&job)?;
        if !proposes(&job, &step) {
            return Err(refused(parent, Refused::NotAtItsPlan));
        }
        let (board, _) = self.every_job().await?;
        let wave = released(parent, &board.jobs, asked)
            .map_err(|why| refused(parent, Refused::NotTheWave(why)))?;
        let members: Vec<&Job> = board
            .jobs
            .iter()
            .filter(|held| wave.contains(held.id()))
            .collect();
        // The machine is asked about every move before any is written, so a
        // member it would refuse refuses the press rather than half of it.
        let at = self.now();
        for member in &members {
            member
                .transition(Target::Queued, Actor::Human, at.clone())
                .map_err(Adrift::IllegalMove)?;
        }
        self.surviving_worktree(&job)?;
        for member in members {
            self.move_job(member, Target::Queued, Actor::Human).await?;
        }
        let job = self.move_step(&job, &step, StepTarget::Advanced).await?;
        self.move_job(&job, Target::Queued, Actor::Human).await
    }

    /// Refused where a person approves an Epic's plan alone while its wave is
    /// waiting: `approve_wave` is the act that approves it.
    pub(crate) async fn no_wave_held(&self, job: &Job, step: &StepId) -> Result<(), Adrift> {
        if !proposes(job, step) {
            return Ok(());
        }
        let (board, _) = self.every_job().await?;
        let wave = held(job.id(), &board.jobs);
        match wave.is_empty() {
            true => Ok(()),
            false => Err(refused(job.id(), Refused::WaveHeld { wave })),
        }
    }

    /// Refused where `approve_dispatch` names one member of a wave.
    pub(crate) fn not_alone(&self, job: &Job) -> Result<(), Adrift> {
        match job.dispatched_by().filter(|_| waiting(job)) {
            Some(by) => Err(refused(
                job.id(),
                Refused::ReleasedWithItsWave {
                    parent: by.job_id.clone(),
                },
            )),
            None => Ok(()),
        }
    }

    /// Refused where a Job that finishes on its members' merges is approved at
    /// its last step while one has not merged.
    pub(crate) async fn members_landed(&self, job: &Job) -> Result<(), Adrift> {
        if self.landing_of(job.id()).await.complete_when != CompleteWhen::AllMembersLanded {
            return Ok(());
        }
        let (board, _) = self.every_job().await?;
        let landed = self
            .store()
            .lock()
            .await
            .landed_by_job()
            .map_err(Adrift::Reading)?;
        let unlanded = unlanded(job.id(), &board.jobs, &landed);
        match unlanded.is_empty() {
            true => Ok(()),
            false => Err(refused(job.id(), Refused::MembersNotLanded { unlanded })),
        }
    }

    /// **A step that proposes Jobs, starting again, proposes its wave again**:
    /// whatever its last attempt proposed and nobody released is withdrawn
    /// first, by Fleet, so a retried plan cannot hold two copies of one wave.
    pub(crate) async fn proposals_withdrawn(&self, job: &Job, step: &StepId) -> Result<(), Adrift> {
        if !proposes(job, step) {
            return Ok(());
        }
        let (board, _) = self.every_job().await?;
        for member in board
            .jobs
            .iter()
            .filter(|held| member_of(held, job.id()) && waiting(held))
        {
            self.move_job(member, Target::Killed, Actor::Fleet).await?;
        }
        Ok(())
    }
}

/// Whether `step` of `job` is one that proposes Jobs.
fn proposes(job: &Job, step: &StepId) -> bool {
    job.workflow()
        .step(step)
        .is_some_and(core_model::ResolvedStep::may_dispatch_jobs)
}

fn refused(job: &JobId, why: Refused) -> Adrift {
    Adrift::WaveRefused {
        job: job.clone(),
        why,
    }
}
