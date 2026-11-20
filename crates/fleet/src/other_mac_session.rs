//! A Job held from another Mac, opened by `get_session`: its row and its thread, read from the Mac that holds it, each
//! time, as `get_job_thread` reads its folded thread. This Mac keeps none of it.
//! `docs/practices/protocol/his-other-macs-fleets-half.md`, the `get_session` row.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SessionId, SessionThread, SessionWindow};
use std::sync::Arc;

use crate::daemon::Fleet;
use crate::other_macs::sent_on;

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
    /// A Job held from Mac `mac`: its row and thread, read from that Mac's own Fleet. **The window is cut there**, so
    /// only the newest rows cross a link whose body stops at 1 MiB and not the whole of a long Job; nothing is cut
    /// where `window` is `None`. The row carries `mac`, as it does in the list.
    pub(crate) async fn session_from_other_mac(&self, mac: &str, job: &SessionId, window: Option<&SessionWindow>) -> Result<SessionThread, Refusal> {
        let mut path = format!("/other_mac/session?session_id={}", ipc::door::encoded(job.as_str()));
        if let Some(SessionWindow { last, before }) = window {
            path.push_str(&format!("&last={last}"));
            if let Some(before) = before {
                path.push_str(&format!("&before={}", ipc::door::encoded(before)));
            }
        }
        let mut thread: SessionThread = self.read_from_other_mac(mac, &path, "a Job's thread from another Mac").await?;
        thread.session.mac = Some(mac.to_string());
        Ok(thread)
    }

    /// Job `job` of this Mac's, for another Mac: its row and thread as `get_session` answers them, cut to `window`.
    /// **Refused as an act on a Job that does not exist** where the Job is not shared.
    pub(crate) async fn job_session_shared(self: Arc<Self>, job: SessionId, window: Option<SessionWindow>) -> Result<SessionThread, Refusal> {
        self.act_from_other_mac(&job).await?;
        let mut thread = api::HostedSessions::get_session(Arc::clone(&self), job).await?;
        thread.session = sent_on(thread.session);
        Ok(match window {
            Some(window) => thread.windowed(&window),
            None => thread,
        })
    }
}
