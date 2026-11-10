//! Requests, as the fake answers them: nothing is sent or kept, and the answer says who sent it.

use std::sync::Arc;

use ipc::{
    DeleteRequestEnvironment, RequestBy, RequestLine, RequestRun, RequestRunAnswer, RequestRunSent, RequestRunSummary,
    RequestRuns, RequestSaved, RequestSent, RequestToSend, Requests as RequestsRead, RequestsSent, RunPlace,
    SaveRequestCollection, SaveRequestEnvironment, SaveRequestToCollection, SendRequest, SendRequests, SessionId,
};

use super::FakeDaemon;
use crate::{Caller, Refusal, Requests, RunsAsked};

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

    /// One line for each body file, named for the template's address, so a test reads what arrived.
    async fn send_requests(&self, _caller: Option<Caller>, send: SendRequests) -> Result<RequestsSent, Refusal> {
        let lines = send
            .body_files
            .iter()
            .map(|file| RequestLine {
                name: format!("{}:{file}", send.template.url),
                status: Some(200),
                ms: 0,
                saved: None,
                error: None,
                skipped: false,
            })
            .collect();
        Ok(RequestsSent { folder: String::new(), lines, note: None })
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

    /// One run, whose address says what was asked so a test reads what arrived through the route.
    async fn list_request_runs(
        &self,
        _caller: Option<Caller>,
        asked: RunsAsked,
    ) -> Result<RequestRuns, Refusal> {
        let url = format!(
            "{}/{}?by={:?}&status={:?}&before={:?}&limit={:?}",
            asked.collection,
            asked.request,
            asked.by,
            asked.status.map(ipc::RequestRunStatus::as_wire),
            asked.before,
            asked.limit,
        );
        let run = RequestRunSummary {
            id: String::from("1"),
            at: ipc::Instant::carried("2026-11-10T10:00:00Z"),
            by: RequestBy::Owner,
            job: None,
            method: String::from("GET"),
            url,
            status: Some(200),
            ms: 0,
            error: None,
            summary: String::new(),
        };
        Ok(RequestRuns { runs: vec![run], next: None })
    }

    async fn get_request_run(
        &self,
        _caller: Option<Caller>,
        reader: RequestBy,
        _repository: Option<String>,
        _session_id: Option<SessionId>,
        id: String,
    ) -> Result<RequestRun, Refusal> {
        Ok(RequestRun {
            id,
            at: ipc::Instant::carried("2026-11-10T10:00:00Z"),
            by: reader,
            job: None,
            collection: String::from("users"),
            request: String::from("me"),
            environment: None,
            sent: RequestRunSent { method: String::from("GET"), url: String::new(), headers: Vec::new(), body: None },
            answer: RequestRunAnswer {
                status: Some(200),
                ms: 0,
                headers: Vec::new(),
                body: String::new(),
                truncated: false,
                bytes: 0,
                content_type: None,
                error: None,
            },
        })
    }
}
