//! Requests, as the fake answers them: nothing is sent, and the answer says who sent it.

use std::sync::Arc;

use ipc::{
    ForgetRequestCookie, RequestBy, RequestEnvironments, RequestEnvironmentsFile,
    RequestEnvironmentsRead, RequestHosts, RequestSent, SendRequest, SessionId, SetRequestCookie,
};

use super::FakeDaemon;
use crate::{Caller, Refusal, Requests};

impl Requests for FakeDaemon {
    /// A 200 whose body is who the router said sent it, so a test reads the door's decision.
    async fn send_request(
        self: Arc<Self>,
        _caller: Option<Caller>,
        by: RequestBy,
        send: SendRequest,
    ) -> Result<RequestSent, Refusal> {
        Ok(RequestSent {
            url: send.url,
            status: Some(200),
            ms: 0,
            headers: Vec::new(),
            body: ipc::encode(&by).unwrap_or_default(),
            truncated: false,
            cookie_host: None,
            kept_header: None,
            error: None,
            note: None,
        })
    }

    async fn list_request_hosts(&self) -> Result<RequestHosts, Refusal> {
        Ok(RequestHosts::default())
    }

    async fn set_request_cookie(&self, set: SetRequestCookie) -> Result<RequestHosts, Refusal> {
        Ok(RequestHosts { hosts: vec![ipc::RequestHost { host: set.host, header: set.header }] })
    }

    async fn forget_request_cookie(
        &self,
        _forget: ForgetRequestCookie,
    ) -> Result<RequestHosts, Refusal> {
        Ok(RequestHosts::default())
    }

    async fn list_request_environments(
        &self,
        _caller: Option<Caller>,
        _session_id: Option<SessionId>,
    ) -> Result<RequestEnvironments, Refusal> {
        Ok(RequestEnvironments {
            saved: RequestEnvironmentsRead::Read { file: RequestEnvironmentsFile::default() },
            servers: Vec::new(),
        })
    }

    async fn save_request_environments(
        &self,
        file: RequestEnvironmentsFile,
    ) -> Result<RequestEnvironmentsRead, Refusal> {
        Ok(RequestEnvironmentsRead::Read { file })
    }
}
