//! The replies a Job's quick bar offers that no setting holds: answers to the agent's last message, written by
//! a small model, and lines he sends often, read off what he sent. `get_session_quick`.
//!
//! **Nothing here calls a model or touches a store.** The question is laid, the answer read, and his sent lines
//! ranked; `session_host/quick.rs` makes the calls and keeps what they cost.

use std::collections::{HashMap, HashSet};

use ipc::{SessionRow, SessionVoice};
use store::settings_file::SettingValue;

use crate::prompts::fill;

/// The most answers to the agent's message, and learnt lines, one Job's bar is offered.
pub(crate) const MOST_ASKED: usize = 3;
pub(crate) const MOST_LEARNT: usize = 6;
/// The longest line a chip carries, in characters.
pub(crate) const LINE_AT_MOST: usize = 60;
/// A line is learnt once he has sent it this many times.
const SENT_AT_LEAST: usize = 2;
/// What a sending is worth halves in this long.
const HALF_LIFE_DAYS: f64 = 14.0;
/// A sending right after a question counts this many times over while the agent's last message asks one.
const MOMENT_WEIGHT: f64 = 2.0;
const DAY_MS: f64 = 86_400_000.0;
/// The most of the agent's message the model is shown, from its end: a question comes last.
const MESSAGE_AT_MOST: usize = 1_500;
/// The most of his own lines the model is shown to catch his voice.
pub(crate) const VOICE_LINES: usize = 5;

/// The prompt as it ships. **Not an editable setting**, as the account's is not: the answer is read as lines.
pub(crate) const SHIPPED: &str = "\
A coding agent has just sent a person this message, and the person is about to answer it. Write the \
short replies he might send back, in his own voice.

The agent's message:

{message}

Lines he has sent before, to catch his voice:

{voice}

Answer with nothing but the replies, one to a line, at most three. Each is a few words, no more \
than sixty characters, written as he would type it. No numbering, no quotes, no explanation. Give \
a reply only where it would plainly be a fine answer; where nothing short fits, write nothing.";

/// The question, filled in. `his` are lines he sent, most recent first.
pub(crate) fn question(message: &str, his: &[String]) -> String {
    let voice = match his.is_empty() {
        true => String::from("  (none yet)"),
        false => his.iter().map(|one| format!("  - {one}")).collect::<Vec<_>>().join("\n"),
    };
    fill(SHIPPED, &[("message", &tail(message.trim(), MESSAGE_AT_MOST)), ("voice", &voice)])
}

fn tail(text: &str, most: usize) -> String {
    let count = text.chars().count();
    match count > most {
        true => format!("…{}", text.chars().skip(count - most).collect::<String>()),
        false => text.to_string(),
    }
}

/// The model's answer, read into at most [`MOST_ASKED`] lines. **Lenient about the dressing**: a number, a
/// bullet or quotes round a line are taken off, and a line that is not a reply (a lead-in ending in a colon, a
/// word for nothing, one past the chip's length) is dropped.
pub(crate) fn read_asked(reply: &str) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for line in reply.lines() {
        let line = undressed(line);
        let nothing = ["none", "nothing", "n/a"].contains(&line.to_lowercase().trim_end_matches('.'));
        if line.is_empty() || nothing || line.ends_with(':') {
            continue;
        }
        let Some(line) = one_line(line) else { continue };
        if kept.iter().any(|one| alike(one) == alike(&line)) {
            continue;
        }
        kept.push(line);
        if kept.len() == MOST_ASKED {
            break;
        }
    }
    kept
}

const QUOTES: [char; 8] = ['"', '\'', '“', '”', '‘', '’', '`', '*'];

fn undressed(line: &str) -> &str {
    let mut line = line.trim().trim_start_matches(['-', '*', '•', ' ']);
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        if let Some(rest) = line[digits..].strip_prefix(['.', ')']) {
            line = rest.trim_start();
        }
    }
    line.trim_matches(QUOTES.as_slice()).trim()
}

/// `text` as a chip carries it: trimmed, one line, and no longer than [`LINE_AT_MOST`].
pub(crate) fn one_line(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty() && !text.contains('\n') && text.chars().count() <= LINE_AT_MOST).then(|| text.to_string())
}

/// Lines that are alike: equal once lowercased, trimmed, with their whitespace collapsed and no punctuation
/// trailing. Empty where nothing but punctuation is left.
pub(crate) fn alike(line: &str) -> String {
    let words: Vec<&str> = line.split_whitespace().collect();
    let joined = words.join(" ").to_lowercase();
    joined.trim_end_matches(|c: char| c.is_ascii_punctuation() || c == '…' || c.is_whitespace()).to_string()
}

/// Whether a message of the agent's asks something: it ends with a question, ignoring the quotes, brackets
/// and emphasis a model closes a sentence with.
pub(crate) fn asks(message: &str) -> bool {
    message.trim().trim_end_matches(|c: char| QUOTES.contains(&c) || matches!(c, ')' | ']' | '_')).ends_with('?')
}

/// The agent's last message, with its row's id, where the thread's last message is the agent's.
pub(crate) fn agent_spoke_last(rows: &[SessionRow]) -> Option<(&str, &str)> {
    rows.iter().rev().find_map(|row| match row {
        SessionRow::Message { id, from, text, .. } => Some(matches!(from, SessionVoice::Agent).then_some((id.as_str(), text.as_str()))),
        _ => None,
    })?
}

/// One line he sent, and the moment he sent it in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Sent {
    pub text: String,
    pub at_ms: i64,
    /// The message before it was the agent's, and asked something.
    pub after_question: bool,
}

/// What a thread holds of lines he sent that a chip could carry. **His words only**: a note of Fleet's, a line with a link, a row
/// carrying a file or a tag, and a line that is more than a line are not.
pub(crate) fn sent_in(rows: &[SessionRow]) -> Vec<Sent> {
    let mut sent = Vec::new();
    let mut before: Option<&SessionRow> = None;
    for row in rows {
        let SessionRow::Message { from, text, at, files, tags, notice, .. } = row else { continue };
        let after_question = matches!(before, Some(SessionRow::Message { from: SessionVoice::Agent, text, .. }) if asks(text));
        before = Some(row);
        if !matches!(from, SessionVoice::You) || notice.is_some() || !files.is_empty() || !tags.is_empty() {
            continue;
        }
        let (Some(text), Some(at_ms)) = (one_line(text), at.to_domain().epoch_millis()) else { continue };
        if text.contains("://") {
            continue;
        }
        sent.push(Sent { text, at_ms, after_question });
    }
    sent
}

/// The lines to learn from what he sent, most useful first. **Alike lines are one**, shown in the wording he used
/// last. A line is offered once he sent it twice; it ranks by how often and how recently, each sending worth
/// half as much for every [`HALF_LIFE_DAYS`] since, and worth more again while `asking`, when it followed a
/// question the way the agent's last message is one. `left_out` are lines as [`alike`] reads them.
pub(crate) fn learnt(sent: &[Sent], left_out: &HashSet<String>, asking: bool, now_ms: i64) -> Vec<String> {
    struct Tally {
        wording: String,
        latest: i64,
        count: usize,
        score: f64,
    }
    let mut tallies: HashMap<String, Tally> = HashMap::new();
    for one in sent {
        let key = alike(&one.text);
        if key.is_empty() || left_out.contains(&key) {
            continue;
        }
        let age = (now_ms - one.at_ms).max(0) as f64 / DAY_MS;
        let weight = 0.5_f64.powf(age / HALF_LIFE_DAYS) * if asking && one.after_question { MOMENT_WEIGHT } else { 1.0 };
        let tally = tallies.entry(key).or_insert_with(|| Tally { wording: one.text.clone(), latest: one.at_ms, count: 0, score: 0.0 });
        tally.count += 1;
        tally.score += weight;
        if one.at_ms >= tally.latest {
            tally.latest = one.at_ms;
            tally.wording = one.text.clone();
        }
    }
    let mut ranked: Vec<Tally> = tallies.into_values().filter(|one| one.count >= SENT_AT_LEAST).collect();
    ranked.sort_by(|a, b| b.score.total_cmp(&a.score).then(b.latest.cmp(&a.latest)).then_with(|| a.wording.cmp(&b.wording)));
    ranked.into_iter().take(MOST_LEARNT).map(|one| one.wording).collect()
}

/// What `thread.look` says that Fleet acts on. Absent or unreadable is Bridge's default: learning on, the bar shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QuickLook {
    pub learn: bool,
    off: bool,
    off_for: Vec<String>,
    /// His quick lines, saved and kept-learnt, and the learnt ones he deleted or reworded, as [`alike`] reads them.
    pub left_out: HashSet<String>,
}

impl QuickLook {
    pub(crate) fn read(look: Option<&SettingValue>) -> QuickLook {
        let SettingValue::Map(look) = look.unwrap_or(&SettingValue::Null) else {
            return QuickLook { learn: true, off: false, off_for: Vec::new(), left_out: HashSet::new() };
        };
        let texts = |value: Option<&SettingValue>| -> Vec<String> {
            match value {
                Some(SettingValue::List(list)) => list
                    .iter()
                    .filter_map(|one| match one {
                        SettingValue::Text(text) => Some(text.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            }
        };
        let quick = match look.get("quick") {
            Some(SettingValue::List(list)) => list
                .iter()
                .filter_map(|one| match one {
                    SettingValue::Map(phrase) => match phrase.get("text") {
                        Some(SettingValue::Text(text)) => Some(text.clone()),
                        _ => None,
                    },
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        QuickLook {
            learn: !matches!(look.get("learn"), Some(SettingValue::Bool(false))),
            off: matches!(look.get("quickOff"), Some(SettingValue::Bool(true))),
            off_for: texts(look.get("quickOffFor")),
            left_out: quick.iter().chain(texts(look.get("forgotten")).iter()).map(|one| alike(one)).filter(|one| !one.is_empty()).collect(),
        }
    }

    /// The bar is not drawn in this Job, so nothing is offered for it.
    pub(crate) fn hides(&self, session: &str) -> bool {
        self.off || self.off_for.iter().any(|one| one == session)
    }
}
