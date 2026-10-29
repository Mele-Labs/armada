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

use std::path::{Path, PathBuf};
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

/// A Job's id as Fleet mints it, standing in for every one when a folder is measured.
const LONGEST_SESSION: &str = "00000000-0000-0000-0000-000000000000";

/// Where keepers go when the machine folder is too deep for a socket: `/tmp/armada-<user>/sessions`.
const PRIVATE_ROOT: &str = "/tmp";

/// Where each Job's keeper socket goes: `machine/sessions` while a socket there fits, so keepers
/// already running are found after an upgrade. A long username puts it past the limit, and then a
/// folder of this user's own under /tmp, which anyone may write to: Fleet refuses one it did not make
/// private, since a keeper's socket drives an agent. One refused there gives way to the user's own
/// temporary folder, which macOS makes for that user alone.
pub fn keepers_dir(machine: &Path, home: &Path, user: &str) -> Result<PathBuf, String> {
    keepers_dir_under(machine, home, user, Path::new(PRIVATE_ROOT), &std::env::temp_dir())
}

fn keepers_dir_under(machine: &Path, home: &Path, user: &str, root: &Path, own: &Path) -> Result<PathBuf, String> {
    let usual = machine.join("sessions");
    if fits(&usual) {
        keepers_made(&usual, home, root)?;
        return Ok(usual);
    }
    let private = root.join(format!("armada-{user}")).join("sessions");
    let refused = match keepers_made(&private, home, root) {
        Ok(()) => return Ok(private),
        Err(refused) => refused,
    };
    let own = own.join("armada-sessions");
    match fits(&own) && keepers_made(&own, home, root).is_ok() {
        true => Ok(own),
        false => Err(refused),
    }
}

fn fits(keepers: &Path) -> bool {
    keepers.join(format!("{LONGEST_SESSION}.sock")).as_os_str().len() <= SOCKET_PATH_MOST
}

/// Make the keepers' folder if it is gone, as /tmp's cleaning may leave it. One under `root` is made
/// 0700 and refused unless it is a folder owned by whoever owns `home` that nobody else may open.
fn keepers_made(keepers: &Path, home: &Path, root: &Path) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    let Some(private) = keepers.parent().filter(|_| keepers.starts_with(root)) else {
        return std::fs::create_dir_all(keepers).map_err(|why| why.to_string());
    };
    let made = |dir: &Path| match std::fs::DirBuilder::new().mode(0o700).create(dir) {
        Err(why) if why.kind() != std::io::ErrorKind::AlreadyExists => Err(format!("{}: {why}", dir.display())),
        _ => Ok(()),
    };
    made(private)?;
    let owner = std::fs::metadata(home).map_err(|why| format!("{}: {why}", home.display()))?.uid();
    let found = std::fs::symlink_metadata(private).map_err(|why| format!("{}: {why}", private.display()))?;
    if !found.is_dir() || found.uid() != owner || found.mode() & 0o777 != 0o700 {
        return Err(format!(
            "{} is not a folder only this user can open, so Fleet keeps no job sockets in it",
            private.display()
        ));
    }
    made(keepers)
}

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
        let launch = self
            .agent
            .render_hosted_session(&hosted)
            .map_err(|why| why.to_string())?;
        keepers_made(&self.keepers, Path::new(&self.home), Path::new(PRIVATE_ROOT))?;
        let socket = self.socket_of(&start.session)?;
        let spool = self.keepers.join(format!("{}.spool", start.session));
        let log = spool.with_extension("log");
        // Emptied, then held for append: the keeper's own error and the
        // agent's, which it writes later, land in one file.
        let logged = std::fs::write(&log, "")
            .and_then(|()| std::fs::OpenOptions::new().append(true).open(&log))
            .map_err(|why| format!("the agent's log could not be made: {why}"))?;
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
            .map_err(|why| format!("the agent would not start: {why}"))?;
        // A keeper binds within milliseconds. Polled, because the trait this
        // answers is not async, and bounded, so a keeper that died says so.
        let waited = std::time::Instant::now();
        let stream = loop {
            if let Some(stream) = super::kept::connect(&socket) {
                break stream;
            }
            if let Ok(Some(status)) = keeper.try_wait() {
                return Err(format!("the agent exited ({status}) before it came up"));
            }
            if waited.elapsed() > KEEPER_WITHIN {
                return Err(format!("the agent did not come up within {} seconds", KEEPER_WITHIN.as_secs()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::tmp::TempDir;
    use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// A short folder straight under /tmp, since a machine folder only fits a socket where its path is
    /// short, and the system temporary folder is not on macOS.
    struct Short(PathBuf);

    impl Short {
        fn new() -> Short {
            let path = PathBuf::from(format!("/tmp/ak-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            std::fs::create_dir_all(&path).expect("a short folder");
            Short(path)
        }
        /// A path under this folder whose socket path, `sessions/<id>.sock`, is `socket` bytes long.
        fn machine_for(&self, socket: usize) -> PathBuf {
            let own = "/sessions/".len() + LONGEST_SESSION.len() + ".sock".len();
            let wanted = socket - own - self.0.as_os_str().len() - 1;
            self.0.join("m".repeat(wanted))
        }
    }

    impl Drop for Short {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn mode_of(path: &Path) -> u32 {
        std::fs::metadata(path).expect("the folder is there").mode() & 0o777
    }

    #[test]
    fn a_machine_folder_whose_sockets_fit_keeps_them_and_is_made() {
        let (short, root, home) = (Short::new(), TempDir::new(), TempDir::new());
        let machine = short.machine_for(SOCKET_PATH_MOST);

        let made = keepers_dir_under(&machine, home.path(), "nick", root.path(), &short.0);

        assert_eq!(made, Ok(machine.join("sessions")));
        assert!(machine.join("sessions").is_dir());
        assert!(!root.path().join("armada-nick").exists(), "nothing is made under the private root");
    }

    #[test]
    fn a_machine_folder_one_byte_too_deep_moves_to_a_private_folder_made_0700() {
        let (short, root, home) = (Short::new(), TempDir::new(), TempDir::new());
        let machine = short.machine_for(SOCKET_PATH_MOST + 1);

        let made = keepers_dir_under(&machine, home.path(), "nick", root.path(), &short.0);

        let private = root.path().join("armada-nick");
        assert_eq!(made, Ok(private.join("sessions")));
        assert_eq!(mode_of(&private), 0o700);
        assert_eq!(mode_of(&private.join("sessions")), 0o700);
        assert!(!machine.exists(), "the machine folder is not made");
    }

    #[test]
    fn a_private_folder_others_can_open_is_refused_for_the_users_own_temporary_folder() {
        let (short, root, home) = (Short::new(), TempDir::new(), TempDir::new());
        let machine = short.machine_for(SOCKET_PATH_MOST + 1);
        let open = root.path().join("armada-nick");
        std::fs::create_dir(&open).unwrap();
        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755)).unwrap();

        let made = keepers_dir_under(&machine, home.path(), "nick", root.path(), &short.0);

        assert_eq!(made, Ok(short.0.join("armada-sessions")));
        assert!(short.0.join("armada-sessions").is_dir());
        assert!(!open.join("sessions").exists(), "nothing is put in the open folder");
        assert_eq!(mode_of(&open), 0o755, "the open folder is left as it was");
    }

    #[test]
    fn an_open_private_folder_with_no_temporary_folder_that_fits_is_an_error_naming_it() {
        let (short, root, home) = (Short::new(), TempDir::new(), TempDir::new());
        let machine = short.machine_for(SOCKET_PATH_MOST + 1);
        let too_deep = short.0.join("t".repeat(80));
        let open = root.path().join("armada-nick");
        std::fs::create_dir(&open).unwrap();
        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755)).unwrap();

        let made = keepers_dir_under(&machine, home.path(), "nick", root.path(), &too_deep);

        let why = made.expect_err("no folder is safe");
        assert!(why.contains(&open.display().to_string()), "it names the open folder: {why}");
        assert!(!open.join("sessions").exists());
        assert!(!too_deep.exists());
    }

    #[test]
    fn a_private_folder_that_is_a_link_to_a_0700_folder_is_refused() {
        let (short, root, home, elsewhere) = (Short::new(), TempDir::new(), TempDir::new(), TempDir::new());
        let machine = short.machine_for(SOCKET_PATH_MOST + 1);
        let target = elsewhere.path().join("theirs");
        std::fs::create_dir(&target).unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700)).unwrap();
        let link = root.path().join("armada-nick");
        symlink(&target, &link).unwrap();

        let made = keepers_dir_under(&machine, home.path(), "nick", root.path(), &short.0);

        assert_eq!(made, Ok(short.0.join("armada-sessions")));
        assert!(!target.join("sessions").exists(), "nothing is put where the link leads");
    }
}
