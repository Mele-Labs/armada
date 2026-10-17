//! What a Drone said, written down: the Job's log and its Drones' transcript files, which are
//! read back and no longer written. Nothing here runs a Drone.
//!
//! # Rows, never the wire shape
//!
//! A row is a `DroneEvent`, which is also the redaction: spike 3's raw opening
//! event carried the operator's home path and whole tool inventory, while
//! `DroneEvent::Started` carries a session, a model and a count.

mod backfill;

/// Renaming what an older Fleet wrote under a ULID.
mod migrating;
mod refused;
mod row;

use std::io;
use std::path::{Path, PathBuf};

use core_model::{DroneId, Envelope};

use row::Line;

pub use backfill::{arguments, history, last_heard, refusals, HISTORY, REFUSALS};
pub use migrating::{rekeyed, Rekeyed};
pub use refused::refused_in;

/// Where one Job's transcripts live: a directory per Job, a file per Drone.
///
/// **Named by the handle, like the four beside it** — `asked::briefs_dir`,
/// `check_output::checks_dir`, `keeping::deliverables_dir` and the worktree
/// (the worktree alone stays under the repository; the other four, and this,
/// are under `crate::records::root`). A Job has more than one Drone over its
/// life, so the Drone stays the file name and the Job becomes the directory;
/// keyed by the Drone's own minted id alone, nothing here could be reached
/// from the one id a person can read.
pub fn transcripts_dir(records_root: &str, handle: &str) -> PathBuf {
    Path::new(records_root)
        .join(".armada")
        .join("transcripts")
        .join(handle)
}

/// Where a Drone's rows land, under the Job it is working.
pub fn transcript_of(records_root: &str, handle: &str, drone: &DroneId) -> PathBuf {
    transcripts_dir(records_root, handle).join(format!("{}.jsonl", drone.as_str()))
}

/// Where a Job's log lines land. The path `docs/concepts/log-envelope.md`
/// names, and until now nothing wrote it.
///
/// **The handle, not the id**, for [`transcripts_dir`]'s reason.
pub fn log_of(records_root: &str, handle: &str) -> PathBuf {
    Path::new(records_root)
        .join(".armada")
        .join("logs")
        .join(format!("{handle}.jsonl"))
}

/// Write one Fleet-authored line into a Job's log.
///
/// The Job log is otherwise written by the transcript's own writer task. This
/// is the other author: something Fleet observed about a Job that no Drone
/// event carries — a step editing outside its declared scope is the first.
pub fn note(records_root: &str, handle: &str, envelope: &Envelope) -> Result<(), io::Error> {
    let mut log = appending(&log_of(records_root, handle))?;
    write_line(&mut log, envelope)
}

/// Write one Fleet-authored line to Fleet's standard error, for what belongs to no Job and so has no log to join.
///
/// The same line the Job log carries, so a query and the redactor reach it; a stream that will not take it
/// loses the line and nothing else.
pub fn note_aloud(envelope: &Envelope) {
    use std::io::Write;
    if let Ok(line) = encoded(&Line::from(envelope)) {
        let _ = std::io::stderr().write_all(line.as_bytes());
    }
}

/// Open for appending, making the directory if it is not there.
///
/// Append rather than truncate: a Job log outlives any one Drone, and a
/// transcript is named by an id nothing else will ever be named by.
fn appending(path: &Path) -> Result<std::fs::File, io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}

fn write_line(file: &mut std::fs::File, envelope: &Envelope) -> Result<(), io::Error> {
    use std::io::Write;
    file.write_all(encoded(&Line::from(envelope))?.as_bytes())
}

/// One JSON object and the newline after it, through `ipc`'s encoder rather
/// than a second one here.
fn encoded<T: serde::Serialize>(value: &T) -> Result<String, io::Error> {
    let mut line =
        ipc::encode(value).map_err(|why| io::Error::new(io::ErrorKind::InvalidData, why.why))?;
    line.push('\n');
    Ok(line)
}
