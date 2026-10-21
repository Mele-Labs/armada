//! The servers a hosted job's process has connected, read and acted on through
//! the control lines its input takes. The CLI pushes no line when a server's
//! state changes, so the state is read by asking.
//!
//! **Bytes enter through `ipc::decode`**, for `crate::transcript`'s reason.

use serde::{Deserialize, Serialize};

/// What one control line asks of a process about its servers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpRequest<'a> {
    Status,
    Reconnect(&'a str),
    TurnOn(&'a str),
    TurnOff(&'a str),
    SignIn(&'a str),
}

/// Where one server stands, in the CLI's five words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpState {
    Connected,
    Failed,
    NeedsSignIn,
    Connecting,
    Off,
}

/// One server a process has, as its `mcp_status` answer lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpServer {
    pub name: String,
    pub state: McpState,
    /// Why it failed, in the CLI's words. Also carries a state word Armada does not know.
    pub error: Option<String>,
    /// Where it was configured: `project`, `user`, `dynamic` and the like.
    pub scope: Option<String>,
    /// The tools it offers. None until it is connected.
    pub tools: usize,
}

/// Where a person signs a server in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpSignIn {
    pub url: String,
    /// The process waits on a local callback and reconnects the server itself
    /// once the person has signed in. `false` for a page that keeps the sign-in
    /// on the account, after which the server is reconnected by asking.
    pub callback_expected: bool,
}

/// What a successful answer carried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answered {
    Servers(Vec<McpServer>),
    /// `None` where the server is signed in already.
    SignIn(Option<McpSignIn>),
    /// Nothing: a reconnect or a toggle that settled.
    Done,
}

/// One `control_response` line, under the id its request was sent with. The
/// `Err` is the CLI's own words for why it refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlAnswer {
    pub request_id: String,
    pub outcome: Result<Answered, String>,
}

#[derive(Serialize)]
struct Asking<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    request_id: &'a str,
    request: Asked<'a>,
}

#[derive(Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
enum Asked<'a> {
    McpStatus,
    McpReconnect {
        #[serde(rename = "serverName")]
        server: &'a str,
    },
    McpToggle {
        #[serde(rename = "serverName")]
        server: &'a str,
        enabled: bool,
    },
    McpAuthenticate {
        #[serde(rename = "serverName")]
        server: &'a str,
    },
}

/// The line that asks `asked` of a process, with no newline.
pub fn mcp_request(request_id: &str, asked: McpRequest<'_>) -> String {
    let request = match asked {
        McpRequest::Status => Asked::McpStatus,
        McpRequest::Reconnect(server) => Asked::McpReconnect { server },
        McpRequest::TurnOn(server) => Asked::McpToggle { server, enabled: true },
        McpRequest::TurnOff(server) => Asked::McpToggle { server, enabled: false },
        McpRequest::SignIn(server) => Asked::McpAuthenticate { server },
    };
    ipc::encode(&Asking { kind: "control_request", request_id, request })
        .expect("strings and a flag always encode")
}

#[derive(Deserialize)]
struct Line {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    response: Option<Reply>,
}

#[derive(Deserialize)]
struct Reply {
    #[serde(default)]
    request_id: String,
    #[serde(default)]
    subtype: String,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    response: Option<Body>,
}

#[derive(Deserialize)]
struct Body {
    #[serde(default, rename = "mcpServers")]
    servers: Option<Vec<Listed>>,
    #[serde(default, rename = "authUrl")]
    url: Option<String>,
    #[serde(default, rename = "requiresUserAction")]
    user_acts: Option<bool>,
    #[serde(default, rename = "callbackExpected")]
    callback_expected: bool,
}

#[derive(Deserialize)]
struct Listed {
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    tools: Vec<serde::de::IgnoredAny>,
}

/// The answer one line of a process's output carries: `None` for a line that
/// is not a `control_response`, so a caller may offer every line it hears.
pub fn control_answer(line: &str) -> Option<ControlAnswer> {
    if !line.contains("\"control_response\"") {
        return None;
    }
    let line: Line = ipc::decode("a control answer", line.trim().as_bytes()).ok()?;
    let reply = line.response.filter(|_| line.kind == "control_response")?;
    let outcome = match reply.subtype.as_str() {
        "success" => answered(reply.response),
        _ => Err(reply.error.filter(|why| !why.trim().is_empty()).unwrap_or_else(|| {
            format!("it answered `{}` and said nothing more", reply.subtype)
        })),
    };
    Some(ControlAnswer { request_id: reply.request_id, outcome })
}

fn answered(body: Option<Body>) -> Result<Answered, String> {
    let Some(body) = body else {
        return Ok(Answered::Done);
    };
    if let Some(listed) = body.servers {
        return Ok(Answered::Servers(listed.into_iter().map(server).collect()));
    }
    match (body.user_acts, body.url) {
        (None, _) => Ok(Answered::Done),
        (Some(false), _) => Ok(Answered::SignIn(None)),
        (Some(true), Some(url)) if !url.trim().is_empty() => Ok(Answered::SignIn(Some(McpSignIn {
            url,
            callback_expected: body.callback_expected,
        }))),
        (Some(true), _) => Err(String::from("it asked for a sign-in and named no page to sign in on")),
    }
}

/// **A state word Armada does not know reads as failed, with the word kept.**
/// The row is not dropped, and failed is the one state a person can act on
/// from it: a reconnect.
fn server(listed: Listed) -> McpServer {
    let state = match listed.status.as_str() {
        "connected" => Some(McpState::Connected),
        "failed" => Some(McpState::Failed),
        "needs-auth" => Some(McpState::NeedsSignIn),
        "pending" => Some(McpState::Connecting),
        "disabled" => Some(McpState::Off),
        _ => None,
    };
    let (state, error) = match state {
        Some(state) => (state, listed.error),
        None => (
            McpState::Failed,
            Some(listed.error.unwrap_or_else(|| format!("its state reads `{}`, which Armada does not know", listed.status))),
        ),
    };
    McpServer {
        name: listed.name,
        state,
        error,
        scope: listed.scope,
        tools: listed.tools.len(),
    }
}
