//! Requests: an HTTP request the owner or a Job's agent sends, with the cookie the owner gave for
//! its host attached here. `docs/practices/protocol.md`, *Requests*.
//!
//! [`sent`] is the whole of a send over the two ports, a function of values, so the claim that a
//! cookie goes out and is read nowhere is proved without a socket or a Keychain
//! (`crates/acceptance/tests/requests.rs`). Fleet runs it on a blocking thread and writes what it
//! answers into the Job's thread (`serving`).
//!
//! **The cookie's value is taken out of everything that leaves here**: the answer's headers and
//! body, the error, the row and the words a Job is told. A server that echoes what it was sent
//! would otherwise hand it to the agent. What is taken out is the whole value, each `name=value`
//! pair in it, and each pair's value of six characters or more; a shorter value is left, since
//! replacing every `dark` in a body would wreck the body and hide nothing worth hiding. An encoded
//! echo (percent, base64) is not caught.

mod environments;
mod serving;

use std::time::Duration;

use adapter_traits::{CookieError, Incoming, Outgoing, RequestCookies, RequestSender, Secret};
use ipc::{
    Instant, RequestBy, RequestEnvironment, RequestHeader, RequestSent, SendRequest, SessionRow,
    BASE_PLACEHOLDER,
};

pub use environments::{check as check_environments, parse as parse_environments, FILE as ENVIRONMENTS_FILE};

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

/// The file in Fleet's machine directory listing the hosts that hold a cookie. Names only.
pub const HOSTS_FILE: &str = "request-hosts";

/// The methods a request may use.
const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
/// What a cookie's value is replaced by.
const HIDDEN: &str = "[cookie]";
/// The shortest pair value taken out on its own.
const SHORTEST_HIDDEN: usize = 6;

/// What sends a request and what holds the cookies. **A seam so a test plants both**: the shipped
/// pair opens sockets and asks the Keychain.
#[derive(Clone)]
pub struct Requesting {
    pub sender: std::sync::Arc<dyn RequestSender + Send + Sync>,
    pub cookies: std::sync::Arc<dyn RequestCookies + Send + Sync>,
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

/// Why a request was not sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsent {
    /// The request itself: a method, an address, an environment, a header, a body. A 422.
    Refused(String),
    /// The cookie store would not read. A 500.
    Unkept(String),
}

/// A request sent, and what is kept of it. **Holds no cookie's value**.
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
            cookie_host: self.answer.cookie_host.clone(),
        }
    }

    /// What the Job's agent is told of an owner's send, in the shape a run's end is told.
    pub fn told(&self) -> String {
        let mut text = format!("Owner sent `{} {}` from Requests.", self.method, self.answer.url);
        let went = match &self.answer.cookie_host {
            Some(host) => format!(" The cookie for {host} went with it."),
            None => String::new(),
        };
        match (self.answer.status, &self.answer.error) {
            (Some(status), _) => {
                text.push_str(&format!(" {status} in {} ms.{went}", self.answer.ms));
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
                " Nothing answered after {} ms: {}{went}",
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

/// `url` with `{{base}}` replaced by the named environment's base.
pub fn resolved(
    url: &str,
    environment: Option<&str>,
    environments: &[RequestEnvironment],
) -> Result<String, String> {
    let url = url.trim();
    let named = environment.map(str::trim).filter(|name| !name.is_empty());
    let base = match named {
        Some(name) => Some(
            environments
                .iter()
                .find(|one| one.name == name)
                .map(|one| one.base.trim_end_matches('/'))
                .ok_or_else(|| format!("there is no environment named {name:?}"))?,
        ),
        None => None,
    };
    match (url.contains(BASE_PLACEHOLDER), base) {
        (true, Some(base)) => Ok(url.replace(BASE_PLACEHOLDER, base)),
        (true, None) => Err(format!("{url} names {BASE_PLACEHOLDER} and no environment was given")),
        (false, _) => Ok(url.to_string()),
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

/// `text` with every piece of `cookie` a reader could recognise it by replaced.
pub fn hidden(text: &str, cookie: &str) -> String {
    let cookie = cookie.trim();
    if cookie.is_empty() {
        return text.to_string();
    }
    let mut pieces = vec![cookie.to_string()];
    for pair in cookie.split(';').map(str::trim).filter(|pair| !pair.is_empty()) {
        pieces.push(pair.to_string());
        if let Some((_, value)) = pair.split_once('=') {
            let value = value.trim().trim_matches('"');
            if value.len() >= SHORTEST_HIDDEN {
                pieces.push(value.to_string());
            }
        }
    }
    pieces.sort_by_key(|piece| std::cmp::Reverse(piece.len()));
    pieces.dedup();
    pieces.iter().fold(text.to_string(), |text, piece| text.replace(piece.as_str(), HIDDEN))
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

fn header_fits(header: &RequestHeader) -> Result<(), String> {
    let token = |c: char| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c);
    if header.name.is_empty() || !header.name.chars().all(token) {
        return Err(format!("{:?} is not a header name", header.name));
    }
    if header.value.contains(['\r', '\n']) {
        return Err(format!("the {} header's value is more than one line", header.name));
    }
    Ok(())
}

fn is_cookie(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case("cookie")
}

/// Send `send` through `sender`, with the cookie `cookies` holds for its host unless it carries its
/// own. **A request nothing answered is sent**: its answer carries the error.
pub fn sent(
    sender: &dyn RequestSender,
    cookies: &dyn RequestCookies,
    by: RequestBy,
    send: &SendRequest,
    environments: &[RequestEnvironment],
    bounds: &Bounds,
) -> Result<Sent, Unsent> {
    let method = send.method.trim().to_ascii_uppercase();
    if !METHODS.contains(&method.as_str()) {
        return Err(Unsent::Refused(format!("{:?} is not one of {}", send.method, METHODS.join(", "))));
    }
    let url = resolved(&send.url, send.environment.as_deref(), environments).map_err(Unsent::Refused)?;
    let host = host_of(&url).map_err(Unsent::Refused)?;
    for header in &send.headers {
        header_fits(header).map_err(Unsent::Refused)?;
    }
    if send.body.as_ref().is_some_and(|body| body.len() > bounds.request_body) {
        return Err(Unsent::Refused(format!("a request's body is at most {} bytes", bounds.request_body)));
    }
    let carries_its_own = send.headers.iter().any(|header| is_cookie(&header.name));
    let held = match carries_its_own {
        true => None,
        false => match cookies.cookie(&host) {
            Ok(held) => held.map(|cookie| cookie.expose().clone()),
            // A host the store cannot name (an IPv6 literal) holds nothing.
            Err(CookieError::Refused(_)) => None,
            Err(CookieError::Unkept(why)) => return Err(Unsent::Unkept(why)),
        },
    };
    let value = held.clone().unwrap_or_default();
    let most = match by {
        RequestBy::Owner => bounds.answer,
        RequestBy::Agent => bounds.agent_answer,
    };
    let outgoing = Outgoing {
        method: method.clone(),
        url: url.clone(),
        headers: send.headers.iter().map(|one| (one.name.trim().to_string(), one.value.clone())).collect(),
        body: send.body.clone(),
        cookie: held.map(Secret::new),
    };
    let started = std::time::Instant::now();
    // Past the bound by the value's length, so a value straddling the cut is still whole to hide.
    let came = sender.send(outgoing, bounds.answer.max(most) + value.len());
    let ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let cookie_host = (!value.is_empty()).then(|| host.clone());
    let mut answer = RequestSent {
        url: hidden(&url, &value),
        status: None,
        ms,
        headers: Vec::new(),
        body: String::new(),
        truncated: false,
        cookie_host,
        error: None,
        note: None,
    };
    let (mut response, mut response_cut) = (String::new(), false);
    match came {
        Ok(Incoming { status, headers, body, truncated }) => {
            let text = hidden(&String::from_utf8_lossy(&body), &value);
            (response, response_cut) = cut(text.clone(), bounds.row);
            response_cut |= truncated;
            let (body, cut_here) = cut(text, most);
            answer.status = Some(status);
            answer.body = body;
            answer.truncated = truncated || cut_here;
            answer.headers = headers
                .into_iter()
                .filter(|(name, _)| by == RequestBy::Owner || !name.eq_ignore_ascii_case("set-cookie"))
                .map(|(name, value_of)| RequestHeader { name, value: hidden(&value_of, &value) })
                .collect();
            if matches!(status, 401 | 403) && value.is_empty() && !carries_its_own {
                answer.note = Some(format!(
                    "{host} holds no cookie. The owner can give one for it in Requests."
                ));
            }
        }
        Err(failure) => answer.error = Some(hidden(&failure.said(), &value)),
    }
    Ok(Sent {
        by,
        method,
        environment: send.environment.clone().filter(|name| !name.trim().is_empty()),
        headers: send
            .headers
            .iter()
            .filter(|header| !is_cookie(&header.name))
            .map(|header| RequestHeader { name: header.name.clone(), value: hidden(&header.value, &value) })
            .collect(),
        body: send.body.as_ref().map(|body| hidden(body, &value)),
        answer,
        response,
        response_cut,
    })
}
