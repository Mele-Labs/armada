//! What a Job costs the machine: its processes, their load, the worktrees it holds and the logs Fleet
//! keeps for it. `get_session_system` and `stop_process` in `crates/ipc/operations/`.
//!
//! **Every reading is the machine's at the moment of asking and nothing is kept.** A figure that could
//! not be read is absent, never zero.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// Whether a process still runs. **Strict**: Bridge picks a mark from it. Fleet lists only what runs,
/// so it answers `running`; Bridge keeps a row it has just stopped as `stopped` until the next reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemProcessState {
    Running,
    Stopped,
}

/// One process the Job started and that is still running.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemProcess {
    pub pid: u32,
    /// The command line, as `ps` shows it.
    pub command: String,
    /// The TCP port it listens on, where it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    pub state: SystemProcessState,
}

/// A worktree the Job holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemWorktree {
    pub slot: u32,
    pub path: String,
    /// The branch it is on, where one is recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// A file Fleet keeps for the Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemLog {
    /// The file's name: `session.log` for what the agent's process wrote to its error output, else the
    /// transcript's own.
    pub name: String,
    pub bytes: u64,
}

/// What `get_session_system` answers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionSystem {
    pub session_id: SessionId,
    pub processes: Vec<SystemProcess>,
    /// The processes' CPU together, as a percentage of one core, so above 100 where they use several.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,
    /// The processes' resident memory together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    /// The machine's memory, for the reading to be set against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_of_bytes: Option<u64>,
    /// What the Job's slot takes on disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_bytes: Option<u64>,
    /// The size of the volume the slot is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_of_bytes: Option<u64>,
    pub worktrees: Vec<SystemWorktree>,
    pub logs: Vec<SystemLog>,
}

/// `stop_process`: end one process the Job started, named by `pid` or by the `port` it listens on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopProcess {
    /// Which Job. **Read only where the connection places no hosted session**, as `AllDone`'s is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// The process to end. Exactly one of `pid` and `port` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// The port a process of the Job listens on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

/// What `stop_process` answers: the process that was ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessStopped {
    pub pid: u32,
    pub command: String,
}
