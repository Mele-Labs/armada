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

/// The rows of the whole lines written from `offset` on. A line still being
/// written is left for the next read.
pub fn read_from(file: &Path, offset: u64) -> std::io::Result<Thread> {
    let mut open = File::open(file)?;
    let length = open.metadata()?.len();
    let from = if offset > length { 0 } else { offset };
    open.seek(SeekFrom::Start(from))?;
    let mut bytes = Vec::new();
    open.read_to_end(&mut bytes)?;
    let whole = bytes.iter().rposition(|byte| *byte == b'\n').map_or(0, |at| at + 1);
    let text = String::from_utf8_lossy(&bytes[..whole]);
    let rows = text.lines().flat_map(|line| drawn(line, false)).collect();
    Ok(Thread {
        rows,
        next: from + whole as u64,
    })
}

/// A subagent's own transcript, drawn as a thread is.
pub struct Subagent {
    pub rows: Vec<SessionRow>,
    /// Its last turn ended and asked for nothing more.
    pub finished: bool,
    /// What it said last, once it has finished.
    pub report: Option<String>,
}

/// Subagent `agent` of session `id`: the file the CLI keeps beside the session's own, at
/// `<project>/<id>/subagents/agent-<agent>.jsonl`. **Only that path is ever read**; both names are
/// checked as `find` checks one.
pub fn find_subagent(home: &str, id: &str, agent: &str) -> Option<PathBuf> {
    if agent.is_empty() || !agent.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    let file = find(home, id)?.with_extension("").join("subagents").join(format!("agent-{agent}.jsonl"));
    file.is_file().then_some(file)
}

/// The whole of a subagent's transcript. It is small beside a session's, and a read from the start
/// keeps the answer one value.
pub fn read_subagent(file: &Path) -> std::io::Result<Subagent> {
    let bytes = std::fs::read(file)?;
    let whole = bytes.iter().rposition(|byte| *byte == b'\n').map_or(0, |at| at + 1);
    let text = String::from_utf8_lossy(&bytes[..whole]);
    let mut rows = Vec::new();
    // **Finished where the last thing it did was finish**: a turn ended, or a hand-back. A
    // background subagent ends on `SubagentHandback` and never on `end_turn`, so a ledger read
    // only the one kept every finished subagent "running" (8 Oct 2026). Work after either, a
    // resumed subagent, makes it running again.
    let mut done: Option<Option<String>> = None;
    for line in text.lines() {
        let drew = drawn(line, true);
        if let Some(said) = handed_back(line) {
            done = Some(Some(said));
        } else if ended(line) {
            let report = drew.iter().rev().find_map(|row| match row {
                SessionRow::Message { from: SessionVoice::Agent, text, .. } => Some(text.clone()),
                _ => None,
            });
            done = Some(report);
        } else if worked(line) {
            done = None;
        }
        rows.extend(drew);
    }
    Ok(match done {
        Some(report) => Subagent { rows, finished: true, report },
        None => Subagent { rows, finished: false, report: None },
    })
}

#[derive(Deserialize)]
struct Turn {
    #[serde(rename = "type", default)]
    kind: String,
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

/// Whether subagent `agent` of session `id` has ended its turn, by the file the CLI keeps for it.
pub fn subagent_ended(home: &str, id: &str, agent: &str) -> bool {
    find_subagent(home, id, agent).is_some_and(|file| finished_cached(&file))
}

/// What a file said last time, keyed by its length and modified time: an unchanged file costs a
/// `stat`. A transcript is only ever appended to, so a file that grew has a new length.
static FINISHED: Mutex<BTreeMap<PathBuf, (u64, SystemTime, bool)>> = Mutex::new(BTreeMap::new());

pub(crate) fn finished_cached(file: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(file) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return finished_at_the_end(file).unwrap_or(false);
    };
    let key = (meta.len(), modified);
    let kept = FINISHED.lock().ok().and_then(|seen| seen.get(file).copied());
    if let Some((len, at, finished)) = kept {
        if (len, at) == key {
            return finished;
        }
    }
    let Ok(finished) = finished_at_the_end(file) else {
        return false;
    };
    if let Ok(mut seen) = FINISHED.lock() {
        seen.insert(file.to_path_buf(), (key.0, key.1, finished));
    }
    finished
}

/// What [`read_subagent`] decides `finished` by, found from the end: the last line that is a
/// hand-back, an ended turn or more work decides, and a read stops at it. Rows are not drawn.
pub(crate) fn finished_at_the_end(file: &Path) -> std::io::Result<bool> {
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
            if handed_back(line).is_some() || ended(line) {
                return Ok(true);
            }
            if worked(line) {
                return Ok(false);
            }
        }
        if from == 0 {
            return Ok(false);
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
                    text
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
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find("<system-reminder>") {
        out.push_str(&rest[..open]);
        match rest[open..].find("</system-reminder>") {
            Some(close) => rest = &rest[open + close + "</system-reminder>".len()..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}
