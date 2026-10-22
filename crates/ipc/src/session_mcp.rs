//! A hosted job's MCP servers, read from its process and acted on there. `get_session_mcp`,
//! `act_on_session_mcp`. `docs/practices/protocol/a-jobs-mcp-servers.md`.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;

/// Where one server stands, in the harness's five words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerState {
    Connected,
    Failed,
    NeedsSignIn,
    Connecting,
    Off,
}

/// One server a hosted job's process has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMcpServer {
    pub name: String,
    pub state: McpServerState,
    /// Why it failed, in the harness's words; also a state word Armada does not know.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Where it was configured: `project`, `user`, `dynamic` and the like.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// The tools it offers; 0 until it is connected.
    pub tools: u32,
}

/// `get_session_mcp`: the servers, as the process answered just now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMcp {
    pub servers: Vec<SessionMcpServer>,
}

/// What a person can do to one server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerAct {
    Reconnect,
    TurnOn,
    TurnOff,
    SignIn,
}

/// `act_on_session_mcp`: do `act` to `server` in a hosted job's process.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActOnSessionMcp {
    pub session_id: SessionId,
    pub server: String,
    pub act: McpServerAct,
}

/// Where a person signs a server in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpSignIn {
    pub url: String,
    /// The process waits on a local callback and reconnects the server itself once the person has
    /// signed in. `false`: the server is reconnected by asking, after the page is done.
    pub callback_expected: bool,
}

/// The servers after the act, and where to sign in when a sign-in needs the person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMcpActed {
    pub servers: Vec<SessionMcpServer>,
    /// Absent for every act but `sign_in`, and for a server signed in already.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_in: Option<McpSignIn>,
}
