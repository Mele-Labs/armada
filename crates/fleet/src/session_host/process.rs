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

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use adapter_traits::{AgentHarness, DroneEvent, McpConfig};
use adapters::{HeadlessAgent, HostedLaunch};
use ipc::SessionMode;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

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
}

/// The agent CLI, as a live process under Fleet.
pub struct ProcessHost {
    agent: HeadlessAgent,
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

/// How much of the CLI's own complaint is kept.
const COMPLAINT: u64 = 2048;

impl ProcessHost {
    pub(crate) fn on_this_machine(host: &crate::daemon::Host) -> ProcessHost {
        ProcessHost {
            agent: HeadlessAgent::at(host.agent_binary.clone()),
            door: std::path::Path::new(&host.mcp_config).with_file_name(DOOR_FILE),
            path: host.path.clone(),
            home: host.home.clone(),
            user: host.user.clone(),
            port: host.port,
        }
    }
}

struct Live {
    pid: Option<u32>,
    input: mpsc::UnboundedSender<String>,
    end: std::sync::Mutex<Option<oneshot::Sender<()>>>,
}

impl Process for Live {
    fn pid(&self) -> Option<u32> {
        self.pid
    }

    fn send(&self, line: String) {
        let _ = self.input.send(line);
    }

    fn end(&self) {
        if let Ok(mut held) = self.end.lock() {
            if let Some(end) = held.take() {
                let _ = end.send(());
            }
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
        let mut child = Detached::launching(&launch)
            .piping_input()
            .capturing_output()
            .spawn()
            .map_err(|why| format!("`{}` would not start: {why}", launch.program()))?;
        let pid = child.id();
        let (Some(mut stdin), Some(stdout), Some(mut stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return Err(String::from(
                "the session's pipes were lost before they were held",
            ));
        };
        let (input, mut lines) = mpsc::unbounded_channel::<String>();
        let (end, ended) = oneshot::channel::<()>();
        tokio::spawn(async move {
            while let Some(line) = lines.recv().await {
                let line = line + "\n";
                if stdin.write_all(line.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });
        tokio::spawn(async move {
            let mut kept = Vec::new();
            let _ = (&mut stderr).take(COMPLAINT).read_to_end(&mut kept).await;
            let _ = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
        });
        let agent = self.agent.clone();
        tokio::spawn(async move {
            let mut reading = BufReader::new(stdout).lines();
            let mut ended = ended;
            loop {
                tokio::select! {
                    line = reading.next_line() => match line {
                        Ok(Some(line)) => {
                            if let Some(names) = adapters::init_commands(&line) {
                                let _ = sink.send(Heard::Commands(names));
                            }
                            let _ = sink.send(Heard::Events(agent.read(&line)));
                        }
                        _ => break,
                    },
                    _ = &mut ended => {
                        // The child is still ours, uncollected, so the group is
                        // still this process's.
                        if let Some(group) = pid.and_then(NonZeroU32::new) {
                            crate::group::end_the_group(group);
                        }
                        break;
                    }
                }
            }
            let _ = child.wait().await;
            let _ = sink.send(Heard::Gone);
        });
        Ok(Arc::new(Live {
            pid,
            input,
            end: std::sync::Mutex::new(Some(end)),
        }))
    }
}
