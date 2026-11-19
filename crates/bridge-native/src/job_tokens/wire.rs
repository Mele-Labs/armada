//! A Job's tokens over time, as Fleet's `GET /usage/job_timeline?session_id=` serves them, read off the main thread on the port the
//! Fleet connection read. `crates/ipc/operations/get_job_timeline.toml`.

use gpui::{App, Task};
pub use ipc::{JobTimeline, TimelineBucket as Bucket};

use crate::settings::wire::{request, Ask, Refused};

/// `GET /usage/job_timeline?session_id=<id>`.
pub fn timeline(port: u16, job: &str, cx: &App) -> Task<Result<JobTimeline, Refused>> {
    request(port, Ask::Get(format!("/usage/job_timeline?session_id={}", crate::link::percent_encoded(job))), "JobTimeline", cx)
}
