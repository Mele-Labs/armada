//! Retros, as the fake answers them: an empty record for any Job it holds, and
//! a listing of nothing. **What it records is the door each read came
//! through**, which is the transport's to name and what the tests here ask.

use ipc::{JobId, JobRetro, LandsIn, Lessons, ManifestId, RetroRecord, RetroState};

use super::FakeDaemon;
use crate::{Refusal, Retros};

impl Retros for FakeDaemon {
    async fn get_job_retro(&self, job_id: JobId) -> Result<JobRetro, Refusal> {
        self.read_via
            .lock()
            .expect("not poisoned")
            .push(crate::via());
        let held = self
            .jobs
            .lock()
            .expect("not poisoned")
            .iter()
            .any(|job| job.id == job_id);
        if !held {
            return Err(self.no_such_job(&job_id));
        }
        Ok(JobRetro {
            job_id,
            state: RetroState::Pending,
            at: None,
            model: None,
            why: None,
            items: Vec::new(),
            record: RetroRecord::default(),
            annotations: Vec::new(),
        })
    }

    async fn list_lessons(
        &self,
        _manifest_id: Option<ManifestId>,
        _lands_in: Option<LandsIn>,
        _most: u32,
    ) -> Result<Lessons, Refusal> {
        self.read_via
            .lock()
            .expect("not poisoned")
            .push(crate::via());
        Ok(Lessons {
            lessons: Vec::new(),
        })
    }
}
