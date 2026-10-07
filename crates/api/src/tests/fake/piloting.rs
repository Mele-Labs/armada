//! Taking a Job over, as the fake answers it: the status moves and nothing
//! else does. What the real thing ends and hands over is
//! `fleet::piloting`'s, against a scripted Fleet.

use ipc::{HandoffBundle, JobId, JobSummary, PilotNote, TakeOver};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Piloting, Refusal};

impl FakeDaemon {
    fn not_piloted(&self, job_id: &JobId) -> Refusal {
        Refusal::IllegalMove(
            ipc::WireError::raised(
                "fleet.not_piloted",
                "the fake's Job is not piloted",
                run_id(),
            )
            .about_job(job_id.clone()),
        )
    }

    fn left_piloting(&self, job_id: &JobId, to: &str) -> Result<JobSummary, Refusal> {
        self.move_to(job_id, "piloted", to, "human")
            .map_err(|_| self.not_piloted(job_id))
    }
}

impl Piloting for FakeDaemon {
    async fn take_over(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        request: TakeOver,
    ) -> Result<JobSummary, Refusal> {
        let status = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let job = jobs
                .iter()
                .find(|job| job.id == job_id)
                .ok_or_else(|| self.no_such_job(&job_id))?;
            job.status.as_wire().to_string()
        };
        self.move_to(&job_id, &status, "piloted", "human")?;
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let job = jobs
            .iter_mut()
            .find(|job| job.id == job_id)
            .ok_or_else(|| self.no_such_job(&job_id))?;
        job.piloted = Some(ipc::Piloted {
            reason: match request.outcome {
                ipc::PilotOutcome::TakeOver => "take_over",
                ipc::PilotOutcome::RestartStep => "restart_step",
            }
            .to_string(),
            session_id: request.session_id,
            since: ipc::Instant::carried("2026-10-07T09:00:00.000Z"),
            exit: None,
            ended_at: None,
            note: None,
        });
        Ok(job.clone())
    }

    async fn get_handoff(&self, job_id: JobId) -> Result<HandoffBundle, Refusal> {
        Err(self.not_piloted(&job_id))
    }

    async fn submit_for_verification(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> Result<JobSummary, Refusal> {
        self.left_piloting(&job_id, "running")
    }

    async fn attest_complete(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _note: PilotNote,
    ) -> Result<JobSummary, Refusal> {
        self.left_piloting(&job_id, "completed_success")
    }

    async fn close_as_superseded(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        _note: PilotNote,
    ) -> Result<JobSummary, Refusal> {
        self.left_piloting(&job_id, "superseded")
    }
}
