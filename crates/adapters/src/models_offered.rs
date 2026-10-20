//! The models the harness itself offers, asked rather than written down.
//!
//! The CLI answers a first control line with its own list: one row per alias, and
//! one per pinned version. No prompt is sent and no token is spent, so asking is
//! a process that starts, answers in well under a second and is killed.
//!
//! **Bytes enter through `ipc::decode`**, for `crate::transcript`'s reason.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::harness::HeadlessAgent;

/// The one control request this module sends, and the id its answer comes back under.
const ASK: &str = r#"{"type":"control_request","request_id":"models","request":{"subtype":"initialize"}}"#;
const ASKED_AS: &str = "models";

/// One model the harness offers by an alias, which follows the vendor's newest
/// version. A pinned older version is not one of these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HarnessModel {
    /// What `--model` takes: the alias, with no version in it.
    pub name: String,
    /// The harness's own label for it, version included.
    pub display: String,
    /// The version the alias points at today.
    pub version: String,
    /// The harness's own words for what it is for.
    pub description: String,
    /// The efforts it takes, weakest first. Empty where it takes none.
    pub efforts: Vec<String>,
}

#[derive(Deserialize)]
struct Line {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    response: Option<Reply>,
}

#[derive(Deserialize)]
struct Reply {
    #[serde(default)]
    request_id: String,
    #[serde(default)]
    subtype: String,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    response: Option<Initialized>,
}

#[derive(Deserialize)]
struct Initialized {
    #[serde(default)]
    models: Option<Vec<Offered>>,
}

#[derive(Deserialize)]
struct Offered {
    value: String,
    #[serde(default, rename = "resolvedModel")]
    resolved: String,
    #[serde(default, rename = "displayName")]
    display: String,
    #[serde(default)]
    description: String,
    #[serde(default, rename = "supportedEffortLevels")]
    efforts: Vec<String>,
}

/// What one line of the harness's output says about its models: `None` for a
/// line that is not the answer, and the aliases or the reason for one that is.
pub(crate) fn models_in(line: &[u8]) -> Option<Result<Vec<HarnessModel>, String>> {
    let line = ipc::decode::<Line>("the harness's answer", line).ok()?;
    let reply = line.response.filter(|_| line.kind == "control_response")?;
    if reply.request_id != ASKED_AS {
        return None;
    }
    if reply.subtype == "error" {
        return Some(Err(format!(
            "it refused the question: {}",
            reply.error.unwrap_or_default()
        )));
    }
    let Some(offered) = reply.response.and_then(|answer| answer.models) else {
        return Some(Err(String::from("its answer named no models")));
    };
    let mut kept: Vec<HarnessModel> = Vec::new();
    for model in offered {
        // `default` is whichever alias the account starts on, so it repeats a row
        // below; a value that is its own version is a pinned one.
        if model.value == "default" || model.value == model.resolved || model.value.trim().is_empty() {
            continue;
        }
        if kept.iter().any(|held| held.name == model.value) {
            continue;
        }
        kept.push(HarnessModel {
            name: model.value,
            display: model.display,
            version: model.resolved,
            description: model.description,
            efforts: model.efforts,
        });
    }
    match kept.is_empty() {
        true => Some(Err(String::from("its answer named no model by an alias"))),
        false => Some(Ok(kept)),
    }
}

impl HeadlessAgent {
    /// The models this harness offers by an alias, asked of the program itself.
    ///
    /// **Blocking, and for the caller to put off the start path.** The process is
    /// killed as soon as it has answered or `within` has passed. No session file is
    /// written and no server is connected. The `Err` is a sentence for a person: what
    /// was tried and why it did not come back.
    pub fn ask_models(&self, within: Duration) -> Result<Vec<HarnessModel>, String> {
        let program = self.program();
        let mut child = Command::new(program)
            .args([
                "-p",
                "--input-format",
                "stream-json",
                "--output-format",
                "stream-json",
                "--verbose",
                "--no-session-persistence",
                "--strict-mcp-config",
            ])
            .current_dir(std::env::temp_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|why| format!("`{program}` could not be started: {why}"))?;
        let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            return Err(format!("`{program}` could not be spoken to"));
        };
        let (lines, heard) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = Vec::new();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if lines.send(line).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        // The pipe stays open until the answer is in: a program that sees the end of
        // its input before it has answered may end instead.
        let written = writeln!(stdin, "{ASK}").and_then(|()| stdin.flush());
        let ended = Instant::now() + within;
        let answer = match written {
            Err(why) => Err(format!("`{program}` could not be asked: {why}")),
            Ok(()) => loop {
                let left = ended.saturating_duration_since(Instant::now());
                match heard.recv_timeout(left) {
                    Ok(line) => {
                        if let Some(answer) = models_in(&line) {
                            break answer.map_err(|why| format!("`{program}`: {why}"));
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        break Err(format!(
                            "`{program}` did not answer within {} seconds",
                            within.as_secs()
                        ))
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break Err(format!("`{program}` ended before it answered"))
                    }
                }
            },
        };
        drop(stdin);
        let _ = child.kill();
        let _ = child.wait();
        answer
    }
}
