//! A session Bridge hosts, as the headless CLI is started for it. Since 23.49.
//! `docs/concepts/session.md`, *A session Fleet hosts*.
//!
//! **A live process, and a person's.** It is [`crate::conversing`]'s launch
//! with the one difference that matters: the process stays up between
//! messages, so another session's `SendMessage` can wake it. It holds what the
//! person's own configuration resolves, with no `--strict-mcp-config` and no
//! `--setting-sources`, so the `armada` mod loads in it and reports through the
//! ledger like any terminal session. A Drone's launch is untouched.
//!
//! **Added to the person's settings, never in place of them**: `--settings`
//! carries two things, the switch that lets another session's message start a
//! turn here and the hook that holds a write until the session has leased a
//! slot.

use std::error::Error;
use std::fmt;

use adapter_traits::{
    DroneSpawnConfig, Environment, Launch, McpConfig, Model, Prompt, SpawnConfigRefused, Toolbelt,
    Worktree,
};
use ipc::SessionMode;
use serde::Serialize;

use crate::harness::HeadlessAgent;

/// What the ledger calls the harness a hosted session runs in: the name the
/// mod in a terminal session reports under, so the two are one harness.
pub const HOSTED_HARNESS: &str = "claude_code";

/// The tools a hosted session's first write is held on. **`Bash` is among
/// them** and a line is told read from write by [`reads_only`].
const GATED_TOOLS: &str = "Write|Edit|NotebookEdit|Bash";

/// Everything one process of a hosted session is started with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedLaunch {
    directory: String,
    session: String,
    resuming: bool,
    /// The session whose conversation this one begins as a copy of.
    fork_of: Option<String>,
    name: String,
    model: Option<Model>,
    effort: Option<String>,
    mode: SessionMode,
    door: McpConfig,
    environment: Environment,
    gate: String,
    readable: Vec<String>,
}

impl HostedLaunch {
    /// A session at `directory`: the repository's root before a lease, the
    /// slot after it. `session` is the id Fleet minted, which the CLI is told
    /// to use so the ledger's row and the CLI's are one; `resuming` is whether
    /// a process of it has run before.
    ///
    /// `gate` is the address of Fleet's hook route and `name` is what another
    /// session addresses it by.
    #[allow(clippy::too_many_arguments)]
    pub fn at(
        directory: &str,
        session: &str,
        resuming: bool,
        name: &str,
        model: Option<&str>,
        effort: Option<&str>,
        mode: SessionMode,
        door: McpConfig,
        environment: Environment,
        gate: &str,
        readable: Vec<String>,
    ) -> Result<HostedLaunch, HostedRefused> {
        if !directory.starts_with('/') {
            return Err(HostedRefused::DirectoryNotAbsolute {
                given: directory.to_string(),
            });
        }
        if !portable(session) {
            return Err(HostedRefused::SessionNotPortable {
                given: session.to_string(),
            });
        }
        if let Some(effort) = effort {
            if !portable(effort) {
                return Err(HostedRefused::EffortNotPortable {
                    given: effort.to_string(),
                });
            }
        }
        let model = match model.filter(|model| !model.trim().is_empty()) {
            Some(model) => Some(Model::named(model).map_err(HostedRefused::Unassembled)?),
            None => None,
        };
        Ok(HostedLaunch {
            directory: directory.to_string(),
            session: session.to_string(),
            resuming,
            fork_of: None,
            name: name.to_string(),
            model,
            effort: effort.map(str::to_string),
            mode,
            door,
            environment,
            gate: gate.to_string(),
            readable,
        })
    }
}

impl HostedLaunch {
    /// Begin as a copy of `old`'s conversation, under this launch's own id.
    /// **Only the first process forks**: once `resuming`, the session has a
    /// conversation of its own and resumes that.
    pub fn forking(mut self, old: &str) -> Result<HostedLaunch, HostedRefused> {
        if !portable(old) {
            return Err(HostedRefused::SessionNotPortable {
                given: old.to_string(),
            });
        }
        self.fork_of = Some(old.to_string());
        Ok(self)
    }
}

/// An id or a word that goes into argv: letters, digits, `-` and `_`, and
/// never a leading `-`, which would be read as a flag.
fn portable(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with('-')
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[derive(Serialize)]
struct Settings<'a> {
    #[serde(rename = "crossSessionInbound")]
    cross_session_inbound: &'static str,
    hooks: Hooks<'a>,
}

#[derive(Serialize)]
struct Hooks<'a> {
    #[serde(rename = "PreToolUse")]
    pre_tool_use: [Matched<'a>; 1],
}

#[derive(Serialize)]
struct Matched<'a> {
    matcher: &'static str,
    hooks: [Hook<'a>; 1],
}

#[derive(Serialize)]
struct Hook<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    url: &'a str,
    /// Seconds. A lease waits on the pool, so this is generous.
    timeout: u32,
}

/// The CLI's own spelling of each mode. **`ask` and `auto` are both
/// `default`**: auto mode is not reachable for a spawned session (spike 25), so
/// Fleet's door decides what is put to the person, and `ask` is the same mode
/// with the door asking about everything the person's settings do not cover.
fn cli_mode(mode: SessionMode) -> &'static str {
    match mode {
        SessionMode::Ask | SessionMode::Auto => "default",
        SessionMode::AcceptEdits => "acceptEdits",
        SessionMode::Plan => "plan",
    }
}

impl HeadlessAgent {
    /// One process of a hosted session.
    ///
    /// **What is absent is half the point**, as for a conversation: no
    /// `--strict-mcp-config`, no `--tools`, no `--setting-sources`.
    pub fn render_hosted_session(&self, hosted: &HostedLaunch) -> Result<Launch, HostedRefused> {
        let settings = ipc::encode(&Settings {
            cross_session_inbound: "accept",
            hooks: Hooks {
                pre_tool_use: [Matched {
                    matcher: GATED_TOOLS,
                    hooks: [Hook {
                        kind: "http",
                        url: &hosted.gate,
                        timeout: 600,
                    }],
                }],
            },
        })
        .map_err(|why| HostedRefused::SettingsNotWritten(why.to_string()))?;
        let mut args: Vec<String> = vec![
            "-p".into(),
            "--input-format".into(),
            "stream-json".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--permission-mode".into(),
            cli_mode(hosted.mode).into(),
            "--permission-prompt-tool".into(),
            crate::conversing::asks_a_person(),
            "--mcp-config".into(),
            hosted.door.path().into(),
            "--settings".into(),
            settings,
            "--name".into(),
            hosted.name.clone(),
        ];
        if let Some(model) = &hosted.model {
            args.push("--model".into());
            args.push(model.as_str().into());
        }
        if let Some(effort) = &hosted.effort {
            args.push("--effort".into());
            args.push(effort.clone());
        }
        for directory in &hosted.readable {
            args.push("--add-dir".into());
            args.push(directory.clone());
        }
        match (&hosted.fork_of, hosted.resuming) {
            (Some(old), false) => {
                args.push("--resume".into());
                args.push(old.clone());
                args.push("--fork-session".into());
                args.push("--session-id".into());
                args.push(hosted.session.clone());
            }
            (_, resuming) => {
                args.push(if resuming { "--resume" } else { "--session-id" }.into());
                args.push(hosted.session.clone());
            }
        }
        let borrowed = DroneSpawnConfig::spawn_in(
            &Worktree::at(hosted.directory.clone(), ""),
            hosted
                .model
                .clone()
                .unwrap_or(Model::named("default").map_err(HostedRefused::Unassembled)?),
            Prompt::assembled("carried on stdin").map_err(HostedRefused::Unassembled)?,
            hosted.door.clone(),
            Toolbelt::evidence_only(),
            hosted.environment.clone(),
        );
        Ok(Launch::rendered(&borrowed, self.program(), args))
    }
}

/// Whether a shell line only reads. **Conservative**: a line this does not
/// recognise is a write, so the first one leases a slot rather than touching
/// the main checkout. `crate::conversing::wrote_the_checkout` is the file
/// tools' half.
pub fn reads_only(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return true;
    }
    // A redirect, a substitution or a chain that this reading cannot follow.
    if line.contains('>') || line.contains('`') || line.contains("$(") {
        return false;
    }
    line.split(['|', ';', '&', '\n'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .all(reads)
}

/// One segment of a line.
fn reads(segment: &str) -> bool {
    let named = |read: &&str| {
        segment == *read
            || segment
                .strip_prefix(*read)
                .is_some_and(|rest| rest.starts_with(' '))
    };
    if segment == "git branch"
        || ["--list", "--show-current", "-a", "-r", "-v"]
            .iter()
            .any(|flag| segment.starts_with(&format!("git branch {flag}")))
    {
        return true;
    }
    if segment.starts_with("find ") && (segment.contains("-delete") || segment.contains("-exec")) {
        return false;
    }
    READS.iter().any(|read| named(read))
}

const READS: &[&str] = &[
    "ls",
    "cat",
    "head",
    "tail",
    "wc",
    "pwd",
    "echo",
    "grep",
    "rg",
    "find",
    "which",
    "file",
    "stat",
    "tree",
    "diff",
    "sort",
    "uniq",
    "cut",
    "date",
    "whoami",
    "printenv",
    "ps",
    "true",
    "false",
    "test",
    "[",
    "git status",
    "git log",
    "git diff",
    "git show",
    "git rev-parse",
    "git ls-files",
    "git blame",
    "git describe",
    "git fetch",
    "gh pr view",
    "gh pr list",
    "gh pr diff",
    "gh pr checks",
    "gh issue view",
    "gh issue list",
    "gh run view",
    "gh run list",
    "armada",
];

/// Why a hosted session's process could not be rendered. Nothing has started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostedRefused {
    DirectoryNotAbsolute { given: String },
    SessionNotPortable { given: String },
    EffortNotPortable { given: String },
    SettingsNotWritten(String),
    Unassembled(SpawnConfigRefused),
}

impl fmt::Display for HostedRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostedRefused::DirectoryNotAbsolute { given } => write!(
                out,
                "the session's directory `{given}` is relative, so it would open wherever \
                 Fleet happened to be"
            ),
            HostedRefused::SessionNotPortable { given } => {
                write!(
                    out,
                    "the session id `{given}` is not one the agent CLI takes"
                )
            }
            HostedRefused::EffortNotPortable { given } => {
                write!(out, "the effort `{given}` is not one the agent CLI takes")
            }
            HostedRefused::SettingsNotWritten(why) => {
                write!(out, "the session's settings would not encode: {why}")
            }
            HostedRefused::Unassembled(cause) => out.write_str(&cause.said()),
        }
    }
}

impl Error for HostedRefused {}
