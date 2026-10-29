//! The processes a Job started, what they load the machine with, and ending them.
//!
//! **Ownership is [`Fleet::started_by`]'s**: the agent's process tree, and whatever runs a file from
//! a directory the Job holds. Every reading shells out, and one that fails is no reading, never an
//! error.

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;

/// How long a process is given after `TERM` before it is `KILL`ed.
const GRACE: Duration = Duration::from_secs(2);

/// One process a Job started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub command: String,
    /// The TCP port it listens on, if it does.
    pub port: Option<u16>,
}

/// What a Job's processes and its slot take. `None` where the reading could not be made.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Load {
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
    pub disk_bytes: Option<u64>,
}

/// `(pid, command)` for each line of `ps -axo pid=,command=`.
pub(crate) fn listed(listing: &str) -> Vec<(u32, String)> {
    listing
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, command) = line.split_once(' ')?;
            Some((pid.parse().ok()?, command.trim().to_string()))
        })
        .collect()
}

/// `(pid, cpu percent, resident bytes)` for each line of `ps -o pid=,pcpu=,rss=`; `rss` is in KiB.
pub(crate) fn loads(listing: &str) -> Vec<(u32, f32, u64)> {
    listing
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let cpu = fields.next()?.parse().ok()?;
            let kib: u64 = fields.next()?.parse().ok()?;
            Some((pid, cpu, kib * 1024))
        })
        .collect()
}

/// The bytes in `du -sk`'s first line.
pub(crate) fn disk(listing: &str) -> Option<u64> {
    let kib: u64 = listing.split_whitespace().next()?.parse().ok()?;
    Some(kib * 1024)
}

/// What `command` printed, if it ran and succeeded.
fn printed(program: &str, args: &[&str]) -> Option<String> {
    let said = Command::new(program).args(args).output().ok()?;
    said.status.success().then(|| String::from_utf8_lossy(&said.stdout).into_owned())
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
    /// The processes the session started and that still run, its agent and Fleet itself left out.
    /// Blocking: it runs `ps` and `lsof`.
    pub(crate) fn processes_of(&self, id: &str, held: &[String]) -> Vec<Process> {
        let Some(table) = printed("ps", &["-axo", "pid=,command="]) else {
            return Vec::new();
        };
        let started = self.started_by(id, held);
        let agents = self.tree_roots(id);
        let ours = std::process::id();
        let mut found: Vec<(u32, String)> = listed(&table)
            .into_iter()
            .filter(|(pid, _)| *pid != ours && !agents.contains(pid) && started(*pid))
            .collect();
        found.sort_by_key(|(pid, _)| *pid);
        let ports: HashMap<u32, u16> = adapters::leasing::listening_ports().into_iter().collect();
        found
            .into_iter()
            .map(|(pid, command)| Process { pid, command, port: ports.get(&pid).copied() })
            .collect()
    }

    /// What `processes` and the folder `slot` take. Blocking.
    pub(crate) fn load_of(&self, processes: &[Process], slot: &str) -> Load {
        let reading = (!processes.is_empty())
            .then(|| {
                let pids: Vec<String> = processes.iter().map(|one| one.pid.to_string()).collect();
                printed("ps", &["-o", "pid=,pcpu=,rss=", "-p", &pids.join(",")])
            })
            .flatten()
            .map(|said| loads(&said));
        Load {
            cpu_percent: reading.as_ref().map(|all| all.iter().map(|(_, cpu, _)| cpu).sum()),
            memory_bytes: reading.as_ref().map(|all| all.iter().map(|(_, _, bytes)| bytes).sum()),
            disk_bytes: printed("du", &["-sk", slot]).and_then(|said| disk(&said)),
        }
    }

    /// End what the session started and left running: `TERM`, then `KILL` after a short wait.
    /// Blocking, and a no-op where `ps` will not answer.
    pub(crate) fn processes_ended(&self, id: &str, held: &[String]) {
        let pids: Vec<u32> = self.processes_of(id, held).into_iter().map(|one| one.pid).collect();
        adapters::leasing::end_pids(&pids, GRACE);
    }
}
