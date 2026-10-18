//! Who holds a slot, written as plain lines beside it.
//!
//! **A process is named by its pid and its start time together**, so a pid
//! the system has since handed to something else never reads as the holder.
//! **A hosted session is named by its own id**, and is never gone while it
//! lives: Fleet ends and resumes the process that leased the slot.

use std::path::Path;
use std::process::Command;

/// What a lease is held for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Holder {
    /// A running process: an agent's session, or a person's terminal.
    Process { pid: u32, started: String },
    /// A hosted session, by its id. Given back when the session ends or a
    /// person releases it, never taken back for a dead process.
    Hosted(String),
}

impl Holder {
    /// The running process `pid`, or `None` where there is none.
    pub fn of(pid: u32) -> Option<Holder> {
        let started = ps(pid, "lstart=")?;
        Some(Holder::Process { pid, started })
    }

    /// What Fleet sets in a hosted session's process, to its session id.
    /// `armada worktree lease` reads it, so what the session leases is held
    /// for the session.
    pub const SESSION_ENV: &'static str = "ARMADA_SESSION";

    /// What the agent sets in every command it runs, to its own session's id. A lease taken by a
    /// terminal session's command is written with it, so the pool says which session holds a slot
    /// that a bare process could not.
    pub const AGENT_SESSION_ENV: &'static str = "CLAUDE_CODE_SESSION_ID";

    /// The hosted session with this id.
    pub fn hosted(id: &str) -> Holder {
        Holder::Hosted(id.to_string())
    }

    /// Who a lease asked for by this process is held for, where `session` is
    /// what [`Holder::SESSION_ENV`] holds. **A hosted session's process is
    /// ended and resumed by Fleet when it moves slots**, and a lease held for
    /// that process then reads as gone and is taken by the next caller. So a
    /// hosted session holds under its own id, and anything else holds
    /// as the process above the caller.
    pub fn lessee(session: Option<&str>) -> Option<Holder> {
        match session.filter(|id| !id.is_empty()) {
            Some(id) => Some(Holder::hosted(id)),
            None => Holder::the_caller(),
        }
    }

    /// The first process above this one that is not a shell: an agent's
    /// session, or the terminal a person typed in. **Every shell between the
    /// two ends with its command, not only the first**: a pipeline or `&&`
    /// inside `$( )` runs the command under a subshell of the tool's own
    /// shell, and holding for that shell read as gone once the call ended.
    pub fn the_caller() -> Option<Holder> {
        Holder::above(std::os::unix::process::parent_id(), |pid| {
            let parent = ps(pid, "ppid=")?.parse().ok()?;
            Some((parent, ps(pid, "comm=")?))
        })
    }

    /// The first process at or above `start` that is not a shell, where
    /// `table` says each process's parent and command. `None` where the walk
    /// reaches the system's own first process or a process it cannot read.
    pub(crate) fn above(
        start: u32,
        table: impl Fn(u32) -> Option<(u32, String)>,
    ) -> Option<Holder> {
        let mut at = start;
        while at > 1 {
            let (parent, command) = table(at)?;
            if !is_shell(&command) {
                return Holder::of(at);
            }
            at = parent;
        }
        None
    }

    /// The process's pid; `None` for a hosted session.
    pub fn pid(&self) -> Option<u32> {
        match self {
            Holder::Process { pid, .. } => Some(*pid),
            Holder::Hosted(_) => None,
        }
    }

    /// Whether the holder still holds: the same process under its pid, or a
    /// hosted session, which always does until it gives the slot back.
    pub fn alive(&self) -> bool {
        match self {
            Holder::Process { pid, started } => {
                ps(*pid, "lstart=").is_some_and(|now| &now == started)
            }
            Holder::Hosted(_) => true,
        }
    }

    /// Who it is, for a person reading `--status`.
    pub fn said(&self) -> String {
        match self {
            Holder::Process { pid, .. } => {
                let command = ps(*pid, "comm=").unwrap_or_default();
                format!("{command} (pid {pid})")
            }
            Holder::Hosted(id) => format!("job {id}"),
        }
    }

    /// A few characters of who it is, fit to sit in a branch name.
    pub(super) fn short(&self) -> String {
        match self {
            Holder::Process { pid, .. } => format!("pid{pid}"),
            Holder::Hosted(id) => id
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .take(8)
                .collect(),
        }
    }

    pub(super) fn written(&self) -> String {
        match self {
            Holder::Process { pid, started } => format!("{pid} {started}"),
            Holder::Hosted(id) => format!("session {id}"),
        }
    }

    pub(super) fn read(said: &str) -> Option<Holder> {
        // `job ` is how a lease written before Jobs were removed names its holder.
        if let Some(id) = said.strip_prefix("session ").or_else(|| said.strip_prefix("job ")) {
            return Some(Holder::hosted(id));
        }
        let (pid, started) = said.split_once(' ')?;
        Some(Holder::Process {
            pid: pid.parse().ok()?,
            started: started.to_string(),
        })
    }
}

/// Whether `command` (as `ps -o comm=` prints it, possibly a login shell's
/// `-zsh`) is a shell, which ends with the command it ran.
fn is_shell(command: &str) -> bool {
    let name = command.rsplit('/').next().unwrap_or(command);
    matches!(
        name.trim_start_matches('-'),
        "sh" | "bash" | "zsh" | "dash" | "ksh" | "fish"
    )
}

/// One `ps` column for one pid; `None` where no such process is running.
fn ps(pid: u32, column: &str) -> Option<String> {
    let said = Command::new("ps")
        .args(["-o", column, "-p", &pid.to_string()])
        .output()
        .ok()?;
    let said = String::from_utf8_lossy(&said.stdout).trim().to_string();
    (!said.is_empty()).then_some(said)
}

/// A slot's lease, as written.
#[derive(Debug)]
pub(super) struct Record {
    pub(super) branch: String,
    pub(super) holder: Holder,
    /// Seconds since the epoch, as the caller read its clock.
    pub(super) since: u64,
    /// The agent Session that holds this lease, or held it last, for a holder that is a process:
    /// the holder alone cannot say whose it was, once it is gone. Written two ways. `armada
    /// worktree lease` writes the Session whose command took the lease, from the agent's own
    /// environment. Fleet writes it with `Pool::note_session` for a live process holder whose
    /// tree it finds in a Session, which also fills leases taken before either existed. A hosted
    /// holder's record never carries one: it already names its Session. A record with none was taken by hand.
    pub(super) session: Option<String>,
}

impl Record {
    /// The record at `path`; `None` for an empty file, which is a free slot,
    /// and for one this cannot read, which a lease then writes over.
    pub(super) fn read(path: &Path) -> Option<Record> {
        let text = std::fs::read_to_string(path).ok()?;
        let field = |name: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(name)?.strip_prefix(' '))
        };
        Some(Record {
            branch: field("branch")?.to_string(),
            holder: Holder::read(field("holder")?)?,
            since: field("since")?.parse().ok()?,
            session: field("session").map(str::to_string),
        })
    }

    pub(super) fn write(&self, path: &Path) -> Result<(), String> {
        let mut text = format!(
            "branch {}\nholder {}\nsince {}\n",
            self.branch,
            self.holder.written(),
            self.since
        );
        if let Some(session) = &self.session {
            text.push_str(&format!("session {}\n", session.replace('\n', " ")));
        }
        std::fs::write(path, text).map_err(|why| format!("{}: {why}", path.display()))
    }

    pub(super) fn clear(path: &Path) -> Result<(), String> {
        std::fs::write(path, "").map_err(|why| format!("{}: {why}", path.display()))
    }
}
