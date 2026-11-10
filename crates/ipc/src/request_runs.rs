//! A saved request's runs: each send of it, by the owner or a Job's agent, kept after the Job
//! ends. `docs/practices/protocol.md`, *Requests*.
//!
//! **A secret is `{{name}}` in every field**, as in [`RequestSent`](crate::RequestSent): Fleet hides
//! each value before it keeps a run, so no read here holds one.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::requests::{RequestBy, RequestHeader};
use crate::sessions::SessionId;

/// Which runs a list keeps by how they ended: a status class, or nothing answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestRunStatus {
    #[serde(rename = "2xx")]
    Success,
    #[serde(rename = "3xx")]
    Redirect,
    #[serde(rename = "4xx")]
    ClientError,
    #[serde(rename = "5xx")]
    ServerError,
    /// Nothing answered: refused, timed out, a name that did not resolve.
    #[serde(rename = "failed")]
    Failed,
}

impl RequestRunStatus {
    pub fn as_wire(self) -> &'static str {
        match self {
            RequestRunStatus::Success => "2xx",
            RequestRunStatus::Redirect => "3xx",
            RequestRunStatus::ClientError => "4xx",
            RequestRunStatus::ServerError => "5xx",
            RequestRunStatus::Failed => "failed",
        }
    }

    /// A spelling read back, for a caller holding a string such as a query parameter.
    pub fn from_wire(spelling: &str) -> Option<RequestRunStatus> {
        [
            RequestRunStatus::Success,
            RequestRunStatus::Redirect,
            RequestRunStatus::ClientError,
            RequestRunStatus::ServerError,
            RequestRunStatus::Failed,
        ]
        .into_iter()
        .find(|status| status.as_wire() == spelling)
    }
}

/// The Job an agent's run was sent from. The title is the Job's now, or its title when it was sent
/// where the ledger no longer names the Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestRunJob {
    pub id: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// One run as a list draws it: when, by whom, how it ended, and what was sent in a line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestRunSummary {
    /// Opaque, and the argument of `get_request_run`.
    pub id: String,
    pub at: Instant,
    pub by: RequestBy,
    /// The Job that sent it, where one did and Fleet knows it. An owner's send names the Job open
    /// in Bridge, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<RequestRunJob>,
    pub method: String,
    /// The address as sent, filled.
    pub url: String,
    /// Absent where nothing answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    pub ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The start of the request's body on one line. Empty where it sent no body.
    pub summary: String,
}

/// `list_request_runs`' answer: one page of a request's runs, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestRuns {
    pub runs: Vec<RequestRunSummary>,
    /// Where the next page begins: pass it as `before`. Absent on the last page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

/// The request exactly as it went, every `{{name}}` filled but a secret's, which stays `{{name}}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestRunSent {
    pub method: String,
    pub url: String,
    pub headers: Vec<RequestHeader>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// What came back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestRunAnswer {
    /// Absent where nothing answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    pub ms: u64,
    /// The answer's headers. **No `set-cookie` through the agent's door.**
    pub headers: Vec<RequestHeader>,
    /// The body as text, its start where it was cut: at 256 KiB, and at 32 KiB through the
    /// agent's door.
    pub body: String,
    /// Whether `body` is less than the answer carried.
    pub truncated: bool,
    /// What the answer weighed as it came, in bytes.
    pub bytes: u64,
    /// The `content-type` header's value, where the answer carried one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Why nothing answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// `get_request_run`'s answer: one run in full.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestRun {
    pub id: String,
    pub at: Instant,
    pub by: RequestBy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<RequestRunJob>,
    /// The saved request's collection and its own id, as `get_requests` lists them.
    pub collection: String,
    pub request: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    pub sent: RequestRunSent,
    pub answer: RequestRunAnswer,
}
