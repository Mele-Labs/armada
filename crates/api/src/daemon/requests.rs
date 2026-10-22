//! Requests: collections of saved requests and environments of variables for a repository, and
//! an HTTP request sent filled from them.
//!
//! **Its own surface**, for [`Sessions`](super::Sessions)' reason: whole on its own.

use std::future::Future;

use ipc::{
    RequestBy, RequestSaved, RequestSent, Requests as RequestsRead, SaveRequestCollection,
    SaveRequestEnvironment, SaveRequestToCollection, SendRequest, SessionId,
};

use crate::daemon::Refusal;

pub trait Requests: Send + Sync + 'static {
    /// `send_request` — send it and answer how it went. `caller` places a hosted Job as
    /// `waiting_for`'s does; `by` is who sent it, which decides what the answer may carry and
    /// whether the Job is told. **Takes the `Arc`**: the owner's send is told to the Job.
    /// [`Refusal::Unacceptable`] for a request that cannot be sent; a request nothing answered is
    /// an answer.
    fn send_request(
        self: std::sync::Arc<Self>,
        caller: Option<crate::Caller>,
        by: RequestBy,
        send: SendRequest,
    ) -> impl Future<Output = Result<RequestSent, Refusal>> + Send;

    /// `get_requests` — both files for the repository `repository` names, or the Job `caller` or
    /// `session_id` places, and the Job's servers.
    fn get_requests(
        &self,
        caller: Option<crate::Caller>,
        repository: Option<String>,
        session_id: Option<SessionId>,
    ) -> impl Future<Output = Result<RequestsRead, Refusal>> + Send;

    /// `save_request_collection` — one collection whole into the person's own file.
    fn save_request_collection(
        &self,
        caller: Option<crate::Caller>,
        save: SaveRequestCollection,
    ) -> impl Future<Output = Result<RequestsRead, Refusal>> + Send;

    /// `save_request_environment` — one environment whole into the person's own file, its
    /// secrets into the Keychain.
    fn save_request_environment(
        &self,
        caller: Option<crate::Caller>,
        save: SaveRequestEnvironment,
    ) -> impl Future<Output = Result<RequestsRead, Refusal>> + Send;

    /// `save_request_to_collection` — the agent keeps one request in a collection. `caller`
    /// places the Job as `send_request`'s does.
    fn save_request_to_collection(
        &self,
        caller: Option<crate::Caller>,
        save: SaveRequestToCollection,
    ) -> impl Future<Output = Result<RequestSaved, Refusal>> + Send;
}
