//! A repair, as the headless CLI is started for one turn in a Job's checkout.
//! `docs/contracts/agent-prompt/5i-the-repair-briefs.md`.
//!
//! **A scout's launch with the tools to write.** It is given the edit and run tools a hosted Job has
//! and none that schedules, messages, starts another agent, reaches the web, works in a worktree of
//! its own or loads a skill: those are denied as well as left out of `--tools`, since a deny beats an
//! operator's own allow. No `--restricted`, because a repair reads a transcript outside its checkout
//! and stops a process in a tree it does not own. `--permission-mode dontAsk` with the tools allowed
//! by name, since nobody is there to be asked. No server, no resume: one turn and the process ends.

use std::error::Error;
use std::fmt;

use adapter_traits::{DroneSpawnConfig, Environment, Launch, McpConfig, Model, Prompt, SpawnConfigRefused, Worktree};

use crate::harness::HeadlessAgent;

/// The tools a repair is given.
pub(crate) const REPAIR_TOOLS: &[&str] = &["Bash", "Read", "Edit", "Write", "Grep", "Glob"];

/// What the CLI offers that a repair is not given, denied as well as left out of the toolset.
pub(crate) const NOT_FOR_A_REPAIR: &[&str] = &[
    "NotebookEdit",
    "Task",
    "WebFetch",
    "WebSearch",
    "EnterWorktree",
    "ExitWorktree",
    "CronCreate",
    "CronDelete",
    "RemoteTrigger",
    "ScheduleWakeup",
    "PushNotification",
    "SendMessage",
    "Skill",
    "Workflow",
    "Monitor",
    "DesignSync",
    "TaskCreate",
    "TaskUpdate",
    "TaskStop",
];

/// The tools a repair's launch denies, for a test to hold the list to.
pub fn denied_to_a_repair() -> &'static [&'static str] {
    NOT_FOR_A_REPAIR
}

/// Everything one repair's process is started with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repairing {
    directory: String,
    model: Model,
    servers: McpConfig,
    environment: Environment,
}

impl Repairing {
    /// A repair in `directory`, the checkout it works in.
    pub fn in_checkout(
        directory: &str,
        model: Model,
        servers: McpConfig,
        environment: Environment,
    ) -> Result<Repairing, RepairRefused> {
        if !directory.starts_with('/') {
            return Err(RepairRefused::DirectoryNotAbsolute { given: directory.to_string() });
        }
        Ok(Repairing { directory: directory.trim_end_matches('/').to_string(), model, servers, environment })
    }
}

impl HeadlessAgent {
    /// One repair's process. The brief is not on it: it goes in on stdin.
    pub fn render_repair(&self, repairing: &Repairing) -> Result<Launch, RepairRefused> {
        let args: Vec<String> = vec![
            "-p".into(),
            "--input-format".into(),
            "stream-json".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--model".into(),
            repairing.model.as_str().into(),
            "--permission-mode".into(),
            "dontAsk".into(),
            "--strict-mcp-config".into(),
            "--mcp-config".into(),
            repairing.servers.path().into(),
            "--tools".into(),
            REPAIR_TOOLS.join(","),
            "--allowedTools".into(),
            REPAIR_TOOLS.join(","),
            "--disallowedTools".into(),
            NOT_FOR_A_REPAIR.join(","),
            // One turn, never resumed: nothing is left for a later session.
            "--no-session-persistence".into(),
        ];
        // `Launch`'s one constructor reads a spawn config; only the directory and environment are taken.
        let borrowed = DroneSpawnConfig::spawn_in(
            &Worktree::at(repairing.directory.clone(), ""),
            repairing.model.clone(),
            Prompt::assembled("carried on stdin").map_err(RepairRefused::Unassembled)?,
            repairing.servers.clone(),
            repairing.environment.clone(),
        );
        Ok(Launch::rendered(&borrowed, self.program(), args).unattended())
    }
}

#[derive(Debug)]
pub enum RepairRefused {
    DirectoryNotAbsolute { given: String },
    Unassembled(SpawnConfigRefused),
}

impl fmt::Display for RepairRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepairRefused::DirectoryNotAbsolute { given } => {
                write!(out, "the checkout `{given}` is relative, so the repair would work wherever Fleet happened to be")
            }
            RepairRefused::Unassembled(cause) => out.write_str(&cause.said()),
        }
    }
}

impl Error for RepairRefused {}
