//! The acts a person takes at a gate.
//!
//! **Every refusal here is one the transport can see.** Which trigger stopped a
//! step, and whether the daemon is still standing at it, are Fleet's to read off
//! the record and the slot; a `JobSummary` carries neither. So what the fake
//! checks is the shape of the body — a blank note, a blank reason, an unsaid
//! report — and it does not pretend to know the rest.

use ipc::{Instant, JobId};

use super::super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::Refusal;

impl FakeDaemon {
    /// Filing, faked on the two things the transport can see: the Job has to
    /// exist, and **a report with no sentence is not a report.** What Fleet
    /// attaches around the sentence is Fleet's — three reads and a render —
    /// and a fake that produced a record would be asserting about a bundle it
    /// invented.
    pub(super) async fn fake_file_report(
        &self,
        job_id: JobId,
        filing: ipc::FileReport,
    ) -> Result<ipc::Report, Refusal> {
        let job = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
                return Err(self.no_such_job(&job_id));
            };
            job.clone()
        };
        if filing.said.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.unsaid_report",
                "a report needs the sentence the record is context for".to_string(),
                run_id(),
            )));
        }
        let report = ipc::Report {
            id: ipc::ReportId::carried("01REPORT"),
            filed_at: Instant::carried("2026-08-28T21:00:00.000Z"),
            origin: ipc::ReportOrigin::Human,
            claim: filing.claim,
            job_id,
            job_title: job.title.clone(),
            step_id: filing.step_id,
            criterion_id: filing.criterion_id,
            said: filing.said,
            record: "## Every move it made".to_string(),
        };
        self.reports
            .lock()
            .expect("not poisoned")
            .push(report.clone());
        Ok(report)
    }
}
