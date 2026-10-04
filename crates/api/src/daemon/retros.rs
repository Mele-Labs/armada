//! A Job's retro and the Lessons listing. Since 23.12. `docs/concepts/retro.md`.
//!
//! **A seventh surface, for [`Studios`](super::Studios)' reason.** Both reads
//! are about what a Job's record says once it is over, and neither is a fact
//! `get_job` or the Board reads.

use std::future::Future;

use ipc::{JobId, JobRetro, LandsIn, Lessons, ManifestId};

use crate::daemon::Refusal;

pub trait Retros: Send + Sync + 'static {
    /// `get_job_retro`. [`Refusal::NoSuchJob`] where no Job is `job_id`; a Job
    /// that has not ended answers `pending`, with its record as it stands.
    fn get_job_retro(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobRetro, Refusal>> + Send;

    /// `list_lessons` — up to `most` items, newest retro first. `manifest_id`
    /// absent is every repository served, and `lands_in` absent is every place
    /// a fix lands, an item kept before 23.15 included.
    fn list_lessons(
        &self,
        manifest_id: Option<ManifestId>,
        lands_in: Option<LandsIn>,
        most: u32,
    ) -> impl Future<Output = Result<Lessons, Refusal>> + Send;
}
