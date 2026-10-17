//! Ending a pilot, as the fake answers it: the status moves and nothing else does. What the real
//! thing does is `fleet::piloting`'s, against a scripted Fleet.

use ipc::{JobId, JobSummary, PilotNote};

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
