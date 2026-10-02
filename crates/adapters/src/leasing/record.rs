//! Who holds a slot, written as three plain lines beside it.
//!
//! **A process is named by its pid and its start time together**, so a pid
//! the system has since handed to something else never reads as the holder.

use std::path::Path;
use std::process::Command;

/// The process a lease is held for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holder {
    pid: u32,
    started: String,
}

impl Holder {
    /// The running process `pid`, or `None` where there is none.
    pub fn of(pid: u32) -> Option<Holder> {
        let started = ps(pid, "lstart=")?;
        Some(Holder { pid, started })
    }

    /// The process that ran the shell this one was run from: an agent's
    /// session, or the terminal a person typed in. The shell between the two
    /// ends with the command, so it cannot be the holder.
    pub fn the_caller() -> Option<Holder> {
        let shell = std::os::unix::process::parent_id();
        let caller = ps(shell, "ppid=")?.parse().ok()?;
        Holder::of(caller)
    }

    /// A holder as a record wrote it, alive or not.
    pub(crate) fn recorded(pid: u32, started: &str) -> Holder {
        Holder {
            pid,
            started: started.to_string(),
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Whether this process is still the one running under its pid.
    pub fn alive(&self) -> bool {
        ps(self.pid, "lstart=").is_some_and(|started| started == self.started)
    }

    /// What the process is running, for a person reading `--status`.
    pub fn command(&self) -> Option<String> {
        ps(self.pid, "comm=")
    }
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
        let (pid, started) = field("holder")?.split_once(' ')?;
        Some(Record {
            branch: field("branch")?.to_string(),
            holder: Holder::recorded(pid.parse().ok()?, started),
            since: field("since")?.parse().ok()?,
        })
    }

    pub(super) fn write(&self, path: &Path) -> Result<(), String> {
        let text = format!(
            "branch {}\nholder {} {}\nsince {}\n",
            self.branch, self.holder.pid, self.holder.started, self.since
        );
        std::fs::write(path, text).map_err(|why| format!("{}: {why}", path.display()))
    }

    pub(super) fn clear(path: &Path) -> Result<(), String> {
        std::fs::write(path, "").map_err(|why| format!("{}: {why}", path.display()))
    }
}
