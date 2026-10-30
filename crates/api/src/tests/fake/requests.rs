//! Requests, as the fake answers them: nothing is sent or kept, and the answer says who sent it.

use std::sync::Arc;

use ipc::{
    DeleteRequestEnvironment, RequestBy, RequestSaved, RequestSent, RequestToSend, Requests as RequestsRead, RunPlace,
    SaveRequestCollection, SaveRequestEnvironment, SaveRequestToCollection, SendRequest, SessionId,
};

use super::FakeDaemon;
use crate::{Caller, Refusal, Requests};

/// An empty read of `repository`, or of `repo` where nothing is named.
fn empty(repository: Option<String>) -> RequestsRead {
    RequestsRead {
        repository: repository.unwrap_or_else(|| String::from("repo")),
        place: RunPlace::ThisMac,
        collections: Vec::new(),
        environments: Vec::new(),
        not_read: Vec::new(),
    }
}

impl Requests for FakeDaemon {
    /// A 200 whose body is who the router said sent it, so a test reads the door's decision.
    async fn send_request(
        self: Arc<Self>,
        _caller: Option<Caller>,
        by: RequestBy,
        send: SendRequest,
    ) -> Result<RequestSent, Refusal> {
        let url = match send.request {
            RequestToSend::Own { url, .. } => url,
            RequestToSend::Saved(saved) => format!("{}/{}", saved.collection, saved.request),
        };
        Ok(RequestSent {
            url,
            status: Some(200),
            ms: 0,
            headers: Vec::new(),
            body: ipc::encode(&by).unwrap_or_default(),
            truncated: false,
            error: None,
            note: None,
            saved: None,
        })
    }

    async fn get_requests(
        &self,
        _caller: Option<Caller>,
        repository: Option<String>,
        _session_id: Option<SessionId>,
    ) -> Result<RequestsRead, Refusal> {
        Ok(empty(repository))
    }

    /// The collection answered as the person's own, as a save reads it back.
    async fn save_request_collection(
        &self,
        _caller: Option<Caller>,
        save: SaveRequestCollection,
    ) -> Result<RequestsRead, Refusal> {
        let mut read = empty(save.repository);
        read.collections.push(ipc::Placed { item: save.collection, place: ipc::RequestPlace::Yours, hides: false });
        Ok(read)
    }

    async fn save_request_environment(
        &self,
        _caller: Option<Caller>,
        save: SaveRequestEnvironment,
    ) -> Result<RequestsRead, Refusal> {
        Ok(empty(save.repository))
    }

    async fn delete_request_environment(
        &self,
        _caller: Option<Caller>,
        delete: DeleteRequestEnvironment,
    ) -> Result<RequestsRead, Refusal> {
        Ok(empty(delete.repository))
    }

    async fn save_request_to_collection(
        &self,
        _caller: Option<Caller>,
        save: SaveRequestToCollection,
    ) -> Result<RequestSaved, Refusal> {
        Ok(RequestSaved { collection_id: save.collection, request_id: save.request.name })
    }
}
