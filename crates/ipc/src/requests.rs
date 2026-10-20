//! Requests: an HTTP request the owner or a Job's agent sends to an address, with the one header
//! the owner kept for that host (a `Cookie`, an `Authorization`, any name) attached by Fleet.
//! `docs/practices/protocol.md`, *Requests*.
//!
//! **No type here carries a kept value but [`SetRequestCookie`]**, which only Bridge sends.
//! Every answer names the host whose header went and which header, never what it held.
//!
//! **The environments file is the wire**, as `flight_controls`' is: `~/.armada/requests.toml` is
//! read into [`RequestEnvironmentsFile`] and a save sends the same type whole.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;

/// The environments file's version this Fleet reads and writes.
pub const REQUEST_ENVIRONMENTS_VERSION: u32 = 1;

/// What starts the name of an environment Fleet offers for a server the Job is running. A saved
/// environment's name may not start with it, so the two never collide.
pub const SERVER_ENVIRONMENT_PREFIX: &str = "server:";

/// What a request's address names to be replaced by an environment's base.
pub const BASE_PLACEHOLDER: &str = "{{base}}";

fn current() -> u32 {
    REQUEST_ENVIRONMENTS_VERSION
}

/// One header, as sent or as answered. A list rather than a map: a name may come more than once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestHeader {
    pub name: String,
    pub value: String,
}

/// `send_request`. `POST /requests/send`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendRequest {
    /// The Job the request is sent from. Read only where the connection places none, as
    /// `waiting_for`'s is. Absent from Bridge: nothing is written in a thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD` or `OPTIONS`, in any case.
    pub method: String,
    /// An `http` or `https` address, which may start with `{{base}}`.
    pub url: String,
    /// The environment whose base replaces `{{base}}`: a saved one's name, or a server's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(default)]
    pub headers: Vec<RequestHeader>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// `send_request`'s answer, 200 whether or not anything answered: a request that could not be
/// made carries `error` and no `status`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSent {
    /// The address as sent, `{{base}}` replaced.
    pub url: String,
    /// Absent where nothing answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// From the request leaving to the answer's body read, or to the failure.
    pub ms: u64,
    /// The answer's headers. **No `set-cookie` through the agent's door.**
    #[serde(default)]
    pub headers: Vec<RequestHeader>,
    /// The answer's body as text, its start where it was cut.
    #[serde(default)]
    pub body: String,
    /// Whether `body` was cut to fit.
    #[serde(default)]
    pub truncated: bool,
    /// The host whose kept header went with the request. Absent where none did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cookie_host: Option<String>,
    /// Which header went, as the owner named it: `Cookie`, `Authorization`. Present with
    /// `cookie_host`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept_header: Option<String>,
    /// Why nothing answered: refused, timed out, a name that did not resolve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Fleet's word on the answer: a 401 or 403 from a host holding no kept header says the owner
    /// can add one in Requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The header a host keeps where none is named.
fn cookie() -> String {
    String::from("Cookie")
}

/// `set_request_cookie`: the header the owner gives for a host, kept in the Keychain.
/// `POST /requests/cookies`. **The one type that carries a kept value**, Bridge to Fleet.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetRequestCookie {
    /// A host, with no scheme, port or path: `api.example.test`.
    pub host: String,
    /// The header's name: `Cookie` where absent, `Authorization`, or any other. Refused unless it
    /// is a valid header name.
    #[serde(default = "cookie")]
    pub header: String,
    /// The whole header's value, as a request would carry it: a `Cookie` as a browser sends it,
    /// an `Authorization` as `Bearer <token>`.
    pub value: String,
}

/// Written by hand so a log line or a panic cannot print the value.
impl std::fmt::Debug for SetRequestCookie {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SetRequestCookie")
            .field("host", &self.host)
            .field("header", &self.header)
            .field("value", &"[hidden]")
            .finish()
    }
}

/// `forget_request_cookie`. `POST /requests/cookies/forget`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetRequestCookie {
    pub host: String,
}

/// A host holding a kept header, and which one. Never the value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestHost {
    pub host: String,
    pub header: String,
}

/// The hosts holding a kept header, by host and header name only. `list_request_hosts`' answer,
/// and the two acts' once they are done. Sorted by host.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestHosts {
    pub hosts: Vec<RequestHost>,
}

/// One named base address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvironment {
    pub name: String,
    /// An `http` or `https` address, with no trailing `/`: `{{base}}/v1` reads as written.
    pub base: String,
}

/// `~/.armada/requests.toml` whole, and the body of `save_request_environments`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvironmentsFile {
    #[serde(default = "current")]
    pub version: u32,
    #[serde(default)]
    pub environment: Vec<RequestEnvironment>,
}

impl Default for RequestEnvironmentsFile {
    fn default() -> RequestEnvironmentsFile {
        RequestEnvironmentsFile { version: REQUEST_ENVIRONMENTS_VERSION, environment: Vec::new() }
    }
}

/// The file as read. **Tagged**, so a file that will not read is never drawn as an empty one.
/// `save_request_environments`' answer once it has read the file again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "what", rename_all = "snake_case")]
pub enum RequestEnvironmentsRead {
    Read { file: RequestEnvironmentsFile },
    NotRead { why: String },
}

/// `list_request_environments`: the saved ones, and one for each server the Job is running.
/// `GET /requests/environments`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestEnvironments {
    pub saved: RequestEnvironmentsRead,
    /// Each named `server:<name>`, at the first port it serves on. Empty with no Job named.
    #[serde(default)]
    pub servers: Vec<RequestEnvironment>,
}

/// Who sent a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestBy {
    /// The owner, from Requests in Bridge.
    Owner,
    /// The Job's agent, through its tool.
    Agent,
}
