//! A terminal session's thread, drawn from the transcript file its agent CLI
//! keeps. `docs/concepts/session.md`, *A terminal session's thread*.
//!
//! **Rows are drawn and nothing is inferred from them.** A line the CLI wrote
//! becomes a message or a tool row in the shape a hosted session's thread has,
//! and a line this does not draw is skipped. The row's id is the line's own
//! `uuid`, so a line read twice replaces itself.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read as _, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use ipc::{Instant, SessionRow, SessionVoice};
use serde::Deserialize;

use adapter_traits::{DroneEvent, Speaker};

use crate::reading_in::SESSIONS;
use crate::transcript;

mod follow;
pub use follow::Followed;

/// Where a read got to, and what it drew.
pub struct Thread {
    pub rows: Vec<SessionRow>,
    /// The offset of the first byte not read: the end of the last whole line.
    pub next: u64,
}

/// The transcript of session `id` under `home`, wherever its directory is.
///
/// **Found by id and not by the directory the session started in**, because a
/// session that changed directory keeps writing the file it began.
pub fn find(home: &str, id: &str) -> Option<PathBuf> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    std::fs::read_dir(Path::new(home).join(SESSIONS))
        .ok()?
        .flatten()
        .map(|entry| entry.path().join(format!("{id}.jsonl")))
        .find(|file| file.is_file())
}

/// Make session `id`'s conversation reachable from `directory`, before the CLI
/// is started there with `--resume`.
///
/// The CLI looks for a resumed conversation in the project folder keyed by its
/// own working directory, so a session that moves (into a slot) finds nothing
/// and exits. The transcript is **hard-linked** into the new folder: one inode,
/// so what the CLI appends there is the file the reader finds. A copy stands in
/// where a link cannot be made. A `<id>/` folder (subagents) is symlinked, as
/// best effort. A conversation already there, or not found, is left alone.
pub fn bring_conversation_to(home: &str, id: &str, directory: &str) -> std::io::Result<()> {
    let Some(file) = find(home, id) else {
        return Ok(());
    };
    let folder = Path::new(home).join(SESSIONS).join(crate::reading_in::keyed(directory));
    let target = folder.join(format!("{id}.jsonl"));
    if target.exists() || file.parent() == Some(folder.as_path()) {
        return Ok(());
    }
    std::fs::create_dir_all(&folder)?;
    if std::fs::hard_link(&file, &target).is_err() {
        std::fs::copy(&file, &target)?;
    }
    if let Some(from) = file.parent().map(|dir| dir.join(id)).filter(|dir| dir.is_dir()) {
        let _ = std::os::unix::fs::symlink(from, folder.join(id));
    }
    Ok(())
}

/// The whole lines written from `offset` on, as text, with the offset they were read from (0 where
/// the file is shorter than `offset`) and the offset of the first byte not read.
fn whole_lines(file: &Path, offset: u64) -> std::io::Result<(String, u64, u64)> {
    let mut open = File::open(file)?;
    let length = open.metadata()?.len();
    let from = if offset > length { 0 } else { offset };
    open.seek(SeekFrom::Start(from))?;
    let mut bytes = Vec::new();
    open.read_to_end(&mut bytes)?;
    let whole = bytes.iter().rposition(|byte| *byte == b'\n').map_or(0, |at| at + 1);
    let text = String::from_utf8_lossy(&bytes[..whole]).into_owned();
    Ok((text, from, from + whole as u64))
}

/// The rows of the whole lines written from `offset` on. A line still being
/// written is left for the next read.
pub fn read_from(file: &Path, offset: u64) -> std::io::Result<Thread> {
    let (text, _, next) = whole_lines(file, offset)?;
    let rows = text.lines().flat_map(|line| drawn(line, false)).collect();
    Ok(Thread { rows, next })
}

/// A subagent's own transcript, drawn as a thread is.
pub struct Subagent {
    pub rows: Vec<SessionRow>,
    /// Its last turn ended and asked for nothing more.
    pub finished: bool,
    /// What it said last, once it has finished.
    pub report: Option<String>,
    /// The model it runs on and how much it had read at its last answer.
    pub gauge: Gauge,
}

/// What a subagent's last answer says about itself. Each is absent until an answer carried it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gauge {
    /// The model id the CLI wrote on the answer, as written.
    pub model: Option<String>,
    /// The input side of the answer's usage: what it sent, read from cache and wrote to it. **What
    /// that answer had in its window**, which is how the parent's figure is counted.
    pub tokens: Option<u64>,
}

/// The window a subagent's figure is drawn against. **The transcript names the model and never its
/// window**, so where the parent Session runs on the same model id and has reported a window, that
/// is the one. Otherwise a guess: the usual window, until the tokens outgrow it, which only the
/// long one can.
pub fn window_for(tokens: u64, model: Option<&str>, parent: Option<(&str, u64)>) -> u64 {
    match (model, parent) {
        (Some(model), Some((same, window))) if model == same => window,
        _ if tokens > USUAL_WINDOW => LONG_WINDOW,
        _ => USUAL_WINDOW,
    }
}

const USUAL_WINDOW: u64 = 200_000;
const LONG_WINDOW: u64 = 1_000_000;

/// Subagent `agent` of session `id`: the file the CLI keeps at
/// `<project>/<id>/subagents/agent-<agent>.jsonl`. **Only that path is ever read**; both names are
/// checked as `find` checks one.
///
/// **Every project folder is tried, not only the one `find` answers.** A session moved into a
/// slot has its transcript linked into the slot's folder, and the CLI writes its later subagents
/// there: the first folder holding `<id>.jsonl` can be the old one, with no subagents beside it,
/// and every such subagent read as running with a blank thread (13 Oct 2026).
pub fn find_subagent(home: &str, id: &str, agent: &str) -> Option<PathBuf> {
    let named = |name: &str| !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !named(id) || !named(agent) {
        return None;
    }
    let key = (home.to_string(), id.to_string(), agent.to_string());
    let kept = FOUND.lock().ok().and_then(|seen| seen.get(&key).cloned());
    if let Some(file) = kept.filter(|file| file.is_file()) {
        return Some(file);
    }
    let found = std::fs::read_dir(Path::new(home).join(SESSIONS))
        .ok()?
        .flatten()
        .map(|entry| entry.path().join(id).join("subagents").join(format!("agent-{agent}.jsonl")))
        .find(|file| file.is_file())?;
    if let Ok(mut seen) = FOUND.lock() {
        seen.insert(key, found.clone());
    }
    Some(found)
}

/// Where a subagent's file was found, so a repeat is a `stat` and not a listing of every project.
static FOUND: Mutex<BTreeMap<(String, String, String), PathBuf>> = Mutex::new(BTreeMap::new());

/// The whole of a subagent's transcript. It is small beside a session's, and a read from the start
/// keeps the answer one value.
pub fn read_subagent(file: &Path) -> std::io::Result<Subagent> {
    let bytes = std::fs::read(file)?;
    let whole = bytes.iter().rposition(|byte| *byte == b'\n').map_or(0, |at| at + 1);
    let text = String::from_utf8_lossy(&bytes[..whole]);
    let mut rows = Vec::new();
    let mut done = None;
    let mut gauge = Gauge::default();
    fold_subagent(&text, &mut rows, &mut done, &mut gauge);
    Ok(match done {
        Some(report) => Subagent { rows, finished: true, report, gauge },
        None => Subagent { rows, finished: false, report: None, gauge },
    })
}

/// Draw the whole lines of a subagent's transcript onto `rows`, carrying `done` and `gauge` from one
/// call to the next: they are the only things that cross a line.
fn fold_subagent(text: &str, rows: &mut Vec<SessionRow>, done: &mut Option<Option<String>>, gauge: &mut Gauge) {
    // **Finished where the last thing it did was finish**: a turn ended, or a hand-back. A
    // background subagent ends on `SubagentHandback` and never on `end_turn`, so a ledger read
    // only the one kept every finished subagent "running" (8 Oct 2026). A run cut off ends on the
    // CLI's interrupt line. Work after any of them, a resumed subagent, makes it running again.
    for line in text.lines() {
        let drew = drawn(line, true);
        if let Some(read) = gauged(line) {
            *gauge = read;
        }
        if let Some(said) = handed_back(line) {
            *done = Some(Some(said));
        } else if ended(line) {
            let report = drew.iter().rev().find_map(|row| match row {
                SessionRow::Message { from: SessionVoice::Agent, text, .. } => Some(text.clone()),
                _ => None,
            });
            *done = Some(report);
        } else if interrupted(line) {
            *done = Some(None);
        } else if worked(line) {
            *done = None;
        }
        rows.extend(drew);
    }
}

#[derive(Deserialize)]
struct Turn {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    message: Option<TurnMessage>,
}

#[derive(Deserialize)]
struct TurnMessage {
    #[serde(default)]
    content: Option<Vec<TurnPart>>,
}

#[derive(Deserialize)]
struct TurnPart {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<HandedBack>,
}

#[derive(Deserialize)]
struct HandedBack {
    #[serde(default)]
    message: Option<String>,
}

#[derive(Deserialize)]
struct Answer {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    message: Option<AnswerMessage>,
}

#[derive(Deserialize)]
struct AnswerMessage {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

/// What an assistant line says of the model and its window. **None for a line that read nothing**:
/// the CLI's own synthetic answers carry a zero usage and no model worth showing.
fn gauged(line: &str) -> Option<Gauge> {
    // Every line of a subagent passes here, and a user line has no usage.
    if !line.contains("\"usage\"") {
        return None;
    }
    let answer = ipc::decode::<Answer>("a transcript line", line.as_bytes()).ok()?;
    let message = answer.message.filter(|_| answer.kind == "assistant")?;
    let usage = message.usage?;
    let tokens = usage.input_tokens + usage.cache_creation_input_tokens + usage.cache_read_input_tokens;
    if tokens == 0 {
        return None;
    }
    let model = message.model.filter(|model| !model.is_empty() && !model.starts_with('<'));
    Some(Gauge { model, tokens: Some(tokens) })
}

/// The report a subagent handed back, where this line is its `SubagentHandback` call.
fn handed_back(line: &str) -> Option<String> {
    let turn = ipc::decode::<Turn>("a transcript line", line.as_bytes()).ok()?;
    turn.message?.content?.into_iter().find_map(|part| {
        (part.kind == "tool_use" && part.name.as_deref() == Some("SubagentHandback"))
            .then(|| part.input.and_then(|input| input.message).unwrap_or_default())
    })
}

/// An assistant line that goes on working: anything it says or calls but a hand-back.
fn worked(line: &str) -> bool {
    ipc::decode::<Turn>("a transcript line", line.as_bytes()).is_ok_and(|turn| turn.kind == "assistant")
}

/// What the CLI writes as a user line where a run was cut off: its parent's `TaskStop`, or the person
/// stopping it. No stop follows one.
const INTERRUPTED: &str = "[Request interrupted by user";

#[derive(Deserialize)]
struct Asked {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    message: Option<AskedMessage>,
}

#[derive(Deserialize)]
struct AskedMessage {
    #[serde(default)]
    content: Option<AskedContent>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AskedContent {
    Prose(String),
    Parts(Vec<AskedPart>),
}

#[derive(Deserialize)]
struct AskedPart {
    #[serde(default)]
    text: Option<String>,
}

/// A user line that is the CLI's interrupt: the run ended there.
fn interrupted(line: &str) -> bool {
    let Ok(asked) = ipc::decode::<Asked>("a transcript line", line.as_bytes()) else {
        return false;
    };
    asked.kind == "user"
        && match asked.message.and_then(|message| message.content) {
            Some(AskedContent::Prose(text)) => text.starts_with(INTERRUPTED),
            Some(AskedContent::Parts(parts)) => {
                parts.iter().any(|part| part.text.as_deref().is_some_and(|text| text.starts_with(INTERRUPTED)))
            }
            None => false,
        }
}

/// When the CLI wrote this line, as it spelled it.
fn stamp(line: &str) -> Option<String> {
    ipc::decode::<Turn>("a transcript line", line.as_bytes()).ok()?.timestamp
}

/// Whether subagent `agent` of session `id` has ended its turn, by the file the CLI keeps for it.
pub fn subagent_ended(home: &str, id: &str, agent: &str) -> bool {
    find_subagent(home, id, agent).is_some_and(|file| last_cached(&file).finished)
}

/// When subagent `agent` of session `id` last worked, where its transcript ends on work: that
/// line's own stamp. `None` where it ends finished, has no stamp, or no file is found.
pub fn subagent_working_at(home: &str, id: &str, agent: &str) -> Option<String> {
    let last = last_cached(&find_subagent(home, id, agent)?);
    last.at.filter(|_| !last.finished)
}

/// The line a subagent's transcript is decided by: whether it finished, and when it was written.
#[derive(Clone, Default)]
pub(crate) struct Last {
    pub(crate) finished: bool,
    pub(crate) at: Option<String>,
}

/// What a file said last time, keyed by its length and modified time: an unchanged file costs a
/// `stat`. A transcript is only ever appended to, so a file that grew has a new length.
static FINISHED: Mutex<BTreeMap<PathBuf, (u64, SystemTime, Last)>> = Mutex::new(BTreeMap::new());

fn last_cached(file: &Path) -> Last {
    let Ok(meta) = std::fs::metadata(file) else {
        return Last::default();
    };
    let Ok(modified) = meta.modified() else {
        return last_at_the_end(file).unwrap_or_default();
    };
    let key = (meta.len(), modified);
    let kept = FINISHED.lock().ok().and_then(|seen| seen.get(file).cloned());
    if let Some((len, at, last)) = kept {
        if (len, at) == key {
            return last;
        }
    }
    let Ok(last) = last_at_the_end(file) else {
        return Last::default();
    };
    if let Ok(mut seen) = FINISHED.lock() {
        seen.insert(file.to_path_buf(), (key.0, key.1, last.clone()));
    }
    last
}

/// What [`read_subagent`] decides `finished` by, found from the end: the last line that is a
/// hand-back, an ended turn or more work decides, and a read stops at it. Rows are not drawn.
pub(crate) fn last_at_the_end(file: &Path) -> std::io::Result<Last> {
    let mut file = File::open(file)?;
    let end = file.metadata()?.len();
    let mut width = 64 * 1024_u64;
    loop {
        let from = end.saturating_sub(width);
        file.seek(SeekFrom::Start(from))?;
        let mut bytes = Vec::with_capacity((end - from) as usize);
        (&mut file).take(end - from).read_to_end(&mut bytes)?;
        let whole = bytes.iter().rposition(|byte| *byte == b'\n').map_or(0, |at| at + 1);
        // A read that starts mid-file begins in the middle of a line, which is not one to decide by.
        let first = if from == 0 { 0 } else { bytes.iter().position(|byte| *byte == b'\n').map_or(whole, |at| at + 1) };
        let text = String::from_utf8_lossy(&bytes[first.min(whole)..whole]);
        for line in text.lines().rev() {
            if handed_back(line).is_some() || ended(line) || interrupted(line) {
                return Ok(Last { finished: true, at: stamp(line) });
            }
            if worked(line) {
                return Ok(Last { finished: false, at: stamp(line) });
            }
        }
        if from == 0 {
            return Ok(Last::default());
        }
        width *= 4;
    }
}

#[derive(Deserialize)]
struct Ending {
    #[serde(default)]
    message: Option<EndingMessage>,
}

#[derive(Deserialize)]
struct EndingMessage {
    #[serde(default)]
    stop_reason: Option<String>,
}

/// An assistant line that ends the turn without a tool to run.
fn ended(line: &str) -> bool {
    ipc::decode::<Ending>("a transcript line", line.as_bytes())
        .ok()
        .and_then(|one| one.message)
        .and_then(|message| message.stop_reason)
        .is_some_and(|reason| reason == "end_turn")
}

#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(rename = "isSidechain", default)]
    sidechain: bool,
    #[serde(rename = "isMeta", default)]
    meta: bool,
    /// The summary the CLI writes in place of a conversation it compacted. It
    /// is a user line and it is not the person's.
    #[serde(rename = "isCompactSummary", default)]
    compact: bool,
    #[serde(default)]
    origin: Option<Origin>,
}

#[derive(Deserialize)]
struct Origin {
    kind: String,
    #[serde(rename = "asUser", default)]
    as_user: bool,
}

impl Origin {
    /// **The person's words**: typed in the terminal, or submitted by a mod as
    /// the person's own, which is how Fleet's message arrives.
    fn is_the_person(&self) -> bool {
        self.kind == "human" || (self.kind == "plugin" && self.as_user)
    }
}

/// The rows one line draws, none for a line that is not the conversation.
fn drawn(line: &str, inside_agent: bool) -> Vec<SessionRow> {
    let Ok(envelope) = ipc::decode::<Envelope>("a transcript line", line.as_bytes()) else {
        return Vec::new();
    };
    if (envelope.sidechain && !inside_agent) || envelope.meta {
        return Vec::new();
    }
    let Some(uuid) = envelope.uuid.as_deref() else {
        return Vec::new();
    };
    let person = match envelope.kind.as_str() {
        "assistant" => false,
        "user" => match &envelope.origin {
            Some(origin) if !origin.is_the_person() => return Vec::new(),
            _ => true,
        },
        _ => return Vec::new(),
    };
    let at = Instant::carried(envelope.timestamp.unwrap_or_default());
    let mut rows = Vec::new();
    for event in transcript::read(line) {
        let id = match rows.len() {
            0 => uuid.to_string(),
            n => format!("{uuid}.{n}"),
        };
        match event {
            DroneEvent::Said { text, .. } if person && envelope.compact => {
                rows.push(SessionRow::Compaction {
                    id,
                    at: at.clone(),
                    text: text.trim().to_string(),
                });
            }
            DroneEvent::Said { text, by } if !text.trim().is_empty() => {
                let text = if person {
                    match worded(&text) {
                        Worded::Skip => continue,
                        Worded::Command(text) => {
                            rows.push(SessionRow::Command {
                                id,
                                at: at.clone(),
                                text,
                            });
                            continue;
                        }
                        Worded::Said(text) => text,
                    }
                } else {
                    match agent_said(&text) {
                        said if said.trim().is_empty() => continue,
                        said => said,
                    }
                };
                let from = match (person, by) {
                    (true, Speaker::Armada) => SessionVoice::You,
                    (false, Speaker::Drone) => SessionVoice::Agent,
                    _ => continue,
                };
                rows.push(SessionRow::Message {
                    id,
                    at: at.clone(),
                    from,
                    text,
                    files: Vec::new(),
                    tags: Vec::new(),
                    notice: None,
                });
            }
            DroneEvent::Called { tool, detail, .. } if !person => {
                let shown = detail.whole().unwrap_or(detail.shown()).to_string();
                let text = if shown.is_empty() {
                    tool
                } else {
                    format!("{tool} {shown}")
                };
                rows.push(SessionRow::Tool {
                    id,
                    at: at.clone(),
                    text,
                });
            }
            _ => {}
        }
    }
    rows
}

/// What a user line's text is, once the CLI's own wrapper tags are read.
enum Worded {
    /// Markup the CLI wrote for itself: a command's output, a caveat, a
    /// reminder, another session's hand-back. **Nothing of it reaches the thread.**
    Skip,
    /// A slash command or a `!` shell line, as the person typed it.
    Command(String),
    Said(String),
}

/// Tags whose whole line is the CLI's and not the conversation.
const NOT_SPOKEN: [&str; 7] = [
    "local-command-stdout",
    "local-command-stderr",
    "local-command-caveat",
    "bash-stdout",
    "system-reminder",
    "task-notification",
    "agent-message",
];

fn worded(text: &str) -> Worded {
    let text = without_reminders(text);
    let text = text.trim();
    if text.is_empty()
        || text.starts_with("Another Claude session sent a message")
        || NOT_SPOKEN.iter().any(|tag| text.starts_with(&format!("<{tag}")))
    {
        return Worded::Skip;
    }
    if let Some(shell) = inside(text, "bash-input") {
        return Worded::Command(format!("! {}", shell.trim()));
    }
    if let Some(name) = inside(text, "command-name") {
        let name = name.trim();
        let name = if name.starts_with('/') { name.to_string() } else { format!("/{name}") };
        let args = inside(text, "command-args").unwrap_or_default().trim();
        return Worded::Command(if args.is_empty() { name } else { format!("{name} {args}") });
    }
    // What was pasted is what the person said; the tag around it is not.
    match (text.starts_with("<pasted_content"), text.find('>')) {
        (true, Some(open)) => {
            let rest = &text[open + 1..];
            Worded::Said(rest.strip_suffix("</pasted_content>").unwrap_or(rest).trim().to_string())
        }
        _ => Worded::Said(text.to_string()),
    }
}

/// The text between `<tag>` and `</tag>`, where the line has both.
fn inside<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let from = text.find(&open)? + open.len();
    let to = text[from..].find(&format!("</{tag}>"))?;
    Some(&text[from..from + to])
}

/// The text with every `<system-reminder>` block taken out.
fn without_reminders(text: &str) -> String {
    without(text, "system-reminder")
}

/// What the agent said, less any `<task-notification>` block it wrote itself:
/// an agent sometimes ends a reply with a copy of the CLI's notice, which is not its prose.
pub fn agent_said(text: &str) -> String {
    without(text, "task-notification").trim_end().to_string()
}

/// The text with every `<tag>` block taken out.
fn without(text: &str, tag: &str) -> String {
    let (open_tag, close_tag) = (format!("<{tag}>"), format!("</{tag}>"));
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find(&open_tag) {
        out.push_str(&rest[..open]);
        match rest[open..].find(&close_tag) {
            Some(close) => rest = &rest[open + close + close_tag.len()..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}
