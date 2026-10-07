//! A terminal session's thread, drawn from the transcript file its agent CLI
//! keeps. `docs/concepts/session.md`, *A terminal session's thread*.
//!
//! **Rows are drawn and nothing is inferred from them.** A line the CLI wrote
//! becomes a message or a tool row in the shape a hosted session's thread has,
//! and a line this does not draw is skipped. The row's id is the line's own
//! `uuid`, so a line read twice replaces itself.

use std::fs::File;
use std::io::{Read as _, Seek, SeekFrom};
use std::path::{Path, PathBuf};

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

/// The rows of the whole lines written from `offset` on. A line still being
/// written is left for the next read.
pub fn read_from(file: &Path, offset: u64) -> std::io::Result<Thread> {
    let mut open = File::open(file)?;
    let length = open.metadata()?.len();
    let from = if offset > length { 0 } else { offset };
    open.seek(SeekFrom::Start(from))?;
    let mut bytes = Vec::new();
    open.read_to_end(&mut bytes)?;
    let whole = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |at| at + 1);
    let text = String::from_utf8_lossy(&bytes[..whole]);
    let rows = text.lines().flat_map(drawn).collect();
    Ok(Thread {
        rows,
        next: from + whole as u64,
    })
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
fn drawn(line: &str) -> Vec<SessionRow> {
    let Ok(envelope) = ipc::decode::<Envelope>("a transcript line", line.as_bytes()) else {
        return Vec::new();
    };
    if envelope.sidechain || envelope.meta {
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
        || NOT_SPOKEN
            .iter()
            .any(|tag| text.starts_with(&format!("<{tag}")))
    {
        return Worded::Skip;
    }
    if let Some(shell) = inside(text, "bash-input") {
        return Worded::Command(format!("! {}", shell.trim()));
    }
    if let Some(name) = inside(text, "command-name") {
        let name = name.trim();
        let name = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{name}")
        };
        let args = inside(text, "command-args").unwrap_or_default().trim();
        return Worded::Command(if args.is_empty() {
            name
        } else {
            format!("{name} {args}")
        });
    }
    // What was pasted is what the person said; the tag around it is not.
    match (text.starts_with("<pasted_content"), text.find('>')) {
        (true, Some(open)) => {
            let rest = &text[open + 1..];
            Worded::Said(
                rest.strip_suffix("</pasted_content>")
                    .unwrap_or(rest)
                    .trim()
                    .to_string(),
            )
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
