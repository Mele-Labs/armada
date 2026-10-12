//! One sentence read as Autopilot's settings. **Pure but for [`settings_from`]**, which makes the
//! one call, the way `crate::proposing` is the question and `crate::proposal` the call.
//!
//! **What this answers is a proposal.** A person confirms or changes it, and the start takes what
//! they confirmed: Fleet never starts on a guess. So a reading it cannot be sure of is refused
//! here, rather than filled in, and the person types the setting instead.

use std::fmt;

use adapter_traits::Ask;
use config::settings::PROMPT_AUTOPILOT_SENTENCE;
use core_model::Timestamp;

use super::Asking;
use crate::clock::rfc3339_utc;
use crate::judging::{said, CallFailed};
use crate::prompts::Prompts;

/// The question, with `{sentence}`, `{now}` and `{local_now}` to fill. As it ships:
/// `prompts.autopilotSentence` is the setting that overrides it.
pub(crate) const QUESTION: &str = "\
A person typed one sentence to set a repository working through its own backlog of issues \
while they are away. Read it as settings. Answer only the block at the end.

The sentence:

    {sentence}

It is now {now} in UTC, and {local_now} on the person's own clock. A time they name is on \
their clock.

Answer with nothing but these ten lines, in this order. Each line ending in `_from` copies the \
words of the sentence its setting was read from, exactly as they were typed, or says none where \
the sentence said nothing about it:

    instruction: <what to work on and what to leave, in the person's words, without the \
numbers and times below>
    instruction_from: <the words, or none>
    at_once: <how many issues may be worked at the same time. 1 where the sentence says nothing>
    at_once_from: <the words, or none>
    until: <the time to stop starting new work, as RFC 3339 with the person's offset, the \
next time that clock reads it. none where the sentence names no time>
    until_from: <the words, or none>
    spend: <the most to spend in total, in dollars, a number. none where it names no amount>
    spend_from: <the words, or none>
    jobs: <the most issues to start in total, a number. none where it names no count>
    jobs_from: <the words, or none>";

/// The settings one sentence was read as, for a person to confirm or change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedSettings {
    pub instruction: String,
    pub at_once: u32,
    /// In UTC, whatever offset the answer wrote it with.
    pub until: Option<Timestamp>,
    pub spend_cap_micros: Option<u64>,
    pub job_cap: Option<u32>,
    /// The words of the sentence each setting was read from.
    pub quoted: Quoted,
}

/// Per setting, the person's own words it was read from. **`None` where the sentence said nothing
/// about it**, and where the reader quoted words the sentence does not hold: a quote is shown to
/// the person as theirs, so one they never typed is dropped rather than shown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Quoted {
    pub instruction: Option<String>,
    pub at_once: Option<String>,
    pub until: Option<String>,
    pub spend: Option<String>,
    pub jobs: Option<String>,
}

/// Why no settings came back. The person types them instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotRead {
    /// The call itself did not answer.
    Call(CallFailed),
    /// It answered, and the answer was not settings: `why` names the line.
    Unreadable { why: String, answer: String },
}

impl fmt::Display for NotRead {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotRead::Call(failed) => write!(out, "the sentence was not read: {failed}"),
            NotRead::Unreadable { why, .. } => write!(out, "the sentence was read, but {why}"),
        }
    }
}

/// The question one sentence is put as, in the words of `prompts.autopilotSentence`.
pub fn settings_question(
    prompts: &Prompts,
    sentence: &str,
    now: &Timestamp,
    local_now: Option<&str>,
) -> String {
    prompts.fill(
        PROMPT_AUTOPILOT_SENTENCE,
        &[
            ("sentence", sentence.trim()),
            ("now", now.as_str()),
            ("local_now", local_now.unwrap_or(now.as_str())),
        ],
    )
}

/// Ask once, and read the answer.
pub async fn settings_from(
    sentence: &str,
    now: &Timestamp,
    local_now: Option<&str>,
    asking: &Asking,
) -> Result<ProposedSettings, NotRead> {
    let question = settings_question(&asking.prompts, sentence, now, local_now);
    let ask = Ask::put(asking.model.clone(), &question, asking.environment.clone())
        .map_err(|_| NotRead::Call(CallFailed::NothingToAsk))?;
    let answer = said(asking.client.as_ref(), &ask, asking.budget)
        .await
        .map_err(NotRead::Call)?;
    read_settings(&answer, sentence, now)
}

/// Read an answer to a question about `sentence`. **Strict about the settings**: a line missing or
/// unreadable refuses the whole reading, since a setting filled in for the person is the guess this
/// exists not to make. **Lenient about the quotes**, which only say where a setting came from.
pub fn read_settings(
    answer: &str,
    sentence: &str,
    now: &Timestamp,
) -> Result<ProposedSettings, NotRead> {
    let unreadable = |why: String| NotRead::Unreadable {
        why,
        answer: answer.to_string(),
    };
    let instruction = line(answer, "instruction")
        .filter(|said| !said.is_empty())
        .ok_or_else(|| unreadable("it named no instruction".to_string()))?;
    let at_once = line(answer, "at_once")
        .and_then(|said| said.parse::<u32>().ok())
        .filter(|count| *count > 0)
        .ok_or_else(|| unreadable("it named no number of issues to work at once".to_string()))?;
    let until = match line(answer, "until").as_deref() {
        None | Some("none") => None,
        Some(said) => {
            let millis = utc_millis(said)
                .ok_or_else(|| unreadable(format!("`{said}` is not a time it could stop at")))?;
            if now.epoch_millis().is_some_and(|now| millis <= now) {
                return Err(unreadable(format!("`{said}` has already passed")));
            }
            utc(said)
        }
    };
    let spend_cap_micros = match line(answer, "spend").as_deref() {
        None | Some("none") => None,
        Some(said) => {
            Some(micros_of(said).ok_or_else(|| unreadable(format!("`{said}` is not an amount")))?)
        }
    };
    let job_cap = match line(answer, "jobs").as_deref() {
        None | Some("none") => None,
        Some(said) => Some(
            said.parse::<u32>()
                .ok()
                .ok_or_else(|| unreadable(format!("`{said}` is not a number of issues")))?,
        ),
    };
    let from = |key: &str| quoted(answer, key, sentence);
    Ok(ProposedSettings {
        instruction,
        at_once,
        until,
        spend_cap_micros,
        job_cap,
        quoted: Quoted {
            instruction: from("instruction_from"),
            at_once: from("at_once_from"),
            until: from("until_from"),
            spend: from("spend_from"),
            jobs: from("jobs_from"),
        },
    })
}

/// The words a `key` line quotes, where `sentence` holds them. Quotation marks around them are
/// taken off, and the match ignores case and runs of spaces, which is how a reader copies.
fn quoted(answer: &str, key: &str, sentence: &str) -> Option<String> {
    let said = line(answer, key)?;
    let said = said.trim_matches(|c: char| c == '"' || c == '\u{201c}' || c == '\u{201d}');
    let said = said.trim();
    if said.is_empty() || said.eq_ignore_ascii_case("none") {
        return None;
    }
    let squeezed = |text: &str| {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    squeezed(sentence)
        .contains(&squeezed(said))
        .then(|| said.to_string())
}

/// The value of the first `key:` line, trimmed, with surrounding backticks taken off.
pub(crate) fn line(answer: &str, key: &str) -> Option<String> {
    answer.lines().find_map(|line| {
        let (named, value) = line.trim().split_once(':')?;
        (named.trim().eq_ignore_ascii_case(key))
            .then(|| value.trim().trim_matches('`').trim().to_string())
    })
}

/// An RFC 3339 instant with `Z` or an offset, held in UTC. `None` where it has no zone.
pub(crate) fn utc(said: &str) -> Option<Timestamp> {
    utc_millis(said).map(|millis| Timestamp::from_rfc3339(rfc3339_utc(millis)))
}

/// An RFC 3339 instant with `Z` or a `±HH:MM` offset, as milliseconds since the epoch in UTC.
fn utc_millis(said: &str) -> Option<i64> {
    let said = said.trim();
    let local = Timestamp::from_rfc3339(said).epoch_millis()?;
    let zone = said.get(19..)?;
    let zone = zone.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset_minutes = match zone {
        "Z" | "z" => 0,
        signed if signed.len() == 6 && &signed[3..4] == ":" => {
            let sign = match &signed[..1] {
                "+" => 1,
                "-" => -1,
                _ => return None,
            };
            let hours: i64 = signed[1..3].parse().ok()?;
            let minutes: i64 = signed[4..6].parse().ok()?;
            sign * (hours * 60 + minutes)
        }
        _ => return None,
    };
    Some(local - offset_minutes * 60_000)
}

/// Dollars, written `30`, `$30` or `30.50`, as millionths of a dollar.
fn micros_of(said: &str) -> Option<u64> {
    let said = said.trim().trim_start_matches('$').replace(',', "");
    let (whole, cents) = said.split_once('.').unwrap_or((&said, ""));
    let whole: u64 = whole.parse().ok()?;
    let fraction = match cents.len() {
        0 => 0,
        1..=6 => {
            let digits: u64 = cents.parse().ok()?;
            digits * 10u64.pow(6 - cents.len() as u32)
        }
        _ => return None,
    };
    whole.checked_mul(1_000_000)?.checked_add(fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offset_is_read_back_to_utc() {
        assert_eq!(
            utc_millis("2026-10-12T07:00:00-04:00"),
            utc_millis("2026-10-12T11:00:00Z")
        );
        assert_eq!(
            utc_millis("2026-10-12T07:00:00.000+02:00"),
            utc_millis("2026-10-12T05:00:00Z")
        );
        assert_eq!(
            utc_millis("2026-10-12T07:00:00"),
            None,
            "a time with no zone is a guess"
        );
    }

    #[test]
    fn dollars_are_read_as_micros() {
        assert_eq!(micros_of("$30"), Some(30_000_000));
        assert_eq!(micros_of("12.5"), Some(12_500_000));
        assert_eq!(micros_of("thirty"), None);
    }

    const SENTENCE: &str = "work through the open issues that don't need design decisions, \
                            two at a time, until 7am, $30 max";

    fn now() -> Timestamp {
        Timestamp::from_rfc3339("2026-10-12T03:00:00Z")
    }

    #[test]
    fn each_setting_carries_the_words_it_was_read_from() {
        let answer = "instruction: work through the open issues that don't need design decisions\n\
                      instruction_from: \"work through the open issues that don't need design decisions\"\n\
                      at_once: 2\n\
                      at_once_from: two at a time\n\
                      until: 2026-10-12T07:00:00-04:00\n\
                      until_from: until 7am\n\
                      spend: 30\n\
                      spend_from: `$30 max`\n\
                      jobs: none\n\
                      jobs_from: none";

        let read = read_settings(answer, SENTENCE, &now()).expect("read");

        assert_eq!(
            read.quoted,
            Quoted {
                instruction: Some(
                    "work through the open issues that don't need design decisions".to_string()
                ),
                at_once: Some("two at a time".to_string()),
                until: Some("until 7am".to_string()),
                spend: Some("$30 max".to_string()),
                jobs: None,
            }
        );
        assert_eq!(read.at_once, 2);
    }

    #[test]
    fn a_quote_the_answer_leaves_out_is_none_and_the_reading_still_stands() {
        let answer = "instruction: fix the flakes\nat_once: 1\nuntil: none\nspend: none\njobs: 3";

        let read = read_settings(answer, "fix the flakes, three of them", &now()).expect("read");

        assert_eq!(read.quoted, Quoted::default(), "no `_from` line, no quote");
        assert_eq!(read.job_cap, Some(3));
    }

    #[test]
    fn words_the_person_never_typed_are_not_quoted_as_theirs() {
        let answer = "instruction: fix the flakes\n\
                      instruction_from: Fix   THE flakes\n\
                      at_once: 1\n\
                      at_once_from: one at a time\n\
                      until: none\nspend: none\njobs: none";

        let read = read_settings(answer, "fix the flakes", &now()).expect("read");

        assert_eq!(
            read.quoted.instruction.as_deref(),
            Some("Fix   THE flakes"),
            "case and spacing are how a reader copies"
        );
        assert_eq!(read.quoted.at_once, None, "the sentence never said it");
    }
}
