//! One sentence read as Autopilot's settings. **Pure but for [`settings_from`]**, which makes the
//! one call, the way `crate::proposing` is the question and `crate::proposal` the call.
//!
//! **What this answers is a proposal.** A person confirms or changes it, and the start takes what
//! they confirmed: Fleet never starts on a guess. So a reading it cannot be sure of is refused
//! here, rather than filled in, and the person types the setting instead.

use std::fmt;

use adapter_traits::Ask;
use core_model::Timestamp;

use super::Asking;
use crate::clock::rfc3339_utc;
use crate::judging::{said, CallFailed};

/// The question, with `{sentence}`, `{now}` and `{local_now}` to fill.
const QUESTION: &str = "\
A person typed one sentence to set a repository working through its own backlog of issues \
while they are away. Read it as settings. Answer only the block at the end.

The sentence:

    {sentence}

It is now {now} in UTC, and {local_now} on the person's own clock. A time they name is on \
their clock.

Answer with nothing but these five lines, in this order:

    instruction: <what to work on and what to leave, in the person's words, without the \
numbers and times below>
    at_once: <how many issues may be worked at the same time. 1 where the sentence says nothing>
    until: <the time to stop starting new work, as RFC 3339 with the person's offset, the \
next time that clock reads it. none where the sentence names no time>
    spend: <the most to spend in total, in dollars, a number. none where it names no amount>
    jobs: <the most issues to start in total, a number. none where it names no count>";

/// The settings one sentence was read as, for a person to confirm or change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedSettings {
    pub instruction: String,
    pub at_once: u32,
    /// In UTC, whatever offset the answer wrote it with.
    pub until: Option<Timestamp>,
    pub spend_cap_micros: Option<u64>,
    pub job_cap: Option<u32>,
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

/// The question one sentence is put as.
pub fn settings_question(sentence: &str, now: &Timestamp, local_now: Option<&str>) -> String {
    QUESTION
        .replace("{sentence}", sentence.trim())
        .replace("{now}", now.as_str())
        .replace("{local_now}", local_now.unwrap_or(now.as_str()))
}

/// Ask once, and read the answer.
pub async fn settings_from(
    sentence: &str,
    now: &Timestamp,
    local_now: Option<&str>,
    asking: &Asking,
) -> Result<ProposedSettings, NotRead> {
    let question = settings_question(sentence, now, local_now);
    let ask = Ask::put(asking.model.clone(), &question, asking.environment.clone())
        .map_err(|_| NotRead::Call(CallFailed::NothingToAsk))?;
    let answer = said(asking.client.as_ref(), &ask, asking.budget)
        .await
        .map_err(NotRead::Call)?;
    read_settings(&answer, now)
}

/// Read an answer. **Strict**: a line missing or unreadable refuses the whole reading, since a
/// setting filled in for the person is the guess this exists not to make.
pub fn read_settings(answer: &str, now: &Timestamp) -> Result<ProposedSettings, NotRead> {
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
    Ok(ProposedSettings {
        instruction,
        at_once,
        until,
        spend_cap_micros,
        job_cap,
    })
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
}
