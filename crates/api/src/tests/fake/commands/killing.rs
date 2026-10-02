//! Pulse's two kills, faked on what Fleet answers. `fleet::ending`'s
//! `kill_process` and `kill_processes` are what this stands in for.
//!
//! **The tree is the one the fake's resources reading draws**, so a pid a test
//! sends is in it exactly where a panel drawn from `get_job_resources` would
//! have offered it. Anything else is Fleet's 409, with its code and the pid on
//! its own field, so a caller is proved against the refusal it will meet.

use ipc::{JobId, JobSummary, WireError, WireValue};

use super::super::FakeDaemon;
use crate::tests::shapes;
use crate::tests::shapes::run_id;
use crate::Refusal;

impl FakeDaemon {
    /// The Drone's own pid ends the Drone; any other pid in the tree is a
    /// child, and the Job comes back as it was.
    pub(super) async fn fake_kill_process(
        &self,
        job_id: JobId,
        pid: u32,
    ) -> Result<JobSummary, Refusal> {
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter_mut().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        let held = shapes::resources(job_id.clone()).processes;
        let Some(process) = held.iter().find(|process| process.pid == pid) else {
            return Err(Refusal::IllegalMove(
                WireError::raised(
                    "fleet.not_the_jobs_process",
                    format!("pid {pid} is not in {}'s process tree", job_id.as_str()),
                    run_id(),
                )
                .about_job(job_id)
                .with_field("pid", WireValue::Int(i64::from(pid))),
            ));
        };
        if process.recorded {
            job.assigned_drone = None;
        }
        Ok(job.clone())
    }

    /// Every process: the Drone goes, and the Job survives. **`Ok` where
    /// there is no Drone**, as Fleet's `kill_drone` answers on a Job with
    /// nothing in its slot.
    pub(super) async fn fake_kill_processes(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter_mut().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        job.assigned_drone = None;
        Ok(job.clone())
    }
}
