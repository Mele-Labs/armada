//! A hosted job's MCP servers: read from its process, and reconnected, turned on or off or
//! signed in to there.
//!
//! **A control line is not a turn.** It is written with `Process::send` and not
//! `State::send_turn`, so nothing is counted, queued or kept to send again, and its answer is
//! routed by request id to the one call waiting on it (`hearing.rs`).

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::{Answered, McpRequest, McpServer, McpSignIn, McpState};
use api::Refusal;
use tokio::sync::oneshot;

use crate::daemon::Fleet;

/// A job whose servers Fleet cannot reach: run in a terminal, or with no process running. A 422.
pub(crate) const SESSION_MCP_UNREACHABLE: &str = "fleet.session_mcp_unreachable";
/// The job's process refused what was asked, or did not answer in time. A 422.
pub(crate) const SESSION_MCP_REFUSED: &str = "fleet.session_mcp_refused";

/// The CLI answers a status from what it holds.
const STATUS_WITHIN: Duration = Duration::from_secs(10);
/// A reconnect, and a turn on, answer once the connect settles, which a slow server takes long over.
const CONNECT_WITHIN: Duration = Duration::from_secs(90);
/// A sign-in answers once the server has said where its sign-in page is.
const SIGN_IN_WITHIN: Duration = Duration::from_secs(30);

/// Whoever waits on one control line's answer.
pub(crate) type Waiter = oneshot::Sender<Result<Answered, String>>;

/// What a person can do to one server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpAct {
    Reconnect,
    TurnOn,
    TurnOff,
    SignIn,
}

/// The servers as they stand after an act, and where to sign in when the act was a sign-in
/// that needs the person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpActed {
    pub servers: Vec<McpServer>,
    pub sign_in: Option<McpSignIn>,
}

impl From<ipc::McpServerAct> for McpAct {
    fn from(act: ipc::McpServerAct) -> Self {
        match act {
            ipc::McpServerAct::Reconnect => McpAct::Reconnect,
            ipc::McpServerAct::TurnOn => McpAct::TurnOn,
            ipc::McpServerAct::TurnOff => McpAct::TurnOff,
            ipc::McpServerAct::SignIn => McpAct::SignIn,
        }
    }
}

/// The servers as the wire carries them.
pub(crate) fn on_wire(servers: Vec<McpServer>) -> Vec<ipc::SessionMcpServer> {
    servers
        .into_iter()
        .map(|server| ipc::SessionMcpServer {
            name: server.name,
            state: match server.state {
                McpState::Connected => ipc::McpServerState::Connected,
                McpState::Failed => ipc::McpServerState::Failed,
                McpState::NeedsSignIn => ipc::McpServerState::NeedsSignIn,
                McpState::Connecting => ipc::McpServerState::Connecting,
                McpState::Off => ipc::McpServerState::Off,
            },
            error: server.error,
            scope: server.scope,
            tools: u32::try_from(server.tools).unwrap_or(u32::MAX),
        })
        .collect()
}

impl super::Hosts {
    /// A control line's request id no other line this Fleet wrote has had.
    fn next_control(&self) -> String {
        format!("armada-{}", self.minted.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The servers job `id`'s process has, and where each stands.
    pub(crate) async fn session_mcp(&self, id: &str) -> Result<Vec<McpServer>, Refusal> {
        match self.asked_of_process(id, McpRequest::Status, STATUS_WITHIN).await? {
            Answered::Servers(servers) => Ok(servers),
            _ => Err(self.hosted_refusal(
                SESSION_MCP_REFUSED,
                &format!("job {id}'s process answered with no list of servers"),
            )),
        }
    }

    /// Do `act` to `server` in job `id`'s process, then read the servers again.
    pub(crate) async fn session_mcp_acted(&self, id: &str, server: &str, act: McpAct) -> Result<McpActed, Refusal> {
        let (request, within) = match act {
            McpAct::Reconnect => (McpRequest::Reconnect(server), CONNECT_WITHIN),
            McpAct::TurnOn => (McpRequest::TurnOn(server), CONNECT_WITHIN),
            McpAct::TurnOff => (McpRequest::TurnOff(server), STATUS_WITHIN),
            McpAct::SignIn => (McpRequest::SignIn(server), SIGN_IN_WITHIN),
        };
        let sign_in = match self.asked_of_process(id, request, within).await? {
            Answered::SignIn(sign_in) => sign_in,
            _ => None,
        };
        let servers = self.session_mcp(id).await?;
        Ok(McpActed { servers, sign_in })
    }

    /// Write one control line to job `id`'s process and wait up to `within` for its answer.
    async fn asked_of_process(&self, id: &str, request: McpRequest<'_>, within: Duration) -> Result<Answered, Refusal> {
        let refused = |code: &'static str, why: String| self.hosted_refusal(code, &why);
        if self.terminal_session(id).await?.is_some() {
            return Err(refused(
                SESSION_MCP_UNREACHABLE,
                format!("job {id} runs in a terminal, and Fleet reaches the servers of a job it hosts only"),
            ));
        }
        let (session, _) = self.session_and_hosting(id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(id));
        }
        let request_id = self.hosts().next_control();
        let runtime = self.hosts().of(id);
        let (waiter, answer) = oneshot::channel();
        {
            let mut state = runtime.state();
            let Some(process) = state.process.clone() else {
                return Err(refused(
                    SESSION_MCP_UNREACHABLE,
                    format!("job {id} has no process running, so none of its servers is connected. Its next message starts one"),
                ));
            };
            state.controls.insert(request_id.clone(), waiter);
            process.send(adapters::mcp_request(&request_id, request));
        }
        match tokio::time::timeout(within, answer).await {
            Ok(Ok(Ok(answered))) => Ok(answered),
            Ok(Ok(Err(why))) => Err(refused(SESSION_MCP_REFUSED, format!("job {id}'s process refused: {why}"))),
            Ok(Err(_)) => Err(refused(
                SESSION_MCP_REFUSED,
                format!("job {id}'s process ended before it answered. Its next message starts another"),
            )),
            Err(_) => {
                runtime.state().controls.remove(&request_id);
                Err(refused(
                    SESSION_MCP_REFUSED,
                    format!("job {id}'s process did not answer within {} seconds", within.as_secs()),
                ))
            }
        }
    }
}
