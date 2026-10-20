//! Requests: an HTTP request sent to an address a person or an agent names, and the cookies the
//! owner gave for its hosts.
//!
//! **The cookie travels as a [`Secret`]**, beside the headers and never among them, so the only
//! code that renders it is the sender writing the `Cookie` header and the store writing it down.

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
    /// Sent as the `Cookie` header where present. Fleet leaves it out where `headers` has one.
    pub cookie: Option<Secret<String>>,
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

/// Why nothing answered. **Never names a header**, so it cannot carry the cookie.
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
    /// a 3xx is the answer, so a cookie goes to the host named and nowhere else.
    fn send(&self, request: Outgoing, most: usize) -> Result<Incoming, RequestFailure>;
}

/// Why the cookie store did not do what was asked. **Never carries a value.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CookieError {
    /// A host or a cookie the store will not take.
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

/// The cookies the owner gave, one per host. A host is lower case, with no scheme, port or path.
pub trait RequestCookies {
    /// The cookie held for `host`, or `None`.
    fn cookie(&self, host: &str) -> Result<Option<Secret<String>>, CookieError>;

    /// Hold `cookie` for `host`, replacing one it held.
    fn keep(&self, host: &str, cookie: Secret<String>) -> Result<(), CookieError>;

    /// Take `host`'s cookie out. A host holding none is not an error.
    fn forget(&self, host: &str) -> Result<(), CookieError>;

    /// Every host holding one, sorted. **Names only.**
    fn hosts(&self) -> Result<Vec<String>, CookieError>;
}
