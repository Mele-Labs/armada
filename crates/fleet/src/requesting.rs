//! Requests: an HTTP request the owner or a Job's agent sends, every `{{name}}` in it filled from
//! an environment's variables, and the files its collections and environments are kept in.
//! `docs/practices/protocol.md`, *Requests*.
//!
//! [`sent`] is the whole of a send over the two ports, a function of values, so the claim that a
//! secret goes out and is read nowhere is proved without a socket or a Keychain
//! (`crates/acceptance/tests/requests.rs`). Fleet runs it on a blocking thread and writes what it
//! answers into the Job's thread (`serving`).
//!
//! **A secret's value is taken out of everything that leaves here**, put back as `{{name}}`: the
//! answer's address, headers and body, the error, the row and the words a Job is told. A server
//! that echoes what it was sent would otherwise hand it to the agent. A value shorter than six
//! characters is left, since replacing every `dark` in a body would wreck the body and hide
//! nothing worth hiding. An encoded echo (percent, base64) is not caught.

mod copying;
mod files;
mod filling;
mod kept;
mod saving;
mod serving;

use std::time::Duration;

use adapter_traits::{Incoming, Outgoing, RequestSecrets, RequestSender};
use ipc::{Instant, RequestBy, RequestHeader, RequestSent, SendRequest, SessionRow};

pub use copying::{copied, NotCopied};
pub use files::{
    check, check_collection, check_environment, environment_written, parse, written, SecretChange,
};
pub use filling::{hidden, Filling};
pub(crate) use kept::{read_file, IN_REPOSITORY};

/// `settings.request-timeout`: how long a request may take, connecting included.
pub const TIMEOUT: Duration = Duration::from_secs(30);
/// `settings.request-answer-cap`: the most of an answer's body the owner is answered.
pub const ANSWER_CAP: usize = 256 * 1024;
/// `settings.request-answer-cap`, through the agent's door: under the door's own 64 KiB, so the
/// JSON around the body is never cut.
pub const AGENT_ANSWER_CAP: usize = 32 * 1024;
/// `settings.request-row-cap`: the most of an answer's body a thread's row and a Job's words keep.
pub const ROW_CAP: usize = 16 * 1024;
/// The most a request's own body may be. Its row keeps it whole, so it is bounded where it is sent.
pub const REQUEST_BODY_CAP: usize = 64 * 1024;

/// The methods a request may use.
pub(crate) const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// What sends a request and what holds the secrets. **A seam so a test plants both**: the shipped
/// pair opens sockets and asks the Keychain.
#[derive(Clone)]
pub struct Requesting {
    pub sender: std::sync::Arc<dyn RequestSender + Send + Sync>,
    pub secrets: std::sync::Arc<dyn RequestSecrets + Send + Sync>,
}

/// How much of what is sent and answered is kept, in bytes.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub answer: usize,
    pub agent_answer: usize,
    pub row: usize,
    pub request_body: usize,
}

impl Bounds {
    pub fn shipped() -> Bounds {
        Bounds { answer: ANSWER_CAP, agent_answer: AGENT_ANSWER_CAP, row: ROW_CAP, request_body: REQUEST_BODY_CAP }
    }
}

/// Why a request was not sent. **Neither carries a secret's value.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsent {
    /// The request itself: a method, an address, a saved request, an environment, a header, a
    /// body. A 422.
    Refused(String),
    /// The Keychain would not read a secret. A 500.
    Unkept(String),
}

/// A request sent, and what is kept of it. **Holds no secret's value**.
#[derive(Clone, Debug)]
pub struct Sent {
    pub by: RequestBy,
    pub answer: RequestSent,
    method: String,
    environment: Option<String>,
    headers: Vec<RequestHeader>,
    body: Option<String>,
    response: String,
    response_cut: bool,
}

impl Sent {
    /// The thread's row, under `id` at `at`.
    pub fn row(&self, id: String, at: Instant) -> SessionRow {
        SessionRow::Requested {
            id,
            at,
            by: self.by,
            method: self.method.clone(),
            url: self.answer.url.clone(),
            environment: self.environment.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            status: self.answer.status,
            ms: self.answer.ms,
            error: self.answer.error.clone(),
            response: self.response.clone(),
            truncated: self.response_cut,
        }
    }

    /// What the Job's agent is told of an owner's send, in the shape a run's end is told.
    pub fn told(&self) -> String {
        let mut text = format!("Owner sent `{} {}` from Requests.", self.method, self.answer.url);
        match (self.answer.status, &self.answer.error) {
            (Some(status), _) => {
                text.push_str(&format!(" {status} in {} ms.", self.answer.ms));
                if self.response.is_empty() {
                    text.push_str(" No body.");
                } else {
                    text.push_str(match self.response_cut {
                        true => " Body, cut:\n",
                        false => " Body:\n",
                    });
                    text.push_str(&crate::session_running::fenced(&self.response));
                }
            }
            (None, error) => text.push_str(&format!(
                " Nothing answered after {} ms: {}",
                self.answer.ms,
                error.as_deref().unwrap_or("no reason was given.")
            )),
        }
        if let Some(note) = &self.answer.note {
            text.push('\n');
            text.push_str(note);
        }
        text
    }
}

/// The host an `http` or `https` address names, in lower case, with no port.
pub fn host_of(url: &str) -> Result<String, String> {
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .ok_or_else(|| format!("{url} is not an http or https address"))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let authority = authority.rsplit_once('@').map_or(authority, |(_, after)| after);
    let host = match authority.strip_prefix('[') {
        Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
        None => authority.split(':').next().unwrap_or_default(),
    };
    match host.is_empty() {
        true => Err(format!("{url} names no host")),
        false => Ok(host.to_string()),
    }
}

/// `text` cut to at most `most` bytes on a character's edge, and whether it was.
fn cut(text: String, most: usize) -> (String, bool) {
    if text.len() <= most {
        return (text, false);
    }
    let mut at = most;
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    (text[..at].to_string(), true)
}

/// Whether `name` is a header name a request line may carry.
pub(crate) fn header_named(name: &str) -> bool {
    let token = |c: char| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c);
    !name.is_empty() && name.chars().all(token)
}

fn header_fits(name: &str, value: &str) -> Result<(), String> {
    if !header_named(name) {
        return Err(format!("{name:?} is not a header name"));
    }
    if value.contains(['\r', '\n']) {
        return Err(format!("the {name} header's value is more than one line"));
    }
    Ok(())
}

pub(crate) fn is_named(name: &str, wanted: &str) -> bool {
    name.trim().eq_ignore_ascii_case(wanted.trim())
}

/// Send `send` through `sender`, every `{{name}}` filled from `filling` and `secrets`. **A request
/// nothing answered is sent**: its answer carries the error.
pub fn sent(
    sender: &dyn RequestSender,
    secrets: &dyn RequestSecrets,
    by: RequestBy,
    send: &SendRequest,
    filling: &Filling,
    bounds: &Bounds,
) -> Result<Sent, Unsent> {
    let asked = filling.asked(send).map_err(Unsent::Refused)?;
    let method = asked.method.trim().to_ascii_uppercase();
    if !METHODS.contains(&method.as_str()) {
        return Err(Unsent::Refused(format!("{:?} is not one of {}", asked.method, METHODS.join(", "))));
    }
    let environment = send.environment.as_deref().map(str::trim).filter(|name| !name.is_empty());
    let values = filling.values(environment, &asked.texts(), secrets)?;
    let refused = |why: String| Unsent::Refused(values.hidden(&why));
    let url = values.filled(asked.url.trim());
    host_of(&url).map_err(refused)?;
    let headers: Vec<(String, String)> =
        asked.headers.iter().map(|one| (one.name.trim().to_string(), values.filled(&one.value))).collect();
    for (name, value) in &headers {
        header_fits(name, value).map_err(refused)?;
    }
    let body = asked.body.as_deref().map(|body| values.filled(body));
    if body.as_ref().is_some_and(|body| body.len() > bounds.request_body) {
        return Err(Unsent::Refused(format!("a request's body is at most {} bytes", bounds.request_body)));
    }
    let unfilled = values.unfilled(&asked.texts());
    let most = match by {
        RequestBy::Owner => bounds.answer,
        RequestBy::Agent => bounds.agent_answer,
    };
    let row_headers = headers
        .iter()
        .map(|(name, value)| RequestHeader { name: name.clone(), value: values.hidden(value) })
        .collect();
    let row_body = body.as_deref().map(|body| values.hidden(body));
    let outgoing = Outgoing { method: method.clone(), url: url.clone(), headers, body };
    let started = std::time::Instant::now();
    // Past the bound by the longest value, so a value straddling the cut is still whole to hide.
    let came = sender.send(outgoing, bounds.answer.max(most) + values.longest());
    let ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let note = (!unfilled.is_empty())
        .then(|| unfilled.iter().map(|name| format!("{{{{{name}}}}} was not filled.")).collect::<Vec<_>>().join(" "));
    let mut answer = RequestSent {
        url: values.hidden(&url),
        status: None,
        ms,
        headers: Vec::new(),
        body: String::new(),
        truncated: false,
        error: None,
        note,
    };
    let (mut response, mut response_cut) = (String::new(), false);
    match came {
        Ok(Incoming { status, headers, body, truncated }) => {
            let text = values.hidden(&String::from_utf8_lossy(&body));
            (response, response_cut) = cut(text.clone(), bounds.row);
            response_cut |= truncated;
            let (body, cut_here) = cut(text, most);
            answer.status = Some(status);
            answer.body = body;
            answer.truncated = truncated || cut_here;
            answer.headers = headers
                .into_iter()
                .filter(|(name, _)| by == RequestBy::Owner || !name.eq_ignore_ascii_case("set-cookie"))
                .map(|(name, value)| RequestHeader { name, value: values.hidden(&value) })
                .collect();
        }
        Err(failure) => answer.error = Some(values.hidden(&failure.said())),
    }
    Ok(Sent {
        by,
        method,
        environment: environment.map(str::to_string),
        headers: row_headers,
        body: row_body,
        answer,
        response,
        response_cut,
    })
}
