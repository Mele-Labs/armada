//! `send_requests`: one template sent once for each body file and each value, so a Job that cannot
//! see a secret sends hundreds of requests in a single call.
//! `docs/concepts/requests.md`, *Many at once*.
//!
//! **The answer is one short line a request and never a body**: each answer is saved whole, secrets
//! as `{{name}}`, in a file under the Job's scratch folder, and a line names that file.

use serde::{Deserialize, Serialize};

use crate::requests::RequestHeader;
use crate::sessions::SessionId;

/// What every request of a batch shares. `{{name}}` is filled from the environment, as in
/// `send_request`, after `vary`'s name is replaced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestTemplate {
    /// `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD` or `OPTIONS`, in any case.
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<RequestHeader>,
    /// A collection's id or name, whose headers go under the template's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
}

/// A name and the values it takes: `{{name}}` in the template and in each body is replaced by one
/// value for each request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestVary {
    pub name: String,
    pub values: Vec<String>,
}

/// `send_requests`. `POST /requests/send_many`. One request for each of `body_files` and each of
/// `vary`'s values, so twenty files and twenty values are four hundred requests.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendRequests {
    /// The Job the requests are sent from, read only where the connection places none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// A root `list_repositories` names, whose collections and environments are read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// A saved environment's name, or a server's: `server:<name>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    pub template: RequestTemplate,
    /// Absolute paths under the Job's scratch folder or slot, each read as one request's body.
    /// Absent or empty, the requests have no body.
    #[serde(default)]
    pub body_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vary: Option<RequestVary>,
    /// Most requests in flight at once. Absent, 5; over 10, 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<u32>,
    /// No new request is sent after this many answers in a row that are a 4xx, a 5xx or an error.
    /// Absent, 5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_after: Option<u32>,
}

/// One request of a batch, as answered. **No body**: `saved` names the file it is in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestLine {
    /// `<file stem>-<value>`, and the saved file's name.
    pub name: String,
    /// Absent where nothing answered or the request was not sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    pub ms: u64,
    /// The saved body's file name inside `folder`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved: Option<String>,
    /// Why nothing answered, or why the request was refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Not sent, since the batch stopped first.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
}

/// `send_requests`' answer, 200 whatever the requests answered. **A secret is `{{name}}` in every
/// field.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestsSent {
    /// Where each line's `saved` file is: a folder under the Job's scratch folder.
    pub folder: String,
    pub lines: Vec<RequestLine>,
    /// Why the batch stopped early, and what was cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
