//! Ending a Drone, ending a Job, and putting one back on its feet.
//!
//! **The Drone and the Job are separate acts on separate things**, which is why
//! `kill_drone` and `kill_job` are two operations and refuse on different
//! statuses. Terminality is read through the DTO from the domain rather than
//! restated here: a second list of which statuses are over is a second
//! vocabulary.

use ipc::{
    JobForgotten, JobId, JobSummary,
    WorktreeReclaimed,
};

use super::super::FakeDaemon;
use crate::tests::shapes;
use crate::tests::shapes::run_id;
use crate::Refusal;

impl FakeDaemon {
    /// The unit of work, not the process. Legal from wherever the Job is, so
    /// long as it is not already over — including the statuses `kill_drone`
    /// refuses, which is the whole reason the two are separate operations.
    pub(super) async fn fake_kill_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let from = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
                return Err(self.no_such_job(&job_id));
            };
            // Terminality is the domain's, read through the DTO rather than
            // restated: a second list of which statuses are over is a second
            // vocabulary.
            if job.status.domain().is_terminal() {
                return Err(Refusal::IllegalMove(ipc::WireError::raised(
                    "fake.illegal_move",
                    format!("a Job at {} is already over", job.status.as_wire()),
                    run_id(),
                )));
            }
            job.status.as_wire()
        };
        self.move_to(&job_id, from, "killed", "human")
    }
    /// The record, gone. **The fake actually removes it**, the same as
    /// `Store::forget_job` does on the real one, so a caller that forgets a
    /// Job and then asks for it again sees exactly what asking for an id that
    /// never existed sees.
    pub(super) async fn fake_forget_job(&self, job_id: JobId) -> Result<JobForgotten, Refusal> {
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        // Terminality is the domain's, read through the DTO rather than
        // restated, for `kill_job`'s reason.
        if !job.status.domain().is_terminal() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.not_forgettable",
                format!("a Job at {} cannot be forgotten", job.status.as_wire()),
                run_id(),
            )));
        }
        jobs.retain(|job| job.id != job_id);
        Ok(JobForgotten { job_id })
    }
    /// The disk, given back. **The record stays** — that is the whole
    /// difference from the method above, and the fake keeps the row so a test
    /// that reclaims and then reads the Job still finds it.
    ///
    /// Terminality is the domain's, read through the DTO rather than restated,
    /// for `kill_job`'s reason.
    pub(super) async fn fake_reclaim_worktree(
        &self,
        job_id: JobId,
    ) -> Result<WorktreeReclaimed, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if !job.status.domain().is_terminal() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.not_reclaimable",
                format!("a Job at {} has no disk to give back", job.status.as_wire()),
                run_id(),
            )));
        }
        Ok(shapes::reclaimed(job_id))
    }
    /// The branch, deleted. **The fake has no repository**, so it refuses only
    /// on the status and answers with the tip it was sent.
    pub(super) async fn fake_delete_branch(
        &self,
        job_id: JobId,
        asked: ipc::DeleteBranch,
    ) -> Result<ipc::BranchDeleted, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if !job.status.domain().is_terminal() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.branch_not_deletable",
                format!("a Job at {} still has its branch", job.status.as_wire()),
                run_id(),
            )));
        }
        let branch = format!("armada/{}", job_id.as_str());
        Ok(ipc::BranchDeleted {
            job_id,
            branch,
            tip: asked.tip,
        })
    }
}
