//! How a folder is run, read from the commands its past sessions ran.
//!
//! A session's record holds every `Bash` call the agent made, with the folder it was in and the
//! time, and the result that came back. **The reading is two steps and only the first touches a
//! file.** [`Reading`] turns record lines into [`Record`]s, keeping a result only for a call it
//! kept; [`derive`] is a pure function over those records, so a fixture of a few lines tests the
//! whole of it.
//!
//! A command is recognised by plain rules on its words (`rules.rs`) and nothing else. **One result
//! covers a whole line of commands**, so when `a && b` fails neither is known to have worked, and
//! both count as failed. A command whose every run failed is left out.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use core_model::Timestamp;
use serde::Deserialize;

mod missing;
mod rules;

pub use missing::missing_commands;

/// Tags the facts as read from sessions.
pub const SOURCE: &str = "claude_code_sessions";
/// Tags the facts as seen in a Job Fleet hosted: the same records, fed from its live stream.
pub const LIVE_SOURCE: &str = "jobs";

/// Records read from one session's file, in bytes.
pub const MOST_BYTES_PER_RECORD: u64 = 8 * 1024 * 1024;
/// A line longer than this is a pasted file or an image, and is skipped unread.
const MOST_LINE_BYTES: u64 = 512 * 1024;
/// How many of a folder's newest sessions are read.
pub const MOST_SESSIONS: usize = 30;
/// How much of a result is kept: enough for the address a server printed.
const KEPT_OF_A_RESULT: usize = 2000;

/// What a command was run for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunKind {
    Tests,
    Lint,
    Build,
    DevServer,
    Typecheck,
    Format,
}

/// One thing a repository was seen to run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunFact {
    /// Relative to the repository root, `/`-separated; `.` is the root.
    pub workspace: String,
    pub kind: RunKind,
    pub command: String,
    pub port: Option<u16>,
    /// Whether the latest run of it worked.
    pub succeeded: bool,
    /// Milliseconds since the epoch.
    pub last_ran: u64,
}

/// What reading kept of a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    Call {
        id: String,
        command: String,
        /// The folder the agent was in, as the line says.
        cwd: Option<String>,
        at_ms: u64,
        background: bool,
    },
    Done {
        id: String,
        failed: bool,
        /// The command's exit code, where what was printed says one. A failure the harness flagged
        /// without a code (a refused call) has none.
        exit: Option<i32>,
        /// The start of what was printed.
        said: String,
    },
}

#[derive(Deserialize)]
struct Line {
    cwd: Option<String>,
    timestamp: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    content: Option<Content>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Text(#[allow(dead_code)] String),
    Blocks(Vec<Block>),
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Block {
    ToolUse {
        id: String,
        name: String,
        input: Input,
    },
    ToolResult {
        tool_use_id: String,
        is_error: Option<bool>,
        content: Option<Printed>,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct Input {
    command: Option<String>,
    run_in_background: Option<bool>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Printed {
    Text(String),
    Parts(Vec<Part>),
}

#[derive(Deserialize)]
struct Part {
    text: Option<String>,
}

/// Record lines in, [`Record`]s out.
#[derive(Default)]
pub struct Reading {
    kept_calls: HashSet<String>,
    pub records: Vec<Record>,
}

impl Reading {
    /// One line. A line that is not valid, or holds no `Bash` call or the result of one, is
    /// skipped. `fallback_ms` stands for a line with no time of its own.
    pub fn feed(&mut self, line: &[u8], fallback_ms: u64) {
        if !(contains(line, b"\"tool_use\"") || contains(line, b"\"tool_result\"")) {
            return;
        }
        let Ok(read) = ipc::decode::<Line>("a session record line", line) else {
            return;
        };
        let at_ms = read
            .timestamp
            .as_deref()
            .and_then(|text| Timestamp::from_rfc3339(text.to_string()).epoch_millis())
            .map_or(fallback_ms, |millis| millis.max(0) as u64);
        let Some(Content::Blocks(blocks)) = read.message.and_then(|message| message.content) else {
            return;
        };
        for block in blocks {
            match block {
                Block::ToolUse { id, name, input } if name == "Bash" => {
                    let Some(command) = input.command else {
                        continue;
                    };
                    self.kept_calls.insert(id.clone());
                    self.records.push(Record::Call {
                        id,
                        command,
                        cwd: read.cwd.clone(),
                        at_ms,
                        background: input.run_in_background.unwrap_or(false),
                    });
                }
                Block::ToolResult { tool_use_id, is_error, content } if self.kept_calls.contains(&tool_use_id) => {
                    let said = match content {
                        Some(Printed::Text(text)) => text,
                        Some(Printed::Parts(parts)) => {
                            parts.into_iter().filter_map(|part| part.text).collect::<Vec<_>>().join("\n")
                        }
                        None => String::new(),
                    };
                    let exit = exit_code(&said);
                    // A pipe hides the exit of what is before it, so a command the shell lacks still reads as run.
                    let failed = is_error.unwrap_or(false)
                        || said.starts_with("Exit code ")
                        || !missing::named_in(&said).is_empty();
                    self.records.push(Record::Done { id: tool_use_id, failed, exit, said: kept(said) });
                }
                _ => {}
            }
        }
    }

    /// A session's file, to the byte cap. `fallback_ms` is the file's own last write.
    pub fn feed_file(&mut self, path: &Path, fallback_ms: u64) {
        let Ok(file) = File::open(path) else {
            return;
        };
        let mut lines = BufReader::new(file.take(MOST_BYTES_PER_RECORD));
        let mut line = Vec::new();
        loop {
            line.clear();
            // One byte past the cap tells a long line from one that fits.
            let Ok(read) = (&mut lines).take(MOST_LINE_BYTES + 1).read_until(b'\n', &mut line) else {
                return;
            };
            if read == 0 {
                return;
            }
            if line.len() as u64 > MOST_LINE_BYTES {
                // Drop the rest of it.
                let mut rest = Vec::new();
                if lines.read_until(b'\n', &mut rest).is_err() {
                    return;
                }
                continue;
            }
            self.feed(&line, fallback_ms);
        }
    }
}

/// The code a failed command's result opens with: `Exit code 2`.
fn exit_code(said: &str) -> Option<i32> {
    let after = said.strip_prefix("Exit code ")?;
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit() || *c == '-').collect();
    digits.parse().ok()
}

fn kept(mut text: String) -> String {
    if text.len() > KEPT_OF_A_RESULT {
        let mut end = KEPT_OF_A_RESULT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

/// One run of a recognised command.
struct Ran {
    at_ms: u64,
    failed: bool,
    port: Option<u16>,
}

/// What a folder was run with, from its records. `root` is what workspaces are relative to; a call
/// outside it is left out.
pub fn derive(records: &[Record], root: &str) -> Vec<RunFact> {
    let done: HashMap<&str, (bool, &str)> = records
        .iter()
        .filter_map(|record| match record {
            Record::Done { id, failed, said, .. } => Some((id.as_str(), (*failed, said.as_str()))),
            Record::Call { .. } => None,
        })
        .collect();
    let mut runs: BTreeMap<(String, RunKind, String), Vec<Ran>> = BTreeMap::new();
    for record in records {
        let Record::Call { id, command, cwd, at_ms, background } = record else {
            continue;
        };
        let Some(start) = cwd.as_deref() else {
            continue;
        };
        // No result means the call never came back: not known to have worked.
        let (failed, said) = done.get(id.as_str()).copied().unwrap_or((true, ""));
        for step in steps(command, start) {
            let Some(verdict) = rules::classify(&step.words) else {
                continue;
            };
            let Some(workspace) = relative(root, &step.cwd) else {
                continue;
            };
            let port = match verdict.kind {
                RunKind::DevServer => step.port.or_else(|| port_named(&step.words)).or_else(|| port_printed(said)),
                _ => None,
            };
            if verdict.needs_proof && !*background && port.is_none() {
                continue;
            }
            // A server is stopped to be left, so it has worked if it said where it listens.
            let failed = failed && !(verdict.kind == RunKind::DevServer && port.is_some());
            runs.entry((workspace, verdict.kind, step.words.join(" ")))
                .or_default()
                .push(Ran { at_ms: *at_ms, failed, port });
        }
    }
    runs.into_iter()
        .filter(|(_, ran)| ran.iter().any(|run| !run.failed))
        .map(|((workspace, kind, command), ran)| {
            let latest = ran.iter().max_by_key(|run| run.at_ms).expect("at least one run");
            RunFact {
                workspace,
                kind,
                command,
                port: ran.iter().filter(|run| !run.failed).filter_map(|run| run.port).next_back(),
                succeeded: !latest.failed,
                last_ran: latest.at_ms,
            }
        })
        .collect()
}

/// One command of a line, with the folder it ran in.
struct Step {
    words: Vec<String>,
    cwd: String,
    /// From a `PORT=3000` in front of the command.
    port: Option<u16>,
}

/// The commands of a shell line in order, each with the folder it ran in. `cd` moves the folder and
/// is not a step; what a pipe feeds and where output goes are not part of the command.
fn steps(line: &str, start: &str) -> Vec<Step> {
    let mut cwd = start.to_string();
    let mut found = Vec::new();
    for piece in pieces(line) {
        let mut words = words_of(&piece);
        let mut env_port = None;
        while words.first().is_some_and(|word| word.contains('=') && !word.starts_with('-') && !word.contains('/')) {
            let set = words.remove(0);
            if let Some(port) = set.strip_prefix("PORT=").and_then(|port| port.parse::<u16>().ok()) {
                env_port = Some(port);
            }
        }
        let Some(first) = words.first() else {
            continue;
        };
        if first == "cd" {
            if let Some(target) = words.get(1) {
                cwd = moved(&cwd, target);
            }
            continue;
        }
        // `make -C dir` and `npm --prefix dir run x` run in `dir`.
        let mut here = cwd.clone();
        if let Some(at) = words.iter().position(|word| matches!(word.as_str(), "-C" | "--prefix" | "--cwd")) {
            if let Some(dir) = words.get(at + 1) {
                here = moved(&cwd, dir);
            }
        }
        found.push(Step { words, cwd: here, port: env_port });
    }
    found
}

/// A line cut at `&&`, `||`, `;` and newlines outside quotes, each cut at its first pipe.
fn pieces(line: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut piped = false;
    let chars: Vec<char> = line.chars().collect();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        match quote {
            Some(open) => {
                if c == open {
                    quote = None;
                }
                if !piped {
                    current.push(c);
                }
            }
            None => match (c, next) {
                ('\'' | '"', _) => {
                    quote = Some(c);
                    if !piped {
                        current.push(c);
                    }
                }
                ('&', Some('&')) | ('|', Some('|')) => {
                    pieces.push(std::mem::take(&mut current));
                    piped = false;
                    at += 1;
                }
                (';' | '\n', _) => {
                    pieces.push(std::mem::take(&mut current));
                    piped = false;
                }
                ('|', _) => piped = true,
                _ => {
                    if !piped {
                        current.push(c);
                    }
                }
            },
        }
        at += 1;
    }
    pieces.push(current);
    pieces
}

/// A piece's words: quotes removed, redirections and a leading `(` dropped.
fn words_of(piece: &str) -> Vec<String> {
    piece
        .trim_start_matches(|c: char| c == '(' || c.is_whitespace())
        .split_whitespace()
        .map(|word| word.trim_matches(|c| c == '\'' || c == '"').to_string())
        .filter(|word| !word.is_empty() && !is_redirection(word))
        .collect()
}

fn is_redirection(word: &str) -> bool {
    let digits = word.trim_start_matches(|c: char| c.is_ascii_digit());
    digits.starts_with('>') || digits.starts_with("&>") || word == ")"
}

/// `cwd` after `cd target`. A target the shell would expand (`~`, `-`, `$X`) leaves it where it was.
fn moved(cwd: &str, target: &str) -> String {
    if target.starts_with('~') || target == "-" || target.contains('$') {
        return cwd.to_string();
    }
    let mut parts: Vec<&str> = match target.starts_with('/') {
        true => Vec::new(),
        false => cwd.split('/').filter(|part| !part.is_empty()).collect(),
    };
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    format!("/{}", parts.join("/"))
}

/// `cwd` below `root`, or `.` at it. Outside it is `None`.
fn relative(root: &str, cwd: &str) -> Option<String> {
    let root = root.trim_end_matches('/');
    let below = cwd.trim_end_matches('/').strip_prefix(root)?;
    match below {
        "" => Some(".".to_string()),
        below => below.strip_prefix('/').map(str::to_string),
    }
}

/// A port the command line gives: `--port 3000`, `--port=3000`, `PORT=3000`, `-p 3000`, or a
/// bare number or `host:3000` as the last word (`python -m http.server 8000`).
fn port_named(words: &[String]) -> Option<u16> {
    let number = |text: &str| text.parse::<u16>().ok().filter(|port| *port >= 80);
    for (at, word) in words.iter().enumerate() {
        let after = words.get(at + 1).map(String::as_str).unwrap_or("");
        if matches!(word.as_str(), "--port" | "-p" | "-P") {
            if let Some(port) = number(after) {
                return Some(port);
            }
        }
        if let Some(port) = word.strip_prefix("--port=").and_then(number) {
            return Some(port);
        }
    }
    let last = words.last()?;
    number(last).or_else(|| last.rsplit_once(':').and_then(|(_, port)| number(port)))
}

/// The first address a server printed it was listening on.
fn port_printed(said: &str) -> Option<u16> {
    for marker in ["localhost:", "127.0.0.1:", "0.0.0.0:", "[::1]:", "[::]:", "port ", "Port "] {
        for (at, _) in said.match_indices(marker) {
            let digits: String =
                said[at + marker.len()..].chars().take_while(char::is_ascii_digit).collect();
            if let Some(port) = digits.parse::<u16>().ok().filter(|port| *port >= 80) {
                return Some(port);
            }
        }
    }
    None
}
