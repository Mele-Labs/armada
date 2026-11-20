//! The artifact a press on a Designs row leaves for the Job it opens. The Job's view takes it with [`take`] once it holds
//! that Job's artifacts, and opens it as a press on its Artifacts tab does.

use gpui::{App, Global};

use crate::titlebar;

/// The artifact to open, by its id, and the Job it is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactRequest {
    pub job: String,
    pub artifact: String,
}

#[derive(Default)]
struct Pending(Option<ArtifactRequest>);

impl Global for Pending {}

/// Open `artifact` of Job `job`: the request is left, the Job opens on the Jobs screen, and the Jobs screen comes to the front.
pub fn open(job: String, artifact: String, cx: &mut App) {
    cx.default_global::<Pending>().0 = Some(ArtifactRequest { job: job.clone(), artifact });
    if let Some(sink) = cx.try_global::<titlebar::AskedSink>().cloned() {
        (sink.0)(&titlebar::Asked::OpenJob(job), cx);
    }
    crate::palette::show_jobs(cx);
}

/// Take the request, which leaves none.
pub fn take(cx: &mut App) -> Option<ArtifactRequest> {
    cx.default_global::<Pending>().0.take()
}

/// The request left, if nobody has taken it, which stays.
pub fn peek(cx: &App) -> Option<ArtifactRequest> {
    cx.try_global::<Pending>().and_then(|pending| pending.0.clone())
}

/// Leave a request as a press would, without opening the Job.
#[cfg(test)]
pub fn leave(request: ArtifactRequest, cx: &mut App) {
    cx.default_global::<Pending>().0 = Some(request);
}
