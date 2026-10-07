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
    let whole = bytes.iter().rposition(|byte| *byte == b'\n').map_or(0, |at| at + 1);
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
            DroneEvent::Said { text, by } if !text.trim().is_empty() => {
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
