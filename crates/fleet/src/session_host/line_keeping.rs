//! The line for a slot, kept in the store as Jobs join and leave it, and read back once as Fleet
//! starts, so a restart serves the same Jobs in the same order. **What the store cannot keep is
//! let go**: a write that fails leaves the line in memory as it was, and only a restart loses it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::Instant;
use store::KeptInLine;

use crate::daemon::Fleet;

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
    /// `id` joined `manifest`'s line at `place`.
    pub(crate) async fn line_kept(&self, manifest: &str, id: &str, place: i64, since: &Instant) {
        let one = KeptInLine { session_id: id.to_string(), manifest_id: manifest.to_string(), place, since: since.as_str().to_string() };
        if let Err(why) = self.store().lock().await.keep_in_slot_line(&one) {
            eprintln!("job {id}: its place in the line for a slot was not kept: {why}");
        }
    }

    /// `id` is in no line now.
    pub(crate) async fn line_let_go(&self, id: &str) {
        let _ = self.store().lock().await.leave_slot_line(id);
    }

    /// Every Job the store has in a line, back in it at its place, its wait begun again and its call
    /// raised again. **One that cannot wait any more is let go**: ended, leased a slot, or paused
    /// with no Resume pressed. Then the line is looked at, since a slot may have freed meanwhile.
    pub(crate) async fn slot_lines_restored(self: &Arc<Self>) {
        let kept = match self.store().lock().await.slot_lines() {
            Ok(kept) => kept,
            Err(why) => {
                eprintln!("the lines for a slot were not read back: {why}");
                return;
            }
        };
        let queue = Arc::clone(self.hosts().slot_queue());
        for one in kept {
            let waits = match self.session_and_hosting(&one.session_id).await {
                Ok((session, hosting)) => {
                    session.state != store::SessionState::Ended
                        && hosting.lease_slot.is_none()
                        && (hosting.paused_at.is_none() || hosting.paused_resuming)
                }
                Err(_) => false,
            };
            if !waits {
                self.line_let_go(&one.session_id).await;
                continue;
            }
            queue.join_at(&one.manifest_id, &one.session_id, one.place, &Instant::carried(&one.since));
            self.slot_wait_began(&one.session_id, "line").await;
            let _ = self.published_hosted(&one.session_id).await;
        }
        queue.freed();
    }
}
