//! A Session's record, assembled for its retro from what Fleet keeps of it
//! since the last retro ended. `docs/concepts/retro.md`, *A Session's retro*.
//!
//! **Mechanical, as a Job's is.** Nothing here reads a file or a clock: the
//! thread, the agent CLI's transcript lines and the restarts are handed in, so
//! a record captured off a real Session is a test of all of it. Every row has
//! a `cite`, `kind:n` counted from one within its list.

use std::collections::BTreeMap;

use core_model::Timestamp;
use ipc::{
    Instant, RecordAct, RecordAsked, RecordRefusal, RecordSaid, RetroRecord, SessionAskState,
    SessionRow, SessionVoice, StepId,
};
use serde::Deserialize;

use super::record::{apart, cite, cut};

/// The most of one message, error or call a record carries.
const SAID: usize = 500;
/// The most corrections a record carries, the newest.
const MOST_CORRECTIONS: usize = 60;
/// The most rows a record carries from subagents.
const MOST_SUBAGENT_ROWS: usize = 30;

/// Everything a Session's record is assembled from.
pub(crate) struct SessionSources<'a> {
    /// The end of the last retro. Nothing at or before it is read.
    pub since: Option<&'a str>,
    /// Where this retro stops.
    pub until: &'a str,
    /// The thread: a hosted Session's rows, or a terminal Session's as drawn from its transcript.
    pub thread: &'a [SessionRow],
    /// The lines of the agent CLI's transcript of the Session.
    pub transcript: &'a [String],
    /// Each subagent's id and the lines of its transcript.
    pub subagents: &'a [(String, Vec<String>)],
    pub restarts: &'a [store::KeptRestart],
    /// What the agent said got in its way, when it was asked, and when.
    pub note: Option<(&'a str, &'a str)>,
}

/// Whether `at` falls after `since` and not after `until`. **A time that will
/// not read is inside**: dropping a row for its spelling would say less went
/// wrong than did.
pub(crate) fn within(at: &str, since: Option<&str>, until: &str) -> bool {
    let millis = |text: &str| Timestamp::from_rfc3339(text).epoch_millis();
    let Some(at) = millis(at) else { return true };
    since.and_then(millis).map_or(true, |since| at > since)
        && millis(until).map_or(true, |until| at <= until)
}

/// Whether the thread has anything in the stretch a retro would cover.
pub(crate) fn has_news(thread: &[SessionRow], since: Option<&str>, until: &str) -> bool {
    thread.iter().any(|row| match row {
        SessionRow::Message { at, from, .. } => {
            !matches!(from, SessionVoice::Session { id, .. } if id == "fleet")
                && within(at.as_str(), since, until)
        }
        SessionRow::Tool { at, .. } | SessionRow::Ask { at, .. } => {
            within(at.as_str(), since, until)
        }
        _ => false,
    })
}

pub(crate) fn assembled(from: &SessionSources<'_>) -> RetroRecord {
    let (mut refusals, mut failed_tools) = tool_trouble(from.transcript, from);
    for (n, row) in refusals.iter_mut().enumerate() {
        row.cite = cite("refusal", n);
    }
    for (n, row) in failed_tools.iter_mut().enumerate() {
        row.cite = cite("tool", n);
    }
    RetroRecord {
        refusals,
        failed_tools,
        asks: asks(from),
        corrections: corrections(from),
        restarts: restarts(from),
        subagents: subagents(from),
        notes: from
            .note
            .map(|(at, said)| RecordSaid {
                cite: cite("note", 0),
                at: Some(Instant::carried(at)),
                step: None,
                said: cut(said, SAID * 3),
            })
            .into_iter()
            .collect(),
        ..RetroRecord::default()
    }
}

fn asks(from: &SessionSources<'_>) -> Vec<RecordAsked> {
    let mut out = Vec::new();
    for row in from.thread {
        let SessionRow::Ask { at, ask, state, .. } = row else {
            continue;
        };
        if !within(ask.asked_at.as_str(), from.since, from.until) {
            continue;
        }
        let answer = match state {
            SessionAskState::Waiting | SessionAskState::RanUnasked => None,
            SessionAskState::AllowedOnce => Some("allowed once"),
            SessionAskState::AllowedAndRemembered => Some("allowed and remembered"),
            SessionAskState::Refused => Some("refused"),
            SessionAskState::Unanswered => Some("nobody answered, so it was refused"),
            SessionAskState::SessionGone => Some("the process ended first"),
        };
        if *state == SessionAskState::RanUnasked {
            continue;
        }
        let about = if ask.detail.is_empty() {
            ask.tool.clone()
        } else {
            format!("{} {}", ask.tool, cut(&ask.detail, SAID))
        };
        out.push(RecordAsked {
            cite: cite("ask", out.len()),
            asked_at: ask.asked_at.clone(),
            step: None,
            about,
            answered_at: answer.map(|_| at.clone()),
            answer: answer.map(str::to_string),
            waited_ms: answer.map(|_| apart(&ask.asked_at, at)),
        });
    }
    out
}

fn corrections(from: &SessionSources<'_>) -> Vec<RecordSaid> {
    let mut said: Vec<RecordSaid> = from
        .thread
        .iter()
        .filter_map(|row| match row {
            SessionRow::Message {
                at,
                from: SessionVoice::You,
                text,
                ..
            } if within(at.as_str(), from.since, from.until) => Some((at, text)),
            _ => None,
        })
        .map(|(at, text)| RecordSaid {
            cite: String::new(),
            at: Some(at.clone()),
            step: None,
            said: cut(text, SAID),
        })
        .collect();
    let skip = said.len().saturating_sub(MOST_CORRECTIONS);
    said.drain(..skip);
    for (n, row) in said.iter_mut().enumerate() {
        row.cite = cite("correction", n);
    }
    said
}

fn restarts(from: &SessionSources<'_>) -> Vec<RecordAct> {
    from.restarts
        .iter()
        .filter(|restart| within(&restart.at, from.since, from.until))
        .enumerate()
        .map(|(n, restart)| RecordAct {
            cite: cite("restart", n),
            at: Instant::carried(restart.at.clone()),
            actor: ipc::Actor::from(core_model::Actor::Fleet),
            via: None,
            moved: restart.kind.clone(),
            said: restart.said.clone(),
        })
        .collect()
}

fn subagents(from: &SessionSources<'_>) -> Vec<RecordSaid> {
    let mut out = Vec::new();
    for (agent, lines) in from.subagents {
        let (refused, failed) = tool_trouble(lines, from);
        for row in refused.into_iter().chain(failed) {
            if out.len() >= MOST_SUBAGENT_ROWS {
                return out;
            }
            let tried = row.tried.map(|tried| format!(" {tried}")).unwrap_or_default();
            out.push(RecordSaid {
                cite: cite("subagent", out.len()),
                at: Some(row.at),
                step: Some(StepId::carried(agent.clone())),
                said: format!(
                    "{}{tried}: {}",
                    row.tool,
                    row.because.unwrap_or_default()
                ),
            });
        }
    }
    out
}

/// One line of the agent CLI's transcript, as far as a retro reads it.
#[derive(Deserialize)]
struct Line {
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    content: Content,
}

/// A message's content: a string, or blocks.
#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Blocks(Vec<Block>),
    Text(#[allow(dead_code)] String),
}

impl Default for Content {
    fn default() -> Content {
        Content::Blocks(Vec::new())
    }
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    input: Input,
    #[serde(default)]
    tool_use_id: String,
    #[serde(default)]
    is_error: bool,
    #[serde(default)]
    content: Output,
}

/// What a call was given: the argument a person would name it by.
#[derive(Deserialize, Default)]
struct Input {
    command: Option<String>,
    file_path: Option<String>,
    pattern: Option<String>,
    url: Option<String>,
    path: Option<String>,
    query: Option<String>,
}

impl Input {
    fn tried(&self) -> String {
        [&self.command, &self.file_path, &self.pattern, &self.url, &self.path, &self.query]
            .into_iter()
            .find_map(|one| one.as_deref())
            .map(|said| cut(said, SAID))
            .unwrap_or_default()
    }
}

/// A result's content: a string, or parts with text.
#[derive(Deserialize)]
#[serde(untagged)]
enum Output {
    Text(String),
    Parts(Vec<Part>),
}

impl Default for Output {
    fn default() -> Output {
        Output::Text(String::new())
    }
}

#[derive(Deserialize)]
struct Part {
    #[serde(default)]
    text: String,
}

impl Output {
    fn text(&self) -> String {
        match self {
            Output::Text(text) => text.clone(),
            Output::Parts(parts) => parts
                .iter()
                .map(|part| part.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

/// The tool results that errored in a transcript, each joined to the call it
/// answered: those the harness or a hook refused, then the rest.
/// **Read by the shape of the CLI's own lines**, `tool_use` blocks on assistant
/// lines and `tool_result` blocks with `is_error` on the user lines after them.
fn tool_trouble(
    lines: &[String],
    from: &SessionSources<'_>,
) -> (Vec<RecordRefusal>, Vec<RecordRefusal>) {
    let mut called: BTreeMap<String, (String, String)> = BTreeMap::new();
    let (mut refused, mut failed) = (Vec::new(), Vec::new());
    for line in lines {
        let Ok(read) = ipc::decode::<Line>("a transcript line", line.as_bytes()) else {
            continue;
        };
        let Some(Message {
            content: Content::Blocks(blocks),
        }) = read.message
        else {
            continue;
        };
        for block in blocks {
            match block.kind.as_str() {
                "tool_use" => {
                    called.insert(block.id.clone(), (block.name.clone(), block.input.tried()));
                }
                "tool_result" if block.is_error => {
                    if !within(&read.timestamp, from.since, from.until) {
                        continue;
                    }
                    let (tool, tried) = called.get(&block.tool_use_id).cloned().unwrap_or_default();
                    let because = block.content.text();
                    let row = RecordRefusal {
                        cite: String::new(),
                        at: Instant::carried(read.timestamp.clone()),
                        step: None,
                        tool,
                        tried: Some(tried).filter(|tried| !tried.is_empty()),
                        because: Some(cut(because.trim(), SAID)).filter(|said| !said.is_empty()),
                    };
                    if is_refusal(&because) {
                        refused.push(row);
                    } else {
                        failed.push(row);
                    }
                }
                _ => {}
            }
        }
    }
    (refused, failed)
}

/// A result that says the call was refused rather than that it failed: a hook's
/// `Refused:`, the harness's own denial, or the person declining.
fn is_refusal(because: &str) -> bool {
    let lower = because.trim_start().to_lowercase();
    lower.starts_with("refused")
        || lower.contains("shown in bridge's window")
        || lower.contains("was denied")
        || lower.contains("has been denied")
        || lower.contains("doesn't want to proceed")
        || lower.contains("permission to use")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines() -> Vec<String> {
        [
            r#"{"type":"assistant","timestamp":"2026-10-08T10:00:00.000Z","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"find / -name pnpm"}},{"type":"tool_use","id":"t2","name":"Bash","input":{"command":"open http://x"}}]}}"#,
            r#"{"type":"user","timestamp":"2026-10-08T10:00:01.000Z","message":{"content":[{"type":"tool_result","tool_use_id":"t1","is_error":true,"content":"Operation not permitted"},{"type":"tool_result","tool_use_id":"t2","is_error":true,"content":[{"type":"text","text":"Refused: use show_window"}]}]}}"#,
            r#"{"type":"user","timestamp":"2026-10-08T09:00:00.000Z","message":{"content":"plain text"}}"#,
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    #[test]
    fn errored_results_split_into_refusals_and_failures_joined_to_their_calls() {
        let transcript = lines();
        let record = assembled(&SessionSources {
            since: Some("2026-10-08T09:30:00.000Z"),
            until: "2026-10-08T11:00:00.000Z",
            thread: &[],
            transcript: &transcript,
            subagents: &[],
            restarts: &[],
            note: Some(("2026-10-08T10:30:00.000Z", "find hit the privacy prompts")),
        });
        assert_eq!(record.failed_tools.len(), 1);
        assert_eq!(record.failed_tools[0].cite, "tool:1");
        assert_eq!(record.failed_tools[0].tried.as_deref(), Some("find / -name pnpm"));
        assert_eq!(record.refusals.len(), 1);
        assert_eq!(record.refusals[0].cite, "refusal:1");
        assert_eq!(record.notes[0].cite, "note:1");
    }

    #[test]
    fn a_row_before_the_last_retro_is_not_read() {
        let transcript = lines();
        let record = assembled(&SessionSources {
            since: Some("2026-10-08T10:00:00.500Z"),
            until: "2026-10-08T11:00:00.000Z",
            thread: &[],
            transcript: &transcript,
            subagents: &[],
            restarts: &[],
            note: None,
        });
        assert_eq!(record.failed_tools.len() + record.refusals.len(), 2);
        let record = assembled(&SessionSources {
            since: Some("2026-10-08T10:00:02.000Z"),
            until: "2026-10-08T11:00:00.000Z",
            thread: &[],
            transcript: &transcript,
            subagents: &[],
            restarts: &[],
            note: None,
        });
        assert!(record.failed_tools.is_empty() && record.refusals.is_empty());
    }
}
