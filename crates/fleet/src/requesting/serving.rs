//! [`Requests`] for Fleet: a send run on a blocking thread, then written into the Job's thread.
//!
//! **The owner's send is told to the Job's agent**, in place of Fleet's message as a run's end is;
//! **the agent's own is only kept**, since it holds the answer. A send that places no Job writes
//! nothing.

use std::sync::Arc;

use adapter_traits::{AgentHarness, CookieError, Delivery, Secret, Vcs, WorkProduct};
use api::{Refusal, Requests};
use ipc::{
    ForgetRequestCookie, RequestBy, RequestEnvironment, RequestEnvironments,
    RequestEnvironmentsFile, RequestEnvironmentsRead, RequestHosts, RequestSent, SendRequest,
    ServerPhase, SessionId, SetRequestCookie, WireError, SERVER_ENVIRONMENT_PREFIX,
};

use super::{sent, Bounds, Sent, Unsent};
use crate::daemon::Fleet;

/// A request that cannot be sent. A 422.
const REQUEST_REFUSED: &str = "fleet.request_refused";
/// A host or a cookie the store will not take. A 422.
const COOKIE_REFUSED: &str = "fleet.request_cookie_refused";
/// The send's own thread failed. A 500.
const REQUEST_UNSENT: &str = "fleet.request_unsent";
/// The Keychain would not read or write. A 500.
const COOKIE_UNKEPT: &str = "fleet.request_cookie_unkept";

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
    fn cookie_refusal(&self, why: CookieError) -> Refusal {
        match why {
            CookieError::Refused(why) => Refusal::Unacceptable(WireError::raised(COOKIE_REFUSED, why, self.run_id())),
            CookieError::Unkept(why) => Refusal::Fault(WireError::raised(COOKIE_UNKEPT, why, self.run_id())),
        }
    }

    /// The Job a call is about: the hosted one holding the connection, else the one named.
    fn requesting_job(&self, caller: Option<api::Caller>, named: Option<&SessionId>) -> Option<String> {
        caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
    }

    /// One environment for each server running in `job`'s slot, at the first port it serves on.
    async fn server_environments(&self, job: Option<&str>) -> Vec<RequestEnvironment> {
        let Some(job) = job else { return Vec::new() };
        let Ok((_, hosting)) = self.session_and_hosting(job).await else { return Vec::new() };
        if hosting.lease_slot.is_none() {
            return Vec::new();
        }
        let Ok(served) = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id))) else {
            return Vec::new();
        };
        let slot = Self::directory_of(&served, &hosting);
        self.server_list()
            .servers
            .into_iter()
            .filter(|server| server.checkout.path == slot && server.phase != ServerPhase::Exited)
            .filter_map(|server| {
                let port = server.ports.first()?.port;
                Some(RequestEnvironment {
                    name: format!("{SERVER_ENVIRONMENT_PREFIX}{}", server.name),
                    base: format!("http://localhost:{port}"),
                })
            })
            .collect()
    }

    /// Where `job` is a Job Fleet knows, its thread keeps the row and an owner's send is told.
    /// **Best effort**: the answer is the caller's whether or not the thread took it.
    async fn request_kept(self: &Arc<Self>, job: &str, done: &Sent) {
        let row = done.row(self.row_id(job), self.instant());
        match done.by {
            RequestBy::Owner => {
                let _ = self.told_by_fleet_as(job, &done.told(), row).await;
            }
            RequestBy::Agent => {
                let hosted = self.session_and_hosting(job).await;
                if hosted.is_ok_and(|(session, _)| session.state != store::SessionState::Ended) {
                    self.row_put(job, row).await;
                }
            }
        }
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
        caller: Option<api::Caller>,
        by: RequestBy,
        send: SendRequest,
    ) -> Result<RequestSent, Refusal> {
        let job = self.requesting_job(caller, send.session_id.as_ref());
        let mut environments = match self.environments_read().await {
            RequestEnvironmentsRead::Read { file } => file.environment,
            RequestEnvironmentsRead::NotRead { .. } => Vec::new(),
        };
        environments.extend(self.server_environments(job.as_deref()).await);
        let requests = self.requests().clone();
        let done = tokio::task::spawn_blocking(move || {
            sent(&*requests.sender, &*requests.cookies, by, &send, &environments, &Bounds::shipped())
        })
        .await
        .map_err(|why| Refusal::Fault(WireError::raised(REQUEST_UNSENT, why.to_string(), self.run_id())))?
        .map_err(|unsent| match unsent {
            Unsent::Refused(why) => Refusal::Unacceptable(WireError::raised(REQUEST_REFUSED, why, self.run_id())),
            Unsent::Unkept(why) => Refusal::Fault(WireError::raised(COOKIE_UNKEPT, why, self.run_id())),
        })?;
        if let Some(job) = job {
            self.request_kept(&job, &done).await;
        }
        Ok(done.answer)
    }

    async fn list_request_hosts(&self) -> Result<RequestHosts, Refusal> {
        let cookies = Arc::clone(&self.requests().cookies);
        let hosts = tokio::task::spawn_blocking(move || cookies.hosts())
            .await
            .map_err(|why| self.cookie_refusal(CookieError::Unkept(why.to_string())))?
            .map_err(|why| self.cookie_refusal(why))?;
        Ok(RequestHosts { hosts })
    }

    async fn set_request_cookie(&self, set: SetRequestCookie) -> Result<RequestHosts, Refusal> {
        let cookies = Arc::clone(&self.requests().cookies);
        let SetRequestCookie { host, cookie } = set;
        tokio::task::spawn_blocking(move || cookies.keep(&host, Secret::new(cookie)))
            .await
            .map_err(|why| self.cookie_refusal(CookieError::Unkept(why.to_string())))?
            .map_err(|why| self.cookie_refusal(why))?;
        self.list_request_hosts().await
    }

    async fn forget_request_cookie(&self, forget: ForgetRequestCookie) -> Result<RequestHosts, Refusal> {
        let cookies = Arc::clone(&self.requests().cookies);
        tokio::task::spawn_blocking(move || cookies.forget(&forget.host))
            .await
            .map_err(|why| self.cookie_refusal(CookieError::Unkept(why.to_string())))?
            .map_err(|why| self.cookie_refusal(why))?;
        self.list_request_hosts().await
    }

    async fn list_request_environments(
        &self,
        caller: Option<api::Caller>,
        session_id: Option<SessionId>,
    ) -> Result<RequestEnvironments, Refusal> {
        let job = self.requesting_job(caller, session_id.as_ref());
        Ok(RequestEnvironments {
            saved: self.environments_read().await,
            servers: self.server_environments(job.as_deref()).await,
        })
    }

    async fn save_request_environments(
        &self,
        file: RequestEnvironmentsFile,
    ) -> Result<RequestEnvironmentsRead, Refusal> {
        self.environments_saved(file).await
    }
}
