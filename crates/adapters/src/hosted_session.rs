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
//! slot. Every process carries `--append-system-prompt` once, holding [`SESSION_RULES`], then
//! Helm's note that says what Helm is (`noting`) where there is one, then a fork's note that says
//! whose work the copied conversation describes.

use std::error::Error;
use std::fmt;

use adapter_traits::{
    DroneSpawnConfig, Environment, Launch, McpConfig, Model, Prompt, SpawnConfigRefused,
    Worktree,
};
use ipc::SessionMode;
use serde::Serialize;

use crate::harness::HeadlessAgent;

/// What the ledger calls the harness a hosted session runs in: the name the
/// mod in a terminal session reports under, so the two are one harness.
pub const HOSTED_HARNESS: &str = "claude_code";

/// The same manifest as it lies in the installed copy of the mod, under its folder.
pub const MOD_INSTALLED_MANIFEST: &str = "armada/.claude-plugin/plugin.json";

/// Where a repository keeps the manifest of the `armada` mod its sessions load, which says the
/// version the mod should be at.
pub const MOD_MANIFEST: &str = "plugins/armada/.claude-plugin/plugin.json";

/// The tools the gate hook runs before: **all of them**, since Fleet notes every call for the row's
/// `activity`. Only a write or a `Bash` line is held on a lease, and a line is told read from write
/// by [`reads_only`]; the gate lets every other call through.
const GATED_TOOLS: &str = "*";

/// The first turn waits on no server: the CLI still waits for the door, as the permission tool's.
/// Spike 34.
const MCP_STARTUP_WAIT: (&str, &str) = ("CLAUDE_CODE_MCP_STARTUP_WAIT_MS", "0");

/// Each server's connect timeout, raised so a person's slow server still connects behind the first
/// turn rather than failing at 30 seconds. Spike 34.
const MCP_CONNECT_TIMEOUT: (&str, &str) = ("MCP_TIMEOUT", "120000");

/// Armada's rules for how a hosted session surfaces what it needs and how far it has got. Compiled
/// in and appended to every process whatever output style the person has; no setting reaches it.
/// `docs/contracts/agent-prompt.md`, *The session rules*.
pub const SESSION_RULES: &str = include_str!("session_rules.md");

/// Everything one process of a hosted session is started with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedLaunch {
    directory: String,
    session: String,
    resuming: bool,
    /// The session whose conversation this one begins as a copy of.
    fork_of: Option<String>,
    /// What every process of a fork is told it is, appended to its system prompt.
    fork_note: Option<String>,
    /// The transcript line a fork's copy ends at, included. Since 23.79.
    fork_at: Option<String>,
    /// What every process of the session is told it is, appended to its system prompt: Helm's note.
    note: Option<String>,
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
            fork_note: None,
            fork_at: None,
            note: None,
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
    ///
    /// **A fork cannot be launched without its note**, which is appended to the
    /// system prompt of every process of it: the copy carries the old session's
    /// subagents, tasks and slots in its words, and nothing else says they are
    /// not this one's.
    pub fn forking(mut self, old: &str, note: &str) -> Result<HostedLaunch, HostedRefused> {
        if !portable(old) {
            return Err(HostedRefused::SessionNotPortable {
                given: old.to_string(),
            });
        }
        if note.trim().is_empty() {
            return Err(HostedRefused::ForkNotTold);
        }
        self.fork_of = Some(old.to_string());
        self.fork_note = Some(note.trim().to_string());
        Ok(self)
    }

    /// Tell every process of this session what it is, in its system prompt. **Appended beside a
    /// fork's note and not instead of it**, so a blank one is no note at all.
    pub fn noting(mut self, note: &str) -> HostedLaunch {
        let note = note.trim();
        self.note = (!note.is_empty()).then(|| note.to_string());
        self
    }

    /// End the copy at the transcript line `uuid` of the session forked from,
    /// included. Hidden in the CLI's help; `docs/spikes/030-*`. Read only
    /// where the launch forks.
    pub fn cut_at(mut self, uuid: &str) -> Result<HostedLaunch, HostedRefused> {
        if !portable(uuid) {
            return Err(HostedRefused::SessionNotPortable {
                given: uuid.to_string(),
            });
        }
        self.fork_at = Some(uuid.to_string());
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
        // One argument, since the CLI reads the last of several. The rules lead; a note adds to them.
        let told: Vec<&str> = [Some(SESSION_RULES.trim()), hosted.note.as_deref(), hosted.fork_note.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        args.push("--append-system-prompt".into());
        args.push(told.join("\n\n"));
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
                if let Some(at) = &hosted.fork_at {
                    args.push("--resume-session-at".into());
                    args.push(at.clone());
                }
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
            crate::browser_shell::environment(
                hosted
                    .environment
                    .clone()
                    .and(crate::conversing::TASK_TOOLS, "1")
                    .and_then(|env| env.and(MCP_STARTUP_WAIT.0, MCP_STARTUP_WAIT.1))
                    .and_then(|env| env.and(MCP_CONNECT_TIMEOUT.0, MCP_CONNECT_TIMEOUT.1))
                    .map_err(HostedRefused::Unassembled)?,
                crate::browser_shell::found().as_deref(),
            )
            .map_err(HostedRefused::Unassembled)?,
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
    let Some(segments) = segments(line) else {
        return false;
    };
    segments
        .iter()
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty())
        .all(reads)
}

/// What a shell line is to the gate's background rule. #2415.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckLine {
    /// No `armada check` in it.
    NoCheck,
    /// A check, and beside it only a `cd` or what reads, such as `| tail -25`.
    Alone,
    /// A check beside something that writes, such as `&& git commit`, which a
    /// background run would hide the outcome of.
    Chained,
}

/// Whether `line` runs an `armada check` (or `armada manifest check`), and with what. A line
/// [`segments`] cannot read, such as one with a redirect, is left as it was written.
pub fn check_line(line: &str) -> CheckLine {
    let Some(segments) = segments(line) else {
        return CheckLine::NoCheck;
    };
    let segments: Vec<&str> = segments
        .iter()
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty())
        .collect();
    let (checks, rest): (Vec<&str>, Vec<&str>) = segments.into_iter().partition(|segment| checks(segment));
    match (checks.is_empty(), rest.iter().all(|segment| reads(segment))) {
        (true, _) => CheckLine::NoCheck,
        (false, true) => CheckLine::Alone,
        (false, false) => CheckLine::Chained,
    }
}

/// What a line puts in front of the command it runs: `taskpolicy -c utility armada check`.
const LAUNCHERS: &[&str] = &["taskpolicy", "nice", "env", "time"];

fn checks(segment: &str) -> bool {
    let mut words = segment.split_whitespace().peekable();
    // A launcher's own flags and values, and `NAME=value`, come before the program.
    let launched = words.peek().is_some_and(|first| LAUNCHERS.contains(first));
    while let Some(&word) = words.peek() {
        let ahead = word.contains('=') || (launched && word.rsplit('/').next() != Some("armada"));
        if !ahead {
            break;
        }
        words.next();
    }
    let program = words.next().unwrap_or_default();
    if program.rsplit('/').next() != Some("armada") {
        return false;
    }
    match words.next() {
        Some("check") => true,
        Some("manifest") => words.next() == Some("check"),
        _ => false,
    }
}


/// Redirects that write nothing: to the null device, or one descriptor onto another.
const HARMLESS_REDIRECTS: &[&str] = &[
    "2>>/dev/null",
    "&>/dev/null",
    "2>/dev/null",
    "1>/dev/null",
    ">/dev/null",
    "2>&1",
    "1>&2",
    ">&2",
    ">&1",
];

/// The length of a harmless redirect that `rest` opens as a word of its own.
fn harmless_redirect(rest: &str) -> Option<usize> {
    HARMLESS_REDIRECTS.iter().find_map(|form| {
        let after = rest.strip_prefix(form)?;
        after
            .chars()
            .next()
            .is_none_or(|next| next.is_whitespace() || matches!(next, '|' | ';' | '&' | ')'))
            .then_some(form.len())
    })
}

/// The line cut at `|`, `;`, `&` and newline outside quotes, with the harmless redirects
/// taken out. `None` for a quote left open, a redirect, or a substitution the shell would run:
/// a `>` inside quotes is text, and so is a backtick or `$(` inside single quotes.
fn segments(line: &str) -> Option<Vec<String>> {
    let mut segments = vec![String::new()];
    let mut quote: Option<char> = None;
    let mut word_start = true;
    let mut skip_to = 0;
    let mut chars = line.char_indices();
    while let Some((at, c)) = chars.next() {
        if at < skip_to {
            continue;
        }
        let current = segments.last_mut()?;
        let substitutes = c == '`' || line[at..].starts_with("$(");
        if c == '\\' && quote != Some('\'') {
            current.push(c);
            current.extend(chars.next().map(|(_, escaped)| escaped));
            word_start = false;
            continue;
        }
        match quote {
            Some('"') if substitutes => return None,
            Some(open) => {
                current.push(c);
                if c == open {
                    quote = None;
                }
            }
            None => {
                if word_start {
                    if let Some(len) = harmless_redirect(&line[at..]) {
                        skip_to = at + len;
                        continue;
                    }
                }
                if c == '>' || substitutes {
                    return None;
                }
                match c {
                    '|' | ';' | '&' | '\n' => {
                        segments.push(String::new());
                        word_start = true;
                        continue;
                    }
                    '"' | '\'' => quote = Some(c),
                    _ => {}
                }
                current.push(c);
                word_start = c.is_whitespace();
                continue;
            }
        }
        word_start = false;
    }
    quote.is_none().then_some(segments)
}

/// A `git` segment with the options that only say where to look taken off, so `git -C <path>
/// log` is read as `git log`. A path with a blank in it is left whole and reads as a write.
fn plain_git(segment: &str) -> std::borrow::Cow<'_, str> {
    let mut words = segment.split_whitespace().peekable();
    if words.next() != Some("git") {
        return segment.into();
    }
    while let Some(&word) = words.peek() {
        match word {
            "--no-pager" => {
                words.next();
            }
            "-C" => {
                words.next();
                if words.next().is_none() {
                    return segment.into();
                }
            }
            _ => break,
        }
    }
    std::iter::once("git").chain(words).collect::<Vec<_>>().join(" ").into()
}

/// Whether a `gh api` call only reads: no body, and a method of GET where one is named.
fn api_reads(segment: &str) -> bool {
    let words: Vec<&str> = segment.split_whitespace().collect();
    if words.iter().any(|word| {
        matches!(*word, "-f" | "-F" | "--field" | "--raw-field" | "--input")
            || word.starts_with("--field=")
            || word.starts_with("--raw-field=")
            || word.starts_with("--input=")
    }) {
        return false;
    }
    words.iter().enumerate().all(|(at, word)| match *word {
        "-X" | "--method" => words.get(at + 1).is_some_and(|method| method.eq_ignore_ascii_case("GET")),
        other => other
            .strip_prefix("--method=")
            .or_else(|| other.strip_prefix("-X"))
            .is_none_or(|method| method.eq_ignore_ascii_case("GET")),
    })
}

/// A `git` subcommand that reads only when its first argument says so, or when it has none:
/// `git worktree list` reads and `git worktree add` does not.
fn git_lists(segment: &str) -> bool {
    let mut words = segment.split_whitespace().skip(1);
    match (words.next(), words.next()) {
        (Some("worktree" | "stash"), Some("list")) => true,
        (Some("tag"), Some("-l" | "--list")) => true,
        (Some("tag" | "remote"), None) => true,
        (Some("remote"), Some("get-url" | "show")) => true,
        (Some("config"), Some("--get" | "--get-all" | "--list" | "-l")) => true,
        _ => false,
    }
}

/// Whether a `sed` only prints lines: `-n` and a script that is an address and `p`, such as
/// `sed -n 10,40p file`. Any other script can write a file (`w`) or run one (`e`).
fn sed_reads(segment: &str) -> bool {
    let words: Vec<&str> = segment.split_whitespace().skip(1).collect();
    let script = words.iter().find(|word| !word.starts_with('-'));
    let quiet = words.contains(&"-n");
    let only_flags = words
        .iter()
        .filter(|word| word.starts_with('-'))
        .all(|word| *word == "-n");
    quiet
        && only_flags
        && script.is_some_and(|script| {
            let script = script.trim_matches(['\'', '"']);
            script.strip_suffix('p').is_some_and(|address| {
                !address.is_empty() && address.chars().all(|c| c.is_ascii_digit() || c == ',' || c == '$')
            })
        })
}

/// One segment of a line.
fn reads(segment: &str) -> bool {
    // A group's `(` `{` and `)` `}` run nothing of their own: `(cd x && ls)`.
    let segment = segment.trim_start_matches(['(', '{', ' ']).trim_end_matches([')', '}', ' ']);
    if segment.is_empty() {
        return true;
    }
    let segment = &*plain_git(segment);
    if segment.starts_with("gh api ") {
        return api_reads(segment);
    }
    if segment.starts_with("sed ") {
        return sed_reads(segment);
    }
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
    if git_lists(segment) {
        return true;
    }
    // `L=~/x.log; grep y $L` sets a name and writes nothing: [`segments`] has already refused a
    // substitution, so the value is text.
    if segment.split_whitespace().all(|word| {
        word.split_once('=').is_some_and(|(name, _)| {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    }) {
        return true;
    }
    if segment.starts_with("find ") && (segment.contains("-delete") || segment.contains("-exec")) {
        return false;
    }
    // `armada` is a read as a whole, but giving a slot back changes the pool and the slot.
    if segment.split_whitespace().take(3).eq(["armada", "worktree", "release"]) {
        return false;
    }
    READS.iter().any(|read| named(read))
}

const READS: &[&str] = &[
    "cd",
    "pushd",
    "popd",
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
    "tr",
    "jq",
    "du",
    "df",
    "uname",
    "basename",
    "dirname",
    "realpath",
    "readlink",
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
    "git merge-base",
    "git remote -v",
    "git for-each-ref",
    "git ls-tree",
    "git cat-file",
    "git grep",
    "git shortlog",
    "git rev-list",
    "git show-ref",
    "git name-rev",
    "git ls-remote",
    "git diff-tree",
    "git check-ignore",
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
    ForkNotTold,
    SettingsNotWritten(String),
    Unassembled(SpawnConfigRefused),
}

impl fmt::Display for HostedRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostedRefused::DirectoryNotAbsolute { given } => write!(
                out,
                "the job's directory `{given}` is relative, so it would open wherever \
                 Fleet happened to be"
            ),
            HostedRefused::SessionNotPortable { given } => {
                write!(
                    out,
                    "the job id `{given}` is not one the agent CLI takes"
                )
            }
            HostedRefused::EffortNotPortable { given } => {
                write!(out, "the effort `{given}` is not one the agent CLI takes")
            }
            HostedRefused::ForkNotTold => out.write_str(
                "a fork would start without being told it is one, and would take the old \
                 job's background work for its own",
            ),
            HostedRefused::SettingsNotWritten(why) => {
                write!(out, "the job's settings would not encode: {why}")
            }
            HostedRefused::Unassembled(cause) => out.write_str(&cause.said()),
        }
    }
}

impl Error for HostedRefused {}
