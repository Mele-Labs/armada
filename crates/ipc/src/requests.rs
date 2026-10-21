//! Requests: an HTTP request the owner or a Job's agent sends to an address, collections of
//! saved requests, and environments of variables a request's `{{name}}` is filled from.
//! `docs/practices/protocol.md`, *Requests*.
//!
//! **No answer carries a secret's value**: [`VariableRead`] has no field for one, and a
//! [`RequestsFile`] refuses one. A secret's value crosses the wire once, Bridge to Fleet, in
//! [`VariableWrite`], whose `Debug` never prints it. Fleet keeps it in the Keychain.
//!
//! **The file is the same at either place**: the person's own at the repository's place, and the
//! repository's committed `.armada/requests.toml`.

use serde::{Deserialize, Serialize};

use crate::run_memory::RunPlace;
use crate::sessions::SessionId;

/// The requests file's version this Fleet reads and writes.
pub const REQUESTS_VERSION: u32 = 1;

/// What starts the name of an environment Fleet offers for a server the Job is running. A saved
/// environment's name may not start with it, so the two never collide.
pub const SERVER_ENVIRONMENT_PREFIX: &str = "server:";

/// The one variable a server's environment holds: the address it serves on.
pub const SERVER_BASE_VARIABLE: &str = "base";

fn current() -> u32 {
    REQUESTS_VERSION
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

/// A request kept in a collection. `id` is unique within its collection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedRequest {
    pub id: String,
    pub name: String,
    pub method: String,
    /// May hold `{{name}}`s, filled from the environment a send names.
    pub url: String,
    #[serde(default)]
    pub headers: Vec<RequestHeader>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// A folder of saved requests. Its `headers` go with every request in it, under the request's
/// own: a name the request carries is the request's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestCollection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub headers: Vec<RequestHeader>,
    #[serde(default)]
    pub requests: Vec<SavedRequest>,
}

/// A variable as a file keeps it. **A secret is its name alone**: its value is in the Keychain,
/// so a file that would be committed or packaged never holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "VariableAsWritten", into = "VariableAsWritten")]
pub enum RequestVariable {
    Plain { name: String, value: String },
    Secret { name: String },
}

/// A variable as a file spells it: `{ name, value }`, or `{ name, secret = true }`.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VariableAsWritten {
    name: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    secret: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

impl TryFrom<VariableAsWritten> for RequestVariable {
    type Error = String;

    fn try_from(written: VariableAsWritten) -> Result<RequestVariable, String> {
        let name = written.name;
        match (written.secret, written.value) {
            (false, Some(value)) => Ok(RequestVariable::Plain { name, value }),
            (true, None) => Ok(RequestVariable::Secret { name }),
            // The words name the variable and never repeat what it held.
            (true, Some(_)) => Err(format!(
                "`{name}` is secret, so the file keeps its name and never its value; its value \
                 is kept in the Keychain"
            )),
            (false, None) => Err(format!(
                "`{name}` has no value: give it one, or mark it `secret = true`"
            )),
        }
    }
}

impl From<RequestVariable> for VariableAsWritten {
    fn from(variable: RequestVariable) -> VariableAsWritten {
        match variable {
            RequestVariable::Plain { name, value } => {
                VariableAsWritten { name, secret: false, value: Some(value) }
            }
            RequestVariable::Secret { name } => VariableAsWritten { name, secret: true, value: None },
        }
    }
}

/// A named set of variables, as a file keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvironment {
    pub name: String,
    #[serde(default)]
    pub variables: Vec<RequestVariable>,
}

/// The requests file whole, at either place. By hand it reads `[[collection]]`,
/// `[[collection.requests]]` and `[[environment]]`, one table each.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestsFile {
    #[serde(default = "current")]
    pub version: u32,
    #[serde(default, rename = "collection")]
    pub collections: Vec<RequestCollection>,
    #[serde(default, rename = "environment")]
    pub environments: Vec<RequestEnvironment>,
}

impl Default for RequestsFile {
    fn default() -> RequestsFile {
        RequestsFile { version: REQUESTS_VERSION, collections: Vec::new(), environments: Vec::new() }
    }
}

/// A variable as Fleet answers it: a plain one's value, or whether a secret's is kept. **There is
/// no field for a secret's value.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "VariableAsRead", into = "VariableAsRead")]
pub enum VariableRead {
    Plain { name: String, value: String },
    /// `set` says the Keychain holds a value for it.
    Secret { name: String, set: bool },
}

/// `{ name, secret: false, value }` or `{ name, secret: true, set }`: serde cannot tag on a bool.
#[derive(Clone, Serialize, Deserialize)]
struct VariableAsRead {
    name: String,
    secret: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    set: Option<bool>,
}

impl TryFrom<VariableAsRead> for VariableRead {
    type Error = String;

    fn try_from(read: VariableAsRead) -> Result<VariableRead, String> {
        let name = read.name;
        match (read.secret, read.value, read.set) {
            (false, Some(value), None) => Ok(VariableRead::Plain { name, value }),
            (true, None, Some(set)) => Ok(VariableRead::Secret { name, set }),
            (false, _, _) => Err(format!("`{name}` is plain, so it carries `value` and no `set`")),
            (true, _, _) => Err(format!("`{name}` is secret, so it carries `set` and no `value`")),
        }
    }
}

impl From<VariableRead> for VariableAsRead {
    fn from(variable: VariableRead) -> VariableAsRead {
        match variable {
            VariableRead::Plain { name, value } => {
                VariableAsRead { name, secret: false, value: Some(value), set: None }
            }
            VariableRead::Secret { name, set } => {
                VariableAsRead { name, secret: true, value: None, set: Some(set) }
            }
        }
    }
}

/// A named set of variables, as Fleet answers it. A server's is `server:<name>` and holds
/// [`SERVER_BASE_VARIABLE`] alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestEnvironmentRead {
    pub name: String,
    pub variables: Vec<VariableRead>,
}

/// Where a collection or environment is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestPlace {
    /// The person's own file for the repository, at the repository's place.
    Yours,
    /// The repository's committed `.armada/requests.toml`.
    Repository,
    /// A server the Job is running. Never written.
    Server,
}

/// A collection or environment as Requests lists it: where it is kept, and whether it is the
/// person's own hiding the repository's of the same id or name, which is then not listed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placed<T> {
    #[serde(flatten)]
    pub item: T,
    pub place: RequestPlace,
    pub hides: bool,
}

/// A file that would not read, and why. **Its items are not listed**, and a save into it is
/// refused, so a file that will not read is never drawn as empty and saved over.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestsNotRead {
    pub place: RequestPlace,
    pub why: String,
}

/// `get_requests`' answer, and each save's once it has read both files again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requests {
    /// A root `list_repositories` names.
    pub repository: String,
    /// Where the person's own file was read: the place chosen for the repository's Armada
    /// configuration, as `get_repository_flight_controls` answers it.
    pub place: RunPlace,
    pub collections: Vec<Placed<RequestCollection>>,
    /// The person's own and the repository's, then one for each server the Job is running.
    pub environments: Vec<Placed<RequestEnvironmentRead>>,
    /// Empty where both files read, an absent file reading as an empty one.
    #[serde(default)]
    pub not_read: Vec<RequestsNotRead>,
}

/// A variable as Bridge writes it. **A secret's `value` goes once, toward Fleet**, which keeps it
/// in the Keychain; a secret written with none keeps the one already kept.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "VariableAsWritten", into = "VariableWritten")]
pub enum VariableWrite {
    Plain { name: String, value: String },
    Secret { name: String, value: Option<String> },
}

/// Written by hand so a log line or a panic cannot print a value, a plain one's included.
impl std::fmt::Debug for VariableWrite {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, name, value) = match self {
            VariableWrite::Plain { name, .. } => ("Plain", name, Some("[hidden]")),
            VariableWrite::Secret { name, value } => {
                ("Secret", name, value.as_ref().map(|_| "[hidden]"))
            }
        };
        formatter.debug_struct(kind).field("name", name).field("value", &value).finish()
    }
}

/// `{ name, secret, value? }` as Bridge sends it. Spelled with `secret` always, as the read is.
#[derive(Clone, Serialize)]
struct VariableWritten {
    name: String,
    secret: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

impl TryFrom<VariableAsWritten> for VariableWrite {
    type Error = String;

    fn try_from(written: VariableAsWritten) -> Result<VariableWrite, String> {
        let name = written.name;
        match (written.secret, written.value) {
            (false, Some(value)) => Ok(VariableWrite::Plain { name, value }),
            (true, value) => Ok(VariableWrite::Secret { name, value }),
            (false, None) => Err(format!("`{name}` has no value: give it one, or mark it secret")),
        }
    }
}

impl From<VariableWrite> for VariableWritten {
    fn from(variable: VariableWrite) -> VariableWritten {
        match variable {
            VariableWrite::Plain { name, value } => {
                VariableWritten { name, secret: false, value: Some(value) }
            }
            VariableWrite::Secret { name, value } => VariableWritten { name, secret: true, value },
        }
    }
}

/// A named set of variables, as Bridge writes it whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvironmentWrite {
    pub name: String,
    #[serde(default)]
    pub variables: Vec<VariableWrite>,
}

/// `save_request_collection`: one collection whole, into the person's own file.
/// `POST /requests/collections`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRequestCollection {
    /// A root `list_repositories` names. Absent, `session_id`'s repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub collection: RequestCollection,
}

/// `save_request_environment`: one environment whole, into the person's own file. `POST
/// /requests/environments`. **The one body that carries a secret's value**: its `Debug` is
/// [`VariableWrite`]'s, which never prints one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRequestEnvironment {
    /// A root `list_repositories` names. Absent, `session_id`'s repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub environment: RequestEnvironmentWrite,
}

/// A request the agent saves: a [`SavedRequest`] before Fleet gives it an id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestToSave {
    pub name: String,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<RequestHeader>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// `save_request_to_collection`: the agent keeps a request it made in a collection.
/// `POST /requests/collections/request`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRequestToCollection {
    /// Read only where the connection places none, as `send_request`'s is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// A collection's id or name. One of that name is made in the person's own where none is.
    pub collection: String,
    pub request: RequestToSave,
}

/// `save_request_to_collection`'s answer: where the request is kept now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSaved {
    pub collection_id: String,
    pub request_id: String,
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
