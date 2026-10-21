//! Requests: an HTTP request sent to an address a person or an agent names, and the secret
//! variables a request's `{{name}}` is filled from, kept one item each.
//!
//! **A secret's value travels as a [`Secret`]** from the store to Fleet, which fills it into the
//! request and takes it back out of everything answered.

use alloc::string::String;
use alloc::vec::Vec;

use crate::Secret;

/// One request, as Fleet hands it to the sender, every `{{name}}` already filled. **No `Debug`**:
/// a filled header may hold a secret's value.
pub struct Outgoing {
    /// Upper case, one of the methods Fleet allows.
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
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

/// Why nothing answered. **Never names a header**; an address it names is hidden by the caller.
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
    /// a 3xx is the answer, so a secret goes to the host named and nowhere else.
    fn send(&self, request: Outgoing, most: usize) -> Result<Incoming, RequestFailure>;
}

/// Which secret variable: the repository's key (`owner--name`), the environment's name and the
/// variable's. **Names only**, none of them secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretVariable {
    pub repository: String,
    pub environment: String,
    pub name: String,
}

/// Why the secret store did not do what was asked. **Never carries a value.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecretError {
    /// A value the store will not take.
    Refused(String),
    /// The store would not read or write.
    Unkept(String),
}

impl SecretError {
    pub fn said(&self) -> String {
        match self {
            SecretError::Refused(why) | SecretError::Unkept(why) => why.clone(),
        }
    }
}

/// The values of secret variables, one per [`SecretVariable`].
pub trait RequestSecrets {
    /// The value kept for `variable`, or `None`.
    fn read(&self, variable: &SecretVariable) -> Result<Option<Secret<String>>, SecretError>;

    /// Whether a value is kept for `variable`, without reading it.
    fn is_kept(&self, variable: &SecretVariable) -> Result<bool, SecretError>;

    /// Keep `value` for `variable`, replacing whatever it kept.
    fn keep(&self, variable: &SecretVariable, value: Secret<String>) -> Result<(), SecretError>;

    /// Take `variable`'s value out. One holding none is not an error.
    fn forget(&self, variable: &SecretVariable) -> Result<(), SecretError>;
}
