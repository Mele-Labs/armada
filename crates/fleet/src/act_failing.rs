//! A background act that failed after its request was answered, told on the stream.
//! [`ipc::ActFailed`] is the event; a request that replies first and finishes in a spawned task
//! calls [`Fleet::act_failed`] with the refusal it has no answer left to carry.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ActFailed, BackgroundAct, ManifestId, SessionId};

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
    /// Say that `act` failed with `refusal`, for the repository and the Job it had, if it had them.
    pub(crate) fn act_failed(
        &self,
        act: BackgroundAct,
        manifest: Option<ManifestId>,
        session: Option<SessionId>,
        refusal: Refusal,
    ) {
        self.publish(ipc::Event::ActFailed(ActFailed {
            act,
            manifest_id: manifest,
            session_id: session,
            error: refusal.error().clone(),
        }));
    }
}
