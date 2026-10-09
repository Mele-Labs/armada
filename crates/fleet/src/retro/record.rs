//! A Job's record, assembled for its retro from what is already kept.
//!
//! **The same reading `scripts/job` makes**, from the same sources: the Job's
//! detail, its history and its evidence as the routes serve them, its Drones'
//! transcripts and its own log. The join `scripts/job` exists for — a refusal
//! to the call it answered, on the call id — is [`refusals`] here.
//!
//! Every row gets a `cite`, `kind:n` counted from one within its list, which
//! is what a retro's items name as evidence. Nothing here reads a file or a
//! clock, so a record captured off a real Job is a test of all of it.

use std::collections::{BTreeMap, BTreeSet};

use core_model::Timestamp;
use ipc::{
    Actor, CheckRunBy, Instant, JobDetail, JobEvidence, JobHistory, LogNote, Movement, RecordAct,
    RecordAsked, RecordCheck, RecordNotMet, RecordPath, RecordRefusal, RecordSaid, RecordWaited,
    RetroRecord, Saw, StepId, TranscriptRow, Voice,
};

use super::lines;

/// The most of one Drone's sentence a record carries. A retro is read about a
/// Job, and a transcript is what it exists to save somebody reading.
const SAID: usize = 1500;

/// The most of a refused call's argument a record carries.
const TRIED: usize = 500;

/// Everything a record is assembled from.
pub(crate) struct Sources<'a> {
    pub detail: &'a JobDetail,
    pub history: &'a JobHistory,
    pub evidence: &'a JobEvidence,
    /// One per Drone the Job had, each in the order it was written.
    pub transcripts: &'a [Vec<TranscriptRow>],
    pub log: &'a [LogNote],
    pub notes: &'a [store::DroneNote],
}

pub(crate) fn assembled(from: &Sources<'_>) -> RetroRecord {
    RetroRecord {
        refusals: refusals(from.transcripts),
        failed_checks: failed_checks(from),
        not_met: not_met(from.detail),
        not_done: not_done(from.evidence),
        said_after: said_after(from.transcripts),
        restarts: restarts(from.history, from.log),
        asked: asked(from.log),
        waited: waited(from.history),
        acts: acts(from.history),
        notes: from
            .notes
            .iter()
            .enumerate()
            .map(|(n, note)| RecordSaid {
                cite: cite("note", n),
                at: Some(Instant::from(&note.at)),
                step: Some(StepId::from(&note.step_id)),
                said: note.said.clone(),
            })
            .collect(),
        ..RetroRecord::default()
    }
}

/// Every `cite` the record holds, which is what a retro item may name.
pub(crate) fn cites(record: &RetroRecord) -> BTreeSet<String> {
    let mut all = BTreeSet::new();
    all.extend(record.refusals.iter().map(|row| row.cite.clone()));
    all.extend(record.failed_checks.iter().map(|row| row.cite.clone()));
    all.extend(record.not_met.iter().map(|row| row.cite.clone()));
    for said in [&record.not_done, &record.said_after, &record.notes] {
        all.extend(said.iter().map(|row| row.cite.clone()));
    }
    all.extend(record.restarts.iter().map(|row| row.cite.clone()));
    all.extend(record.acts.iter().map(|row| row.cite.clone()));
    all.extend(record.asked.iter().map(|row| row.cite.clone()));
    all.extend(record.waited.iter().map(|row| row.cite.clone()));
    all.extend(record.asks.iter().map(|row| row.cite.clone()));
    all.extend(record.failed_tools.iter().map(|row| row.cite.clone()));
    all.extend(record.corrections.iter().map(|row| row.cite.clone()));
    all.extend(record.subagents.iter().map(|row| row.cite.clone()));
    all
}

pub(super) fn cite(kind: &str, n: usize) -> String {
    format!("{kind}:{}", n + 1)
}

pub(super) fn cut(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_string(),
    }
}

/// Each refusal, joined to the call it answered on the call id: the refusal
/// names the tool and never the argument.
fn refusals(transcripts: &[Vec<TranscriptRow>]) -> Vec<RecordRefusal> {
    let mut out = Vec::new();
    for rows in transcripts {
        let called: BTreeMap<&str, &str> = rows
            .iter()
            .filter_map(|row| match &row.saw {
                Saw::Called {
                    call,
                    detail,
                    whole,
                    ..
                } => Some((call.as_str(), whole.as_deref().unwrap_or(detail))),
                _ => None,
            })
            .collect();
        for row in rows {
            if let Saw::Refused {
                tool,
                call,
                because,
            } = &row.saw
            {
                out.push(RecordRefusal {
                    cite: cite("refusal", out.len()),
                    at: row.ts.clone(),
                    step: row.step.clone(),
                    tool: tool.clone(),
                    tried: called.get(call.as_str()).map(|tried| cut(tried, TRIED)),
                    because: Some(because.clone()).filter(|said| !said.trim().is_empty()),
                });
            }
        }
    }
    out
}

/// The gate's failures off the record, then any the transcripts hold that the
/// record does not — a check the gate runs outside the declared set, like
/// `out_of_bounds` — then the Drone's own runs, off the Job's log.
fn failed_checks(from: &Sources<'_>) -> Vec<RecordCheck> {
    let mut out: Vec<RecordCheck> = Vec::new();
    let mut seen = BTreeSet::new();
    for step in &from.detail.steps {
        for run in step
            .check_runs
            .iter()
            .filter(|run| !run.outcome.domain().advances())
        {
            seen.insert((
                step.step_id.as_str().to_string(),
                run.attempt,
                run.name.clone(),
            ));
            out.push(RecordCheck {
                cite: String::new(),
                at: None,
                step: Some(step.step_id.clone()),
                attempt: Some(run.attempt),
                name: run.name.clone(),
                run: CheckRunBy::Gate,
                expected: run.expected.clone(),
                produced: run.produced.clone(),
                paths: Vec::new(),
            });
        }
    }
    for row in from.transcripts.iter().flatten() {
        let Saw::Checked { run } = &row.saw else {
            continue;
        };
        let step = row.step.as_ref().map(|step| step.as_str().to_string());
        let key = (
            step.clone().unwrap_or_default(),
            run.attempt,
            run.name.clone(),
        );
        if run.outcome.domain().advances() || seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        out.push(RecordCheck {
            cite: String::new(),
            at: Some(row.ts.clone()),
            step: row.step.clone(),
            attempt: Some(run.attempt),
            name: run.name.clone(),
            run: CheckRunBy::Gate,
            expected: run.expected.clone(),
            produced: run.produced.clone(),
            paths: Vec::new(),
        });
    }
    for note in from
        .log
        .iter()
        .filter(|note| note.msg == lines::A_DRONE_RAN_CHECKS)
    {
        let failed = field(note, "failed")
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0);
        if failed == 0 {
            continue;
        }
        let names: Vec<String> = match field(note, "failed_checks") {
            Some(named) => named.split(", ").map(str::to_string).collect(),
            None => vec![format!("{failed} of {}", field(note, "ran").unwrap_or("?"))],
        };
        for name in names {
            out.push(RecordCheck {
                cite: String::new(),
                at: Some(note.at.clone()),
                step: note.step.clone(),
                attempt: None,
                name,
                run: CheckRunBy::Drone,
                expected: None,
                produced: None,
                paths: Vec::new(),
            });
        }
    }
    // The gate's own reds it ran again alone: Fleet's friction, not the Drone's.
    for note in from
        .log
        .iter()
        .filter(|note| note.msg == lines::A_RED_RUN_ALONE)
    {
        let failed = field(note, "failed").unwrap_or("?");
        let alone = match field(note, "passed_alone") {
            Some("true") => "passed",
            _ => "failed",
        };
        let first = match field(note, "one_by_one") {
            Some("true") => format!("{failed} tests failed, and each passed run alone"),
            _ => format!("{failed} tests failed, too many to run one by one"),
        };
        out.push(RecordCheck {
            cite: String::new(),
            at: Some(note.at.clone()),
            step: note.step.clone(),
            attempt: field(note, "attempt").and_then(|n| n.parse().ok()),
            name: field(note, "check").unwrap_or("?").to_string(),
            run: CheckRunBy::Gate,
            expected: Some(String::from("passes with other checks running beside it")),
            produced: Some(format!(
                "{first}; the whole check run again alone {alone}, and that run was ruled on"
            )),
            paths: Vec::new(),
        });
    }
    for (n, row) in out.iter_mut().enumerate() {
        row.cite = cite("check", n);
        if row.run == CheckRunBy::Gate {
            row.paths = paths_named(row, from.transcripts);
        }
    }
    out
}

/// Each file a gate failure names, and whether a tool call of the step's
/// Drones names it too. **Read off the text and the transcript**: a failure
/// that reaches a file the Drone's calls never name is a failure the Drone did
/// not cause, and the retro is handed that as a fact instead of being left to
/// work it out. A `true` says a call names the file, which may be a read, and
/// a shell command that wrote it by a glob names nothing.
fn paths_named(check: &RecordCheck, transcripts: &[Vec<TranscriptRow>]) -> Vec<RecordPath> {
    let calls: Vec<String> = transcripts
        .iter()
        .flatten()
        .filter(|row| check.step.is_none() || row.step.is_none() || row.step == check.step)
        .filter_map(|row| match &row.saw {
            Saw::Called { detail, whole, .. } => Some(match whole {
                Some(whole) => format!("{detail}\n{whole}"),
                None => detail.clone(),
            }),
            _ => None,
        })
        .collect();
    let said = format!(
        "{} {}",
        check.expected.as_deref().unwrap_or_default(),
        check.produced.as_deref().unwrap_or_default()
    );
    let mut seen = BTreeSet::new();
    said.split(|c: char| c.is_whitespace() || "`'\"(),;:".contains(c))
        .map(|token| token.trim_end_matches('.'))
        .filter(|token| looks_like_a_path(token))
        .filter(|token| seen.insert(token.to_string()))
        .map(|path| RecordPath {
            path: path.to_string(),
            named_in_drone_calls: calls.iter().any(|call| call.contains(path)),
        })
        .collect()
}

/// A word that is a path: one with a directory in it, or a name with an
/// extension that has a letter in it. `e.g` and `1.5` are neither.
pub(crate) fn looks_like_a_path(token: &str) -> bool {
    if token.contains('/') {
        return token.len() > 2 && token.chars().any(char::is_alphanumeric);
    }
    match token.rsplit_once('.') {
        Some((stem, extension)) => {
            stem.chars().filter(|c| c.is_alphanumeric()).count() >= 2
                && (1..=5).contains(&extension.len())
                && extension.chars().all(char::is_alphanumeric)
                && extension.chars().any(char::is_alphabetic)
        }
        None => false,
    }
}

fn not_met(detail: &JobDetail) -> Vec<RecordNotMet> {
    let mut out = Vec::new();
    for step in &detail.steps {
        for judged in &step.judged {
            if judged.verdict.domain() != core_model::JudgeVerdict::NotMet {
                continue;
            }
            out.push(RecordNotMet {
                cite: cite("not_met", out.len()),
                step: step.step_id.clone(),
                attempt: judged.attempt,
                criterion: judged.criterion_id.as_str().to_string(),
                expected: judged.expected.clone(),
                produced: judged.produced.clone(),
            });
        }
    }
    out
}

fn not_done(evidence: &JobEvidence) -> Vec<RecordSaid> {
    evidence
        .steps
        .iter()
        .filter_map(|step| {
            let said = step.not_claimed.as_deref()?.trim();
            (!said.is_empty()).then(|| (step.step_id.clone(), cut(said, SAID)))
        })
        .enumerate()
        .map(|(n, (step, said))| RecordSaid {
            cite: cite("not_done", n),
            at: None,
            step: Some(step),
            said,
        })
        .collect()
}

/// What a Drone said in prose, where the row is one and says something.
fn spoke(row: &TranscriptRow) -> Option<&str> {
    match (&row.saw, row.by) {
        (Saw::Said { text }, Voice::Drone) if !text.trim().is_empty() => Some(text.trim()),
        _ => None,
    }
}

/// What each Drone said in prose right after each submission, and the last
/// thing it said: where a Drone tells a person what it could not get to, after
/// a submission that had no field for it.
fn said_after(transcripts: &[Vec<TranscriptRow>]) -> Vec<RecordSaid> {
    let mut out: Vec<RecordSaid> = Vec::new();
    for rows in transcripts {
        let mut kept: Vec<&TranscriptRow> = Vec::new();
        let mut submitted = false;
        for row in rows {
            if let Saw::Called { tool, .. } = &row.saw {
                submitted |= tool.ends_with(ipc::mcp::TOOL);
            } else if submitted && spoke(row).is_some() {
                kept.push(row);
                submitted = false;
            }
        }
        if let Some(last) = rows.iter().rev().find(|row| spoke(row).is_some()) {
            if !kept.iter().any(|row| std::ptr::eq(*row, last)) {
                kept.push(last);
            }
        }
        for row in kept {
            out.push(RecordSaid {
                cite: cite("said", out.len()),
                at: Some(row.ts.clone()),
                step: row.step.clone(),
                said: cut(spoke(row).unwrap_or_default(), SAID),
            });
        }
    }
    out
}

fn by_a_person(actor: Actor) -> bool {
    matches!(
        actor.domain(),
        core_model::Actor::Human | core_model::Actor::Helm
    )
}

/// One move, in a few words.
fn moved(row: &ipc::Recorded) -> String {
    match &row.moved {
        Movement::Status(to) => format!("status {}", to.to.as_wire()),
        Movement::Step(step) => format!(
            "step {} {} to {}",
            step.step_id.as_str(),
            step.from.as_wire(),
            step.to.as_wire()
        ),
        Movement::Drone(drone) => format!(
            "drone {} {}",
            drone.drone_id.as_str(),
            drone.presence.as_wire()
        ),
    }
}

fn act(kind: &str, n: usize, row: &ipc::Recorded, said: Option<String>) -> RecordAct {
    RecordAct {
        cite: cite(kind, n),
        at: row.at.clone(),
        actor: row.actor,
        via: row.via,
        moved: moved(row),
        said,
    }
}

fn acts(history: &JobHistory) -> Vec<RecordAct> {
    history
        .moves
        .iter()
        .filter(|row| by_a_person(row.actor))
        .enumerate()
        .map(|(n, row)| act("act", n, row, None))
        .collect()
}

/// A person sending a held Job round again — re-queued, a stopped step run
/// again — and a Job replaced by a redispatch, with what the person said where
/// the log kept it.
fn restarts(history: &JobHistory, log: &[LogNote]) -> Vec<RecordAct> {
    let mut out = Vec::new();
    for row in &history.moves {
        let again = match &row.moved {
            Movement::Status(to) => match to.to.as_wire() {
                "superseded" => true,
                "queued" => by_a_person(row.actor) && row.status.as_wire() != "awaiting_approval",
                _ => false,
            },
            Movement::Step(step) => {
                by_a_person(row.actor)
                    && step.from.as_wire() == "stopped"
                    && step.to.as_wire() == "running"
            }
            Movement::Drone(_) => false,
        };
        if !again {
            continue;
        }
        let said = log
            .iter()
            .find(|note| {
                note.msg.starts_with(lines::A_PERSON_RESTARTED)
                    && apart(&note.at, &row.at).max(apart(&row.at, &note.at)) < 2_000
            })
            .map(|note| note.msg.clone());
        out.push(act("restart", out.len(), row, said));
    }
    out
}

/// Each time a person was asked something, what about, and how long until
/// they answered.
fn asked(log: &[LogNote]) -> Vec<RecordAsked> {
    use lines::*;
    let kinds: [(&str, &[&str], &str, Option<&str>); 3] = [
        (
            A_DRONE_ASKS,
            &[A_PERSON_ANSWERS_A_DRONE],
            "question_id",
            Some("chose"),
        ),
        (
            A_JUDGE_ASKS,
            &[
                A_PERSON_AGREES,
                A_PERSON_DISAGREES_ONCE,
                A_PERSON_DISAGREES_ALWAYS,
            ],
            "criterion_id",
            None,
        ),
        (
            A_COMMAND_ASKS,
            &[A_PERSON_ANSWERS_A_COMMAND],
            "call",
            Some("answer"),
        ),
    ];
    let mut out = Vec::new();
    for (at, note) in log.iter().enumerate() {
        let Some((_, answers, key, answer_field)) =
            kinds.iter().find(|(asks, ..)| note.msg == *asks)
        else {
            continue;
        };
        let about = field(note, "question")
            .or_else(|| field(note, "command"))
            .or_else(|| field(note, key))
            .map(|what| format!("{}: {}", note.msg, cut(what, TRIED)))
            .unwrap_or_else(|| note.msg.clone());
        let answered = log[at + 1..].iter().find(|later| {
            answers.contains(&later.msg.as_str())
                && later.step == note.step
                && field(later, key) == field(note, key)
        });
        out.push(RecordAsked {
            cite: cite("asked", out.len()),
            asked_at: note.at.clone(),
            step: note.step.clone(),
            about,
            answered_at: answered.map(|later| later.at.clone()),
            answer: answered.map(
                |later| match answer_field.and_then(|name| field(later, name)) {
                    Some(said) => said.to_string(),
                    None => later.msg.clone(),
                },
            ),
            waited_ms: answered.map(|later| apart(&note.at, &later.at)),
        });
    }
    out
}

/// Each stretch at an `awaiting_*` status, from the move that entered it to
/// the one that left it.
fn waited(history: &JobHistory) -> Vec<RecordWaited> {
    let statuses: Vec<(&Instant, ipc::JobStatus)> = history
        .moves
        .iter()
        .filter_map(|row| match &row.moved {
            Movement::Status(to) => Some((&row.at, to.to)),
            _ => None,
        })
        .collect();
    statuses
        .iter()
        .enumerate()
        .filter(|(_, (_, status))| status.as_wire().starts_with("awaiting_"))
        .enumerate()
        .map(|(n, (at, (from, status)))| {
            let until = statuses.get(at + 1).map(|(until, _)| (*until).clone());
            RecordWaited {
                cite: cite("waited", n),
                status: *status,
                from: (*from).clone(),
                ms: until.as_ref().map(|until| apart(from, until)),
                until,
            }
        })
        .collect()
}

fn field<'a>(note: &'a LogNote, name: &str) -> Option<&'a str> {
    note.fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
}

/// Milliseconds from one instant to a later one. Zero where either will not
/// read or the order is backwards.
pub(super) fn apart(from: &Instant, to: &Instant) -> u64 {
    let millis = |at: &Instant| Timestamp::from_rfc3339(at.as_str()).epoch_millis();
    match (millis(from), millis(to)) {
        (Some(from), Some(to)) => u64::try_from(to - from).unwrap_or(0),
        _ => 0,
    }
}
