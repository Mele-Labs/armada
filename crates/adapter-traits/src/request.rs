//! Requests: an HTTP request sent to an address a person or an agent names, and the one header
//! the owner kept for each of its hosts: a `Cookie` by default, or an `Authorization`, or any other.
//!
//! **The kept value travels as a [`Secret`]**, beside the headers and never among them, so the only
//! code that renders it is the sender writing the header and the store writing it down.

use alloc::string::String;
use alloc::vec::Vec;

use crate::Secret;

/// One request, as Fleet hands it to the sender.
pub struct Outgoing {
    /// Upper case, one of the methods Fleet allows.
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    /// Sent under its own name where present. Fleet leaves it out where `headers` has one of that
    /// name.
    pub kept: Option<KeptHeader>,
}

/// The header a host keeps where the owner names none.
pub const COOKIE_HEADER: &str = "Cookie";

/// The header the owner kept for a host: its name, which is no secret, and its value.
pub struct KeptHeader {
    /// `Cookie`, `Authorization`, or any other header name, as the owner typed it.
    pub header: String,
    pub value: Secret<String>,
}

/// A host holding a kept header, and which header. **Names only**, never the value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptHost {
    pub host: String,
    pub header: String,
}

/// What came back. `body` is at most what the caller asked for, and `truncated` says whether
/// more was there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incoming {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub truncated: bool,
}

/// Why nothing answered. **Never names a header**, so it cannot carry the kept value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestFailure {
    /// No answer inside the sender's timeout.
    TimedOut,
    /// Refused, a name that did not resolve, a broken connection, an address that will not parse.
    Unreachable(String),
}

impl RequestFailure {
    /// A sentence for a person.
    pub fn said(&self) -> String {
        match self {
            RequestFailure::TimedOut => String::from("It timed out."),
            RequestFailure::Unreachable(why) => why.clone(),
        }
    }
}

/// Sends one request. **A network round trip**, so a caller holds it on a blocking thread.
pub trait RequestSender {
    /// Send it, and read at most `most` bytes of the answer's body. Redirects are not followed:
    /// a 3xx is the answer, so a kept header goes to the host named and nowhere else.
    fn send(&self, request: Outgoing, most: usize) -> Result<Incoming, RequestFailure>;
}

/// Why the cookie store did not do what was asked. **Never carries a value.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CookieError {
    /// A host, a header name or a value the store will not take.
    Refused(String),
    /// The store would not read or write.
    Unkept(String),
}

impl CookieError {
    pub fn said(&self) -> String {
        match self {
            CookieError::Refused(why) | CookieError::Unkept(why) => why.clone(),
        }
    }
}

/// The headers the owner kept, one per host. A host is lower case, with no scheme, port or path.
/// Named for the cookie it first held, before it held any header.
pub trait RequestCookies {
    /// The header kept for `host`, or `None`.
    fn kept(&self, host: &str) -> Result<Option<KeptHeader>, CookieError>;

    /// Keep `value` under `header` for `host`, replacing whatever it kept.
    fn keep(&self, host: &str, header: &str, value: Secret<String>) -> Result<(), CookieError>;

    /// Take `host`'s header out. A host holding none is not an error.
    fn forget(&self, host: &str) -> Result<(), CookieError>;

    /// Every host holding one, sorted by host. **Names only.**
    fn hosts(&self) -> Result<Vec<KeptHost>, CookieError>;
}
