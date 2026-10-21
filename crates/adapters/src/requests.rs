//! Requests over HTTP: [`HttpRequests`] sends one request a person or an agent names, and reads
//! back a bounded answer.
//!
//! **A client per request**, made inside the call, since `reqwest`'s blocking client panics if it
//! is made or dropped on an async worker and a caller holds this on a blocking thread
//! (`crate::store`'s reason). **No redirect is followed**: a secret filled in goes to the host named and
//! nowhere else, and the 3xx is the answer a person reads.

use std::io::Read;
use std::time::Duration;

use adapter_traits::{Incoming, Outgoing, RequestFailure, RequestSender};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;

/// Sends a request and gives up after `timeout`, connecting included.
pub struct HttpRequests {
    timeout: Duration,
}

impl HttpRequests {
    pub fn within(timeout: Duration) -> HttpRequests {
        HttpRequests { timeout }
    }
}

/// `why` and every cause under it, without the address, so a resolver's or a socket's own words
/// reach the person. **Never a header**: no `reqwest` error carries one.
fn failure(why: reqwest::Error) -> RequestFailure {
    if why.is_timeout() {
        return RequestFailure::TimedOut;
    }
    let why = why.without_url();
    let mut said = why.to_string();
    let mut cause = std::error::Error::source(&why);
    while let Some(under) = cause {
        let more = under.to_string();
        if !said.contains(&more) {
            said.push_str(": ");
            said.push_str(&more);
        }
        cause = under.source();
    }
    RequestFailure::Unreachable(said)
}

impl RequestSender for HttpRequests {
    fn send(&self, request: Outgoing, most: usize) -> Result<Incoming, RequestFailure> {
        let client = Client::builder()
            .timeout(self.timeout)
            .connect_timeout(self.timeout)
            .redirect(Policy::none())
            .build()
            .map_err(failure)?;
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|why| RequestFailure::Unreachable(why.to_string()))?;
        let mut sending = client.request(method, request.url.as_str());
        for (name, value) in &request.headers {
            sending = sending.header(name.as_str(), value.as_str());
        }
        if let Some(body) = request.body {
            sending = sending.body(body);
        }
        let response = sending.send().map_err(failure)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (name.as_str().to_string(), String::from_utf8_lossy(value.as_bytes()).into_owned())
            })
            .collect();
        // One byte past the bound says whether there was more.
        let mut body = Vec::new();
        response
            .take(most as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|why| match why.kind() {
                std::io::ErrorKind::TimedOut => RequestFailure::TimedOut,
                _ => RequestFailure::Unreachable(format!("the answer stopped arriving: {why}")),
            })?;
        let truncated = body.len() > most;
        body.truncate(most);
        Ok(Incoming { status, headers, body, truncated })
    }
}
