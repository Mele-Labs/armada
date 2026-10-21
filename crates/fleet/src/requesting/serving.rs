//! [`Requests`] for Fleet: a send run on a blocking thread, then written into the Job's thread,
//! and the reads and saves of a repository's collections and environments.
//!
//! **The owner's send is told to the Job's agent**, in place of Fleet's message as a run's end is;
//! **the agent's own send and save are only kept**, since it holds the answer. A send that places
//! no Job writes nothing.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Requests};
use ipc::{
    RequestBy, RequestEnvironment, RequestSaved, RequestSent, RequestVariable, Requests as RequestsRead,
    SaveRequestCollection, SaveRequestEnvironment, SaveRequestToCollection, SendRequest, ServerPhase,
    SessionId, SessionRow, WireError, SERVER_BASE_VARIABLE, SERVER_ENVIRONMENT_PREFIX,
};

use super::saving::{SavedTo, SECRET_UNKEPT};
use super::{sent, Bounds, Sent, Unsent};
use crate::daemon::Fleet;

/// A request that cannot be sent. A 422.
const REQUEST_REFUSED: &str = "fleet.request_refused";
/// The send's own thread failed. A 500.
const REQUEST_UNSENT: &str = "fleet.request_unsent";
/// Nothing names a repository, or a Job for an agent's save. A 422.
const UNPLACED: &str = "fleet.requests_unplaced";

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
    /// The Job a call is about: the hosted one holding the connection, else the one named.
    fn requesting_job(&self, caller: Option<api::Caller>, named: Option<&SessionId>) -> Option<String> {
        caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
    }

    fn unplaced(&self, why: &str) -> Refusal {
        Refusal::Unacceptable(WireError::raised(UNPLACED, why.to_string(), self.run_id()))
    }

    /// The repository `repository` names, else the one `job` stands in.
    async fn requests_root(&self, repository: Option<&str>, job: Option<&str>) -> Result<Option<String>, Refusal> {
        match repository.map(str::trim).filter(|named| !named.is_empty()) {
            Some(named) => self.run_repository(named).map(Some),
            None => match job {
                Some(job) => Ok(self.job_repository(job).await),
                None => Ok(None),
            },
        }
    }

    /// The repository and Job a read or a save is about. **Some repository must be placed.**
    async fn requests_placed(
        &self,
        caller: Option<api::Caller>,
        repository: Option<&str>,
        session_id: Option<&SessionId>,
    ) -> Result<(String, Option<String>), Refusal> {
        let job = self.requesting_job(caller, session_id);
        match self.requests_root(repository, job.as_deref()).await? {
            Some(root) => Ok((root, job)),
            None => Err(self.unplaced("Nothing names a repository: name one, or a Job that stands in one.")),
        }
    }

    /// One environment for each server running in `job`'s slot, at the first port it serves on.
    pub(super) async fn server_environments(&self, job: Option<&str>) -> Vec<RequestEnvironment> {
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
                    variables: vec![RequestVariable::Plain {
                        name: SERVER_BASE_VARIABLE.to_string(),
                        value: format!("http://localhost:{port}"),
                    }],
                })
            })
            .collect()
    }

    /// Whether `job` is a Job whose thread is still written.
    async fn thread_open(&self, job: &str) -> bool {
        let hosted = self.session_and_hosting(job).await;
        hosted.is_ok_and(|(session, _)| session.state != store::SessionState::Ended)
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
                if self.thread_open(job).await {
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
        let root = self.requests_root(send.repository.as_deref(), job.as_deref()).await?;
        let filling = self.filling_for(root.as_deref(), job.as_deref()).await?;
        let requests = self.requests().clone();
        let done = tokio::task::spawn_blocking(move || {
            sent(&*requests.sender, &*requests.secrets, by, &send, &filling, &Bounds::shipped())
        })
        .await
        .map_err(|why| Refusal::Fault(WireError::raised(REQUEST_UNSENT, why.to_string(), self.run_id())))?
        .map_err(|unsent| match unsent {
            Unsent::Refused(why) => Refusal::Unacceptable(WireError::raised(REQUEST_REFUSED, why, self.run_id())),
            Unsent::Unkept(why) => Refusal::Fault(WireError::raised(SECRET_UNKEPT, why, self.run_id())),
        })?;
        if let Some(job) = job {
            self.request_kept(&job, &done).await;
        }
        Ok(done.answer)
    }

    async fn get_requests(
        &self,
        caller: Option<api::Caller>,
        repository: Option<String>,
        session_id: Option<SessionId>,
    ) -> Result<RequestsRead, Refusal> {
        let (root, job) = self.requests_placed(caller, repository.as_deref(), session_id.as_ref()).await?;
        self.requests_answer(&root, job.as_deref()).await
    }

    async fn save_request_collection(
        &self,
        caller: Option<api::Caller>,
        save: SaveRequestCollection,
    ) -> Result<RequestsRead, Refusal> {
        let (root, job) = self.requests_placed(caller, save.repository.as_deref(), save.session_id.as_ref()).await?;
        self.request_collection_saved(&root, job.as_deref(), save.collection).await
    }

    async fn save_request_environment(
        &self,
        caller: Option<api::Caller>,
        save: SaveRequestEnvironment,
    ) -> Result<RequestsRead, Refusal> {
        let (root, job) = self.requests_placed(caller, save.repository.as_deref(), save.session_id.as_ref()).await?;
        self.request_environment_saved(&root, job.as_deref(), save.environment).await
    }

    async fn save_request_to_collection(
        &self,
        caller: Option<api::Caller>,
        save: SaveRequestToCollection,
    ) -> Result<RequestSaved, Refusal> {
        let job = self
            .requesting_job(caller, save.session_id.as_ref())
            .ok_or_else(|| self.unplaced("A request is saved from a Job, and nothing places one."))?;
        let root = self
            .job_repository(&job)
            .await
            .ok_or_else(|| self.unplaced("The Job stands in no repository Fleet serves."))?;
        let SavedTo { saved, name, collection } =
            self.request_saved_to_collection(&root, &job, &save.collection, save.request).await?;
        if self.thread_open(&job).await {
            let row = SessionRow::Saved {
                id: self.row_id(&job),
                at: self.instant(),
                name,
                collection,
                collection_id: saved.collection_id.clone(),
                request: saved.request_id.clone(),
            };
            self.row_put(&job, row).await;
        }
        Ok(saved)
    }
}
