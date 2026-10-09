//! Retros, as the fake answers them: an empty record for any Job it holds, and
//! a listing of nothing. **What it records is the door each read came
//! through**, which is the transport's to name and what the tests here ask.

use std::sync::Arc;

use ipc::{
    JobId, JobRetro, LandsIn, Lesson, LessonState, Lessons, ManifestId, RetroRecord, RetroState,
};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Redirector, Refusal, Retros};

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
            session: None,
            annotations: Vec::new(),
        })
    }

    async fn get_session_retro(&self, session_id: String) -> Result<JobRetro, Refusal> {
        Ok(self.session_retro(&session_id))
    }

    async fn write_session_retro(
        self: Arc<Self>,
        session_id: String,
    ) -> Result<JobRetro, Refusal> {
        Ok(self.session_retro(&session_id))
    }

    async fn list_lessons(
        &self,
        _manifest_id: Option<ManifestId>,
        _lands_in: Option<LandsIn>,
        _state: Option<LessonState>,
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

    async fn agree_lesson(
        self: Arc<Self>,
        lesson_id: String,
        _by: Redirector,
    ) -> Result<Lesson, Refusal> {
        Err(self.refusing_lesson(&lesson_id))
    }

    async fn disagree_lesson(&self, lesson_id: String) -> Result<Lesson, Refusal> {
        Err(self.refusing_lesson(&lesson_id))
    }
}

impl FakeDaemon {
    fn session_retro(&self, session_id: &str) -> JobRetro {
        JobRetro {
            job_id: JobId::carried(session_id),
            state: RetroState::Pending,
            at: None,
            model: None,
            why: None,
            items: Vec::new(),
            record: RetroRecord::default(),
            session: Some(ipc::RetroSession {
                id: session_id.to_string(),
                title: None,
            }),
            annotations: Vec::new(),
        }
    }

    /// The one id the fake knows answers with a 422, and any other is one
    /// nothing answers to.
    fn refusing_lesson(&self, lesson_id: &str) -> Refusal {
        if lesson_id == crate::tests::shapes::THE_LESSON {
            return Refusal::Unacceptable(ipc::WireError::raised(
                "fake.lesson_names_no_place",
                format!("{lesson_id} names no place its fix lands"),
                run_id(),
            ));
        }
        self.no_such_lesson(lesson_id)
    }

    /// The fake holds no retro items, so every id is one nothing answers to.
    fn no_such_lesson(&self, lesson_id: &str) -> Refusal {
        Refusal::NoSuchJob(ipc::WireError::raised(
            "fake.no_such_lesson",
            format!("no retro item is named {lesson_id}"),
            run_id(),
        ))
    }
}
