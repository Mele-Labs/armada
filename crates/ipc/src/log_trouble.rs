//! The warnings and refusals in Fleet's log, as names only, for the debug report.
//!
//! **Here because a log line is bytes entering the process**, and `serde_json::from_*` may appear only
//! in `ipc` and `store`. A line keeps its time, level and target and a `code` field where one is
//! shaped like a code. Its `msg` and every other field are free text, a sentence or a path, and are
//! never read, so none can reach the report. A line that is not JSON is left out: it has no level.

use serde::Deserialize;
use serde_json::Value;

use crate::debug_report::Trouble;

/// The lines read from the end of the log.
pub const LINES_READ: usize = 200;

#[derive(Deserialize)]
struct LogLine {
    ts: String,
    level: String,
    #[serde(default)]
    component: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    fields: Option<Fields>,
}

#[derive(Deserialize)]
struct Fields {
    #[serde(default)]
    code: Option<Value>,
}

/// A name a code or target may be: lowercase letters, digits and `_ . -`, short. Anything else is a
/// sentence or a value somebody typed, and is dropped.
fn is_a_name(text: &str) -> bool {
    (1..=64).contains(&text.len()) && text.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-'))
}

/// `HH:MM:SS` of an RFC3339 instant, or `None` where it is not shaped like one.
fn time_of(ts: &str) -> Option<String> {
    let clock = ts.get(11..19)?;
    let shaped = ts.as_bytes().get(10) == Some(&b'T')
        && clock.bytes().enumerate().all(|(at, byte)| if at == 2 || at == 5 { byte == b':' } else { byte.is_ascii_digit() });
    shaped.then(|| clock.to_string())
}

/// One log line's trouble, where it is a warning or an error.
fn trouble_of(line: &str) -> Option<Trouble> {
    let read: LogLine = crate::decode("a log line", line.as_bytes()).ok()?;
    let level = match read.level.to_ascii_lowercase().as_str() {
        "warn" => "WARN",
        "error" => "ERROR",
        _ => return None,
    };
    let target = read
        .target
        .filter(|target| is_a_name(target))
        .or(read.component.filter(|component| is_a_name(component)))
        .unwrap_or_else(|| "unknown".to_string());
    let code = read
        .fields
        .and_then(|fields| fields.code)
        .and_then(|code| code.as_str().map(str::to_string))
        .filter(|code| is_a_name(code));
    Some(Trouble { time: time_of(&read.ts)?, at: read.ts, level: level.to_string(), target, code })
}

/// The trouble in the last [`LINES_READ`] lines of `log` that is no older than `since_millis`
/// (epoch milliseconds), oldest first. **A line is recent by its own instant**, so a log nobody has
/// written to lately is never read as trouble from today.
pub fn trouble_in(log: &str, since_millis: i64) -> Vec<Trouble> {
    let lines: Vec<&str> = log.lines().collect();
    let from = lines.len().saturating_sub(LINES_READ);
    lines[from..]
        .iter()
        .filter_map(|line| trouble_of(line))
        .filter(|one| core_model::Timestamp::from_rfc3339(one.at.as_str()).epoch_millis().is_some_and(|at| at >= since_millis))
        .collect()
}
