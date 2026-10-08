//! The process a hosted session runs in, behind one interface. Since 23.49.
//!
//! **Built to change, as `helm::Hosting` is**: a test plants a host that
//! answers with events, and a host elsewhere is a second implementation. What
//! it is asked to carry is a [`Start`] and what it gives back is a [`Process`]
//! and a stream of [`Heard`].
//!
//! **One live process per open session**, unlike Helm's one per message: the
//! process stays up so another session's message can start a turn in it, and
//! Fleet ends it after a quiet spell and resumes it on the next message.
//!
//! **The process is a keeper's, not Fleet's** (`keeper.rs`): the agent runs
//! under `armada session-keep`, and Fleet holds a connection to it. A Fleet
//! that restarts finds the keeper again with [`Processes::reattach`].

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{DroneEvent, McpConfig};
use adapters::{HeadlessAgent, HostedLaunch};
use ipc::SessionMode;
use tokio::sync::mpsc;

use crate::detach::Detached;
use crate::drone::{environment, HostPaths};

/// What one process of a session is started with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start {
    /// The repository's root before the first write, the leased slot after.
    pub directory: String,
    /// The id Fleet minted, which the agent CLI is told to use.
    pub session: String,
    /// Whether a process of this session has run before.
    pub resuming: bool,
    /// The session whose conversation the first process starts as a copy of.
    pub forking: Option<String>,
    /// What another session addresses this one by.
    pub name: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub mode: SessionMode,
    /// Directories beyond the working one the session may read: where its
    /// attachments are kept.
    pub readable: Vec<String>,
}

/// What a process says, in the order it said it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Heard {
    Events(Vec<DroneEvent>),
    /// What the stream's `init` line said the agent has: slash commands and
    /// skills, by name. Since 23.51.
    Commands(Vec<String>),
    /// Fleet is connected to a process that was already running, and `busy`
    /// says whether it was in the middle of a turn.
    Attached { busy: bool },
    /// The process is gone, whoever ended it.
    Gone,
}

pub type Sink = mpsc::UnboundedSender<Heard>;

/// One live process.
pub trait Process: Send + Sync {
    fn pid(&self) -> Option<u32>;

    /// Queue one line on its input. Best effort: a process that is gone says
    /// so through [`Heard::Gone`].
    fn send(&self, line: String);

    /// End it and what it started.
    fn end(&self);
}

/// Something that can start a session's process.
pub trait Processes: Send + Sync + 'static {
    fn start(&self, start: &Start, sink: Sink) -> Result<Arc<dyn Process>, String>;

    /// The process a Fleet before this one started for `session`, if it is
    /// still running. Nothing is restarted: the thread continues from what the
    /// process said while Fleet was away.
    fn reattach(&self, _session: &str, _sink: Sink) -> Option<Arc<dyn Process>> {
        None
    }
}

/// The agent CLI, as a live process under Fleet.
pub struct ProcessHost {
    agent: HeadlessAgent,
    /// The `armada` binary that runs a session's keeper.
    exe: PathBuf,
    /// Where each session's keeper socket and spool are.
    keepers: PathBuf,
    door: PathBuf,
    path: String,
    home: String,
    user: String,
    port: u16,
}

/// The agent's door as a repository's `.mcp.json` names it, spelled as
/// `crate::helm::ProcessHost` spells it for the same reason.
const DOOR_PROGRAM: &str = "armada";
const DOOR_ARGS: &[&str] = &["mcp"];

/// The door's configuration file, beside Helm's.
const DOOR_FILE: &str = "session-mcp.json";

/// How long a keeper has to bind its socket.
const KEEPER_WITHIN: Duration = Duration::from_secs(10);

/// The longest a unix socket's path may be on macOS, less its terminator.
const SOCKET_PATH_MOST: usize = 103;

impl ProcessHost {
    fn socket_of(&self, session: &str) -> Result<PathBuf, String> {
        let socket = self.keepers.join(format!("{session}.sock"));
        match socket.as_os_str().len() > SOCKET_PATH_MOST {
            true => Err(format!(
                "{} is too long for a socket path",
                socket.display()
            )),
            false => Ok(socket),
        }
    }

    pub(crate) fn on_this_machine(host: &crate::daemon::Host) -> ProcessHost {
        ProcessHost {
            agent: HeadlessAgent::at(host.agent_binary.clone()),
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("armada")),
            keepers: PathBuf::from(&host.keepers_dir),
            door: std::path::Path::new(&host.mcp_config).with_file_name(DOOR_FILE),
            path: host.path.clone(),
            home: host.home.clone(),
            user: host.user.clone(),
            port: host.port,
        }
    }
}

impl Processes for ProcessHost {
    fn start(&self, start: &Start, sink: Sink) -> Result<Arc<dyn Process>, String> {
        adapters::publish_the_agents_door(&self.door, DOOR_PROGRAM, DOOR_ARGS)
            .map_err(|why| why.to_string())?;
        let door = McpConfig::only_these(&self.door.to_string_lossy()).map_err(|why| why.said())?;
        let env = environment(
            HostPaths {
                path: &self.path,
                home: &self.home,
                user: &self.user,
            },
            &[],
        )
        .map_err(|why| why.said())?;
        let hosted = HostedLaunch::at(
            &start.directory,
            &start.session,
            start.resuming,
            &start.name,
            start.model.as_deref(),
            start.effort.as_deref(),
            start.mode,
            door,
            env,
            &format!("http://127.0.0.1:{}/sessions/gate", self.port),
            start.readable.clone(),
        )
        .map_err(|why| why.to_string())?;
        let hosted = match &start.forking {
            Some(old) => hosted.forking(old).map_err(|why| why.to_string())?,
            None => hosted,
        };
        let launch = self
            .agent
            .render_hosted_session(&hosted)
            .map_err(|why| why.to_string())?;
        std::fs::create_dir_all(&self.keepers).map_err(|why| why.to_string())?;
        let socket = self.socket_of(&start.session)?;
        let spool = self.keepers.join(format!("{}.spool", start.session));
        let alone = super::keeper::ALONE_FOR.as_secs().to_string();
        let mut keeper = Detached::program(&self.exe)
            .arg("session-keep")
            .arg(&socket)
            .arg(&spool)
            .arg(&alone)
            .arg("--")
            .arg(launch.program())
            .args(launch.args())
            .in_directory(launch.directory())
            .in_environment(launch.environment())
            .outliving_fleet()
            .ignoring_output()
            .spawn()
            .map_err(|why| format!("the session's keeper would not start: {why}"))?;
        // A keeper binds within milliseconds. Polled, because the trait this
        // answers is not async, and bounded, so a keeper that died says so.
        let waited = std::time::Instant::now();
        let stream = loop {
            if let Some(stream) = super::kept::connect(&socket) {
                break stream;
            }
            if let Ok(Some(status)) = keeper.try_wait() {
                return Err(format!(
                    "the session's keeper exited ({status}) before it answered"
                ));
            }
            if waited.elapsed() > KEEPER_WITHIN {
                return Err(String::from("the session's keeper did not answer"));
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        super::kept::attached(stream, self.agent.clone(), sink)
    }

    fn reattach(&self, session: &str, sink: Sink) -> Option<Arc<dyn Process>> {
        let socket = self.socket_of(session).ok()?;
        let stream = super::kept::connect(&socket)?;
        super::kept::attached(stream, self.agent.clone(), sink).ok()
    }
}
