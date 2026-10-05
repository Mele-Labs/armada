//! A press to merge under `merge_by: push`: the Job joins its repository's line
//! and waits for what the line does with it. `crate::taking_turns` is the line.
//!
//! **The wait is the press's and the landing is not.** A caller that goes away,
//! or a Fleet that restarts, leaves the entry in the store, and the sweep lands
//! it and moves the Job on without anybody waiting.

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, NotMerged, Vcs, WorkProduct};
use core_model::{Actor, Job};
use store::LineState;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// How often a press that is not driving the line looks at its entry.
const LOOK_EVERY: Duration = Duration::from_millis(250);

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Join the line, drive it while it is free, and answer once the entry has
    /// landed and the Job has moved on, or been refused. **A second press on a
    /// Job already waiting joins the entry it has**, so a press made again
    /// after a restart keeps its place.
    pub(crate) async fn merged_through_the_line(
        &self,
        job: &Job,
        served: &Served,
        url: &str,
        by: Actor,
    ) -> Result<Job, Adrift> {
        let nonce = self.mint().ulid().as_str().to_string();
        let at = self.now();
        let entry = self
            .store()
            .lock()
            .await
            .join_the_line(served.root(), job.id(), &nonce, by.as_wire(), url, &at)
            .map_err(Adrift::Writing)?;
        loop {
            self.drive_the_line(served.root()).await?;
            let read = self
                .store()
                .lock()
                .await
                .line_entry(entry.id)
                .map_err(Adrift::Writing)?;
            let Some(read) = read else {
                return self.load(job.id()).await;
            };
            let refused = |kind: Option<&str>, said: Option<String>| Adrift::NotMerged {
                job: job.id().clone(),
                why: NotMerged::from_kind(kind.unwrap_or("refused"), said.unwrap_or_default()),
            };
            match read.state {
                LineState::Landed if read.finished => return self.load(job.id()).await,
                LineState::Refused => return Err(refused(read.kind.as_deref(), read.said)),
                LineState::Stopped => return Err(refused(None, read.said)),
                LineState::Waiting | LineState::Landed => {}
            }
            tokio::time::sleep(LOOK_EVERY).await;
        }
    }
}
