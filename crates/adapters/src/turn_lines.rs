//! What each turn of a session cost, read from the transcripts the agent CLI keeps on disk.
//!
//! **Bytes enter through `ipc::decode`**, for `crate::transcript`'s reason. The shapes below are
//! that CLI's, which is why they are here. A line that does not decode is skipped: this reads for
//! figures, and a stream it cannot read yields none rather than a guess.
//!
//! **Names and counts only.** A tool is its name; a shell command is read for its label
//! ([`classify_bash`]) and dropped.
//!
//! **One message is several lines.** The CLI writes a message once for each content block as it
//! streams, under one id, so a read joins the lines of an id: the figures of the line with the most
//! output, and the calls in the order they were written. A message whose lines fall in two reads is
//! joined again by the store.
//!
//! **When each turn waited and ran** is read from the lines around a message: the user line before
//! it (the prompt or a tool's result) is when the model began, and a tool's result line is when that
//! call finished. A result whose call was read before it is returned alone, for the store to join.
//! Stamps only: a result's content is never read.

mod bash_use;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ipc::{BashUse, Instant, TurnUsage};
use serde::Deserialize;

pub use bash_use::classify_bash;

use crate::reading_in::SESSIONS;

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
    #[serde(default)]
    content: Content,
}

#[derive(Default, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Blocks(Vec<Block>),
    #[allow(dead_code)]
    Text(String),
}

impl Default for Content {
    fn default() -> Self {
        Content::Blocks(Vec::new())
    }
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    /// A call's own id, which its result names as `tool_use_id`.
    id: Option<String>,
    tool_use_id: Option<String>,
    name: Option<String>,
    input: Option<Input>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Input {
    Shell { command: String },
    Other(serde::de::IgnoredAny),
}

/// One message so far: the lines of an id, joined.
struct Joined {
    turn: TurnUsage,
    /// The first call of the message has been seen, whatever it was.
    first_call_seen: bool,
    times: TurnTimes,
    /// The ids of its calls, in the order written.
    calls: Vec<String>,
}

/// When a turn's model began and its tools ran, as RFC3339 stamps the CLI wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TurnTimes {
    pub message_id: String,
    /// The user line this message answers: the prompt or the last result before it.
    pub model_began: Option<String>,
    /// The first of its calls written.
    pub tools_began: Option<String>,
    /// The last result read for its calls. Absent while one has none in this read, or in any.
    pub tools_ended: Option<String>,
}

/// A tool's result, for a call that was read before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub at: String,
}

/// A call whose result is not in this read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenCall {
    pub tool_use_id: String,
    pub message_id: String,
}

/// The turns of the whole lines from an offset on, and where the read got to.
pub struct TurnRead {
    pub turns: Vec<TurnUsage>,
    /// One for each turn, in the same order.
    pub times: Vec<TurnTimes>,
    /// Results whose call is not in this read.
    pub late: Vec<ToolResult>,
    /// Calls of this read that have no result in it.
    pub open: Vec<OpenCall>,
    /// The stamp of the last user line read, or the one given where none was: the next read's
    /// `input_at`.
    pub input_at: Option<String>,
    /// The offset of the first byte not read: the end of the last whole line.
    pub next: u64,
}

/// Read the whole lines of `file` from `offset` on, up to about `budget` bytes, into turns.
/// `subagent` marks every turn as a subagent's. A line still being written is left for the next
/// read; an `offset` past the end of the file starts again from the top. `input_at` is the last
/// user line the read before this one reached, for a message whose prompt is not in this read.
pub fn read_turns(
    file: &Path,
    offset: u64,
    subagent: bool,
    budget: u64,
    input_at: Option<String>,
) -> std::io::Result<TurnRead> {
    let mut open = File::open(file)?;
    let from = if offset > open.metadata()?.len() { 0 } else { offset };
    open.seek(SeekFrom::Start(from))?;
    let mut reader = BufReader::new(open);
    let (mut next, mut raw) = (from, Vec::new());
    let mut joined: Vec<Joined> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    let mut input_at = if from == 0 { None } else { input_at };
    let mut results: Vec<ToolResult> = Vec::new();
    while next - from < budget {
        raw.clear();
        let read = reader.read_until(b'\n', &mut raw)?;
        if read == 0 || raw.last() != Some(&b'\n') {
            break;
        }
        next += read as u64;
        let assistant = raw.windows(11).any(|window| window == b"\"assistant\"")
            && raw.windows(7).any(|window| window == b"\"usage\"");
        if !assistant && !raw.windows(6).any(|window| window == b"\"user\"") {
            continue;
        }
        let Ok(line) = ipc::decode::<Line>("transcript line", &raw) else {
            continue;
        };
        let (Some(stamp), Some(message)) = (line.timestamp, line.message) else {
            continue;
        };
        // By the line's own type: a result that quotes a message reads as an assistant line to the filter.
        if line.kind.as_deref() == Some("user") {
            if let Content::Blocks(blocks) = message.content {
                for id in blocks.into_iter().filter(|block| block.kind == "tool_result").filter_map(|block| block.tool_use_id) {
                    results.push(ToolResult { tool_use_id: id, at: stamp.clone() });
                }
            }
            input_at = Some(stamp);
            continue;
        }
        let (Some(id), Some(model)) = (message.id, message.model) else {
            continue;
        };
        let usage = message.usage.unwrap_or_default();
        let slot = *at.entry(id.clone()).or_insert_with(|| {
            joined.push(Joined {
                times: TurnTimes { message_id: id.clone(), model_began: input_at.clone(), ..Default::default() },
                turn: TurnUsage {
                    message_id: id,
                    at: Instant::carried(stamp.clone()),
                    model: model.clone(),
                    input_tokens: 0,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 0,
                    output_tokens: 0,
                    tools: Vec::new(),
                    subagent,
                    bash_use: None,
                },
                first_call_seen: false,
                calls: Vec::new(),
            });
            joined.len() - 1
        });
        let one = &mut joined[slot];
        let output = usage.output_tokens.unwrap_or(0);
        if output >= one.turn.output_tokens {
            one.turn.at = Instant::carried(stamp.clone());
            one.turn.model = model;
            one.turn.input_tokens = usage.input_tokens.unwrap_or(0);
            one.turn.cache_creation_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
            one.turn.cache_read_tokens = usage.cache_read_input_tokens.unwrap_or(0);
            one.turn.output_tokens = output;
        }
        if let Content::Blocks(blocks) = message.content {
            for block in blocks.into_iter().filter(|block| block.kind == "tool_use") {
                let name = block.name.unwrap_or_default();
                if !one.first_call_seen {
                    one.first_call_seen = true;
                    if name == "Bash" {
                        one.turn.bash_use = Some(match block.input {
                            Some(Input::Shell { command }) => classify_bash(&command),
                            _ => BashUse::Other,
                        });
                    }
                }
                one.turn.tools.push(name);
                if one.times.tools_began.is_none() {
                    one.times.tools_began = Some(stamp.clone());
                }
                if let Some(call) = block.id {
                    one.calls.push(call);
                }
            }
        }
    }
    let mut calls: HashMap<String, usize> = HashMap::new();
    for (slot, one) in joined.iter().enumerate() {
        calls.extend(one.calls.iter().map(|call| (call.clone(), slot)));
    }
    let mut late = Vec::new();
    let mut answered: std::collections::HashSet<String> = std::collections::HashSet::new();
    for result in results {
        match calls.get(&result.tool_use_id) {
            Some(&slot) => {
                let ended = &mut joined[slot].times.tools_ended;
                if ended.as_ref().is_none_or(|held| *held < result.at) {
                    *ended = Some(result.at);
                }
                answered.insert(result.tool_use_id);
            }
            None => late.push(result),
        }
    }
    let mut open = Vec::new();
    for one in &joined {
        for call in one.calls.iter().filter(|call| !answered.contains(*call)) {
            open.push(OpenCall { tool_use_id: call.clone(), message_id: one.turn.message_id.clone() });
        }
    }
    let times = joined.iter().map(|one| one.times.clone()).collect();
    Ok(TurnRead { turns: joined.into_iter().map(|one| one.turn).collect(), times, late, open, input_at, next })
}

/// A transcript on disk, and whose it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscriptFile {
    pub path: PathBuf,
    /// The session it belongs to: for a subagent's, the session that started it.
    pub session_id: String,
    pub subagent: bool,
    pub size: u64,
    pub modified: SystemTime,
}

/// Every transcript under `home` written to since `since`.
pub fn transcripts_since(home: &str, since: SystemTime) -> Vec<TranscriptFile> {
    collect(home, None, Some(since))
}

/// Every transcript of session `id` under `home`: its own and its subagents'.
pub fn transcripts_of(home: &str, id: &str) -> Vec<TranscriptFile> {
    collect(home, Some(id), None)
}

/// **A session found under several project folders is counted once**, by its largest file: a
/// session moved into a slot is linked or copied into the slot's folder, and the copy that has the
/// most is the one that kept being written.
fn collect(home: &str, only: Option<&str>, since: Option<SystemTime>) -> Vec<TranscriptFile> {
    let mut best: HashMap<(String, Option<String>), TranscriptFile> = HashMap::new();
    let Ok(projects) = std::fs::read_dir(Path::new(home).join(SESSIONS)) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = projects.flatten().map(|entry| entry.path()).collect();
    folders.sort();
    for folder in folders {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if let Some(id) = name.strip_suffix(".jsonl") {
                if only.is_none_or(|only| only == id) {
                    offer(&mut best, &path, id, None, since);
                }
            } else if only.is_none_or(|only| only == name) {
                let Ok(agents) = std::fs::read_dir(path.join("subagents")) else {
                    continue;
                };
                for agent in agents.flatten() {
                    let file = agent.path();
                    let Some(stem) = file.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".jsonl")) else {
                        continue;
                    };
                    offer(&mut best, &file, name, Some(stem.to_string()), since);
                }
            }
        }
    }
    let mut files: Vec<TranscriptFile> = best.into_values().collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

fn offer(
    best: &mut HashMap<(String, Option<String>), TranscriptFile>,
    path: &Path,
    session: &str,
    agent: Option<String>,
    since: Option<SystemTime>,
) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    let Ok(modified) = meta.modified() else {
        return;
    };
    if !meta.is_file() || since.is_some_and(|since| modified < since) {
        return;
    }
    let found = TranscriptFile {
        path: path.to_path_buf(),
        session_id: session.to_string(),
        subagent: agent.is_some(),
        size: meta.len(),
        modified,
    };
    let held = best.entry((session.to_string(), agent)).or_insert_with(|| found.clone());
    if found.size > held.size {
        *held = found;
    }
}
