//! Requests: an HTTP request sent with the owner's cookie for its host, the cookies' hosts, and
//! the named base addresses a request's `{{base}}` can be.
//!
//! **Its own surface**, for [`Sessions`](super::Sessions)' reason: whole on its own.

use std::future::Future;

use ipc::{
    ForgetRequestCookie, RequestBy, RequestEnvironments, RequestEnvironmentsFile,
    RequestEnvironmentsRead, RequestHosts, RequestSent, SendRequest, SessionId, SetRequestCookie,
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

    /// `list_request_hosts` — the hosts holding a cookie, by name.
    fn list_request_hosts(&self) -> impl Future<Output = Result<RequestHosts, Refusal>> + Send;

    /// `set_request_cookie` — keep a host's cookie, replacing one it held. Answers every host.
    fn set_request_cookie(
        &self,
        set: SetRequestCookie,
    ) -> impl Future<Output = Result<RequestHosts, Refusal>> + Send;

    /// `forget_request_cookie` — take a host's cookie out. Answers every host left.
    fn forget_request_cookie(
        &self,
        forget: ForgetRequestCookie,
    ) -> impl Future<Output = Result<RequestHosts, Refusal>> + Send;

    /// `list_request_environments` — the saved ones, and the servers of the Job `caller` or
    /// `session_id` places.
    fn list_request_environments(
        &self,
        caller: Option<crate::Caller>,
        session_id: Option<SessionId>,
    ) -> impl Future<Output = Result<RequestEnvironments, Refusal>> + Send;

    /// `save_request_environments` — write the file whole, and answer it read again.
    fn save_request_environments(
        &self,
        file: RequestEnvironmentsFile,
    ) -> impl Future<Output = Result<RequestEnvironmentsRead, Refusal>> + Send;
}
