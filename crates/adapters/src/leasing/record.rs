//! Who holds a slot, written as plain lines beside it.
//!
//! **A process is named by its pid and its start time together**, so a pid
//! the system has since handed to something else never reads as the holder.
//! **A Job is named by its id**, and is never gone: Fleet restarting is not
//! the Job ending. **A hosted session is named the same way**, by its own id,
//! because Fleet ends and resumes the process that leased the slot.

use std::path::Path;
use std::process::Command;

/// What a lease is held for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Holder {
    /// A running process: an agent's session, or a person's terminal.
    Process { pid: u32, started: String },
    /// One of Fleet's Jobs, by id. Given back when the Job ends or a person
    /// clears a completed one, never taken back for a dead process.
    Job(String),
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

    /// The Job with this id.
    pub fn job(id: &str) -> Holder {
        Holder::Job(id.to_string())
    }

    /// Who a lease asked for by this process is held for, where `session` is
    /// what [`Holder::SESSION_ENV`] holds. **A hosted session's process is
    /// ended and resumed by Fleet when it moves slots**, and a lease held for
    /// that process then reads as gone and is taken by the next caller. So a
    /// hosted session holds as a Job of its own id, and anything else holds
    /// as the process above the caller.
    pub fn lessee(session: Option<&str>) -> Option<Holder> {
        match session.filter(|id| !id.is_empty()) {
            Some(id) => Some(Holder::job(id)),
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

    /// The process's pid; `None` for a Job.
    pub fn pid(&self) -> Option<u32> {
        match self {
            Holder::Process { pid, .. } => Some(*pid),
            Holder::Job(_) => None,
        }
    }

    /// Whether the holder still holds: the same process under its pid, or a
    /// Job, which always does until it gives the slot back.
    pub fn alive(&self) -> bool {
        match self {
            Holder::Process { pid, started } => {
                ps(*pid, "lstart=").is_some_and(|now| &now == started)
            }
            Holder::Job(_) => true,
        }
    }

    /// Who it is, for a person reading `--status`.
    pub fn said(&self) -> String {
        match self {
            Holder::Process { pid, .. } => {
                let command = ps(*pid, "comm=").unwrap_or_default();
                format!("{command} (pid {pid})")
            }
            Holder::Job(id) => format!("job {id}"),
        }
    }

    pub(super) fn written(&self) -> String {
        match self {
            Holder::Process { pid, started } => format!("{pid} {started}"),
            Holder::Job(id) => format!("job {id}"),
        }
    }

    pub(super) fn read(said: &str) -> Option<Holder> {
        if let Some(id) = said.strip_prefix("job ") {
            return Some(Holder::job(id));
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
    /// Why a release was refused when the holder gave the slot up, where one
    /// was. Only a Job's lease carries one: it is how a slot a finished Job
    /// could not give back says so.
    pub(super) kept: Option<String>,
    /// The agent Session that holds this lease, or held it last, for a holder that is a process:
    /// the holder alone cannot say whose it was, once it is gone. Written two ways. `armada
    /// worktree lease` writes the Session whose command took the lease, from the agent's own
    /// environment. Fleet writes it with `Pool::note_session` for a live process holder whose
    /// tree it finds in a Session, which also fills leases taken before either existed. A Job's
    /// record never carries one: it already names its Session. A record with none was taken by hand.
    pub(super) session: Option<String>,
    /// The Job completed and holds the slot until a person clears it.
    pub(super) completed: bool,
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
            kept: field("kept").map(str::to_string),
            session: field("session").map(str::to_string),
            completed: text.lines().any(|line| line == "completed"),
        })
    }

    pub(super) fn write(&self, path: &Path) -> Result<(), String> {
        let mut text = format!(
            "branch {}\nholder {}\nsince {}\n",
            self.branch,
            self.holder.written(),
            self.since
        );
        if let Some(why) = &self.kept {
            text.push_str(&format!("kept {}\n", why.replace('\n', " ")));
        }
        if let Some(session) = &self.session {
            text.push_str(&format!("session {}\n", session.replace('\n', " ")));
        }
        if self.completed {
            text.push_str("completed\n");
        }
        std::fs::write(path, text).map_err(|why| format!("{}: {why}", path.display()))
    }

    pub(super) fn clear(path: &Path) -> Result<(), String> {
        std::fs::write(path, "").map_err(|why| format!("{}: {why}", path.display()))
    }
}
