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
    /// What every process of a fork is told it is. Read only with `forking`, which is refused without it.
    pub fork_note: Option<String>,
    /// The transcript line the copy ends at, included. Read only with `forking`. Since 23.79.
    pub fork_cut: Option<String>,
    /// What every process of the session is told it is, appended to its system prompt. Helm's
    /// note for a repository's Helm Session, and none for any other.
    pub note: Option<String>,
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
    /// The commands a line asked for or answered, as `adapters::session_runs` reads them.
    Ran(Vec<adapters::session_runs::Record>),
    /// What a subagent inside the process did, under the call that started it. Its words are not the
    /// agent's and are not drawn, but its reads, edits and commands are the session's work.
    Subagent { events: Vec<DroneEvent>, call: Option<String> },
    /// Subagents a line started: the call of each, the type it asked for, and whether it runs in the background.
    Spawned(Vec<adapters::SubagentCall>),
    /// What the stream's `init` line said the agent has: slash commands and
    /// skills, by name. Since 23.51.
    Commands(Vec<String>),
    /// Fleet is connected to a process that was already running, and `busy`
    /// says whether it was in the middle of a turn.
    Attached {
        busy: bool,
    },
    /// A `control_response`: what the process answered a control line Fleet wrote. Not a turn's.
    Answered(adapters::ControlAnswer),
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

    /// The last of what it said on its error output, and how it ended, for a
    /// process that is gone. Empty where there is none.
    fn complaint(&self) -> String {
        String::new()
    }
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
    /// What the agent's own builds may take: `crate::drone::HostPaths::width`.
    width: checks_runner::CheckWidth,
    port: u16,
    /// Where each Job's scratch folder is: `crate::scratch`.
    attachments: PathBuf,
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

    pub(crate) fn on_this_machine(
        host: &crate::daemon::Host,
        width: checks_runner::CheckWidth,
    ) -> ProcessHost {
        ProcessHost {
            agent: HeadlessAgent::at(host.agent_binary.clone()),
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("armada")),
            keepers: PathBuf::from(&host.keepers_dir),
            door: std::path::Path::new(&host.mcp_config).with_file_name(DOOR_FILE),
            path: host.path.clone(),
            home: host.home.clone(),
            user: host.user.clone(),
            width,
            port: host.port,
            attachments: PathBuf::from(&host.attachments_dir),
        }
    }
}

impl Processes for ProcessHost {
    fn start(&self, start: &Start, sink: Sink) -> Result<Arc<dyn Process>, String> {
        adapters::publish_the_agents_door(&self.door, DOOR_PROGRAM, DOOR_ARGS)
            .map_err(|why| why.to_string())?;
        let door = McpConfig::only_these(&self.door.to_string_lossy()).map_err(|why| why.said())?;
        let scratch = crate::scratch::scratch_under(&self.attachments, &start.session);
        let _ = std::fs::create_dir_all(&scratch);
        let env = environment(
            HostPaths {
                path: &self.path,
                home: &self.home,
                user: &self.user,
                width: self.width,
            },
            &[],
        )
        // The id `armada worktree lease` holds a slot under, so Fleet ending
        // and resuming this process does not read as the lease's holder gone.
        .and_then(|env| env.and(adapters::leasing::Holder::SESSION_ENV, &start.session))
        .and_then(|env| env.and(crate::scratch::SCRATCH_ENV, &scratch.to_string_lossy()))
        .map_err(|why| why.said())?;
        // A conversation resumed from another directory (a Session moving into
        // its slot) is not found by the CLI unless it is put where it looks.
        if let Some(old) = start
            .forking
            .as_deref()
            .or(start.resuming.then_some(start.session.as_str()))
        {
            adapters::terminal_thread::bring_conversation_to(&self.home, old, &start.directory)
                .map_err(|why| {
                    format!(
                        "the conversation would not follow into {}: {why}",
                        start.directory
                    )
                })?;
        }
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
        let hosted = match (&start.forking, &start.fork_note) {
            (Some(old), note) => hosted
                .forking(old, note.as_deref().unwrap_or_default())
                .map_err(|why| why.to_string())?,
            (None, _) => hosted,
        };
        let hosted = match &start.fork_cut {
            Some(line) => hosted.cut_at(line).map_err(|why| why.to_string())?,
            None => hosted,
        };
        let hosted = match &start.note {
            Some(note) => hosted.noting(note),
            None => hosted,
        };
        // The copy's plugin, when it holds one: a job gets the mod whether or not the person ever
        // installed it into their CLI.
        let plugin = crate::runtime::mod_dir(&self.home);
        let hosted = match plugin.join(adapters::MOD_INSTALLED_MANIFEST).is_file() {
            true => hosted
                .loading_mod(&plugin.join("armada").to_string_lossy())
                .map_err(|why| why.to_string())?,
            false => hosted,
        };
        let launch = self
            .agent
            .render_hosted_session(&hosted)
            .map_err(|why| why.to_string())?;
        std::fs::create_dir_all(&self.keepers).map_err(|why| why.to_string())?;
        let socket = self.socket_of(&start.session)?;
        let spool = self.keepers.join(format!("{}.spool", start.session));
        let log = spool.with_extension("log");
        // Emptied, then held for append: the keeper's own error and the
        // agent's, which it writes later, land in one file.
        let logged = std::fs::write(&log, "")
            .and_then(|()| std::fs::OpenOptions::new().append(true).open(&log))
            .map_err(|why| format!("the job's keeper has no log: {why}"))?;
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
            .writing_output_to(&logged)
            .map_err(|why| why.to_string())?
            .spawn()
            .map_err(|why| format!("the job's keeper would not start: {why}"))?;
        // A keeper binds within milliseconds. Polled, because the trait this
        // answers is not async, and bounded, so a keeper that died says so.
        let waited = std::time::Instant::now();
        let stream = loop {
            if let Some(stream) = super::kept::connect(&socket) {
                break stream;
            }
            if let Ok(Some(status)) = keeper.try_wait() {
                return Err(format!(
                    "the job's keeper exited ({status}) before it answered{}",
                    super::kept::tail_of(&log)
                ));
            }
            if waited.elapsed() > KEEPER_WITHIN {
                return Err(String::from("the job's keeper did not answer"));
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        super::kept::attached(stream, self.agent.clone(), sink, &socket, &spool)
    }

    fn reattach(&self, session: &str, sink: Sink) -> Option<Arc<dyn Process>> {
        let socket = self.socket_of(session).ok()?;
        let spool = self.keepers.join(format!("{session}.spool"));
        let stream = super::kept::connect(&socket)?;
        super::kept::attached(stream, self.agent.clone(), sink, &socket, &spool).ok()
    }
}
