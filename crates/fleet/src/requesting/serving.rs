//! [`Requests`] for Fleet.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Requests};
use ipc::{
    ForgetRequestCookie, RequestBy, RequestEnvironments, RequestEnvironmentsFile,
    RequestEnvironmentsRead, RequestHosts, RequestSent, SendRequest, SessionId, SetRequestCookie,
    WireError,
};

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
    fn unbuilt(&self) -> Refusal {
        Refusal::Fault(WireError::raised("fleet.requests_unbuilt", "Requests is not built yet", self.run_id()))
    }
}

impl<H, V, W> Requests for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn send_request(
        self: Arc<Self>,
        _caller: Option<api::Caller>,
        _by: RequestBy,
        _send: SendRequest,
    ) -> Result<RequestSent, Refusal> {
        Err(self.unbuilt())
    }

    async fn list_request_hosts(&self) -> Result<RequestHosts, Refusal> {
        Err(self.unbuilt())
    }

    async fn set_request_cookie(&self, _set: SetRequestCookie) -> Result<RequestHosts, Refusal> {
        Err(self.unbuilt())
    }

    async fn forget_request_cookie(&self, _forget: ForgetRequestCookie) -> Result<RequestHosts, Refusal> {
        Err(self.unbuilt())
    }

    async fn list_request_environments(
        &self,
        _caller: Option<api::Caller>,
        _session_id: Option<SessionId>,
    ) -> Result<RequestEnvironments, Refusal> {
        Err(self.unbuilt())
    }

    async fn save_request_environments(
        &self,
        _file: RequestEnvironmentsFile,
    ) -> Result<RequestEnvironmentsRead, Refusal> {
        Err(self.unbuilt())
    }
}
