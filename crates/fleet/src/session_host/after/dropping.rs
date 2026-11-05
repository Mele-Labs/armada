//! The Job that started a waiting Job drops its wait: the card stays held for the person to press and
//! waits on nothing. `docs/concepts/session/a-job-that-started-jobs.md`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{WaitBy, WaiterSide};

use crate::daemon::Fleet;

use super::row_label;

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
    /// `starter` drops the wait of `waiting`. **Held whatever the person approved**, since what he
    /// approved was a start when the wait ended: the brief alone is the first message, with no handoff
    /// from a Job it no longer waits on. The Job waited on is told, as a change tells it.
    pub(crate) async fn wait_dropped(self: &Arc<Self>, starter: &str, waiting: &str) -> Result<(), Refusal> {
        let wait = self.open_wait_of(waiting).await?;
        let at = self.now().as_str().to_string();
        // Once: the first to claim the wait ends it, and the watcher then finds it started.
        if !matches!(self.store().lock().await.claim_wait_start(waiting, &at), Ok(true)) {
            return Err(self.wait_over(waiting));
        }
        self.held_with(waiting, &wait.brief).await?;

        let on = wait.on_session.clone();
        let parts = self.plan_parts(&on).await;
        let said = (row_label(&wait, &parts), String::from("No wait"));
        let (waiting_title, on_title) = (self.waiting_title(&wait).await, self.session_title(&on).await);
        let for_waited_on = self.changed_row(&on, waiting, &waiting_title, WaiterSide::Waited, &said, Some(WaitBy::Agent));
        if starter == on {
            self.row_put(&on, for_waited_on).await;
        } else {
            let starter_title = self.session_title(starter).await;
            let text = format!(
                "Job \"{starter_title}\" ({starter}) dropped the wait of job \"{waiting_title}\" ({waiting}), which it started: it no longer waits on this job."
            );
            self.told_or_kept(&on, &text, for_waited_on).await;
        }
        self.row_put(waiting, self.changed_row(waiting, &on, &on_title, WaiterSide::Waiting, &said, Some(WaitBy::Agent))).await;
        let _ = self.published_hosted(&on).await;
        let _ = self.published_hosted(waiting).await;
        Ok(())
    }
}
