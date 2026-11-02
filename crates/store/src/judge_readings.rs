//! A Job's Judge readings, one a `(kind, artifact)`, with their notes.
//! `docs/concepts/judge.md`; `docs/practices/protocol/a-job-is-judged-on-demand.md`.
//!
//! **Strings and numbers, not the wire's types**: this crate does not know `ipc`. A reading's state
//! is `reading`, `done` or `failed`, a note's cites are one per line, and a note settled is `sent` or
//! `dismissed` beside when.

use rusqlite::{params, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One reading of one artifact of a Job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptJudgeReading {
    /// Fleet's. Asking again for the same artifact is a new one.
    pub id: String,
    /// `job`, `plan`, `pull_request`, and the rest of the wire's kinds.
    pub kind: String,
    /// The kind's own id: a number, a path, an address, the plan's version.
    pub artifact_id: String,
    pub title: String,
    /// What the person asked. Empty asks what was missed.
    pub prompt: String,
    /// `reading`, `done` or `failed`.
    pub state: String,
    pub asked_at: String,
    pub finished_at: Option<String>,
    /// Why a failed reading failed, in Fleet's words.
    pub failed: Option<String>,
    /// The person opened the notes.
    pub read: bool,
    pub cost_micros: Option<u64>,
    pub notes: Vec<KeptJudgeNote>,
}

/// One note of a reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptJudgeNote {
    /// Unique within its reading.
    pub id: String,
    /// The part, in the artifact's own terms. None is the artifact as a whole.
    pub about: Option<String>,
    pub claim: String,
    /// The same part as a person reads it.
    pub place: Option<String>,
    pub expected: Option<String>,
    pub found: Option<String>,
    pub impact: Option<String>,
    pub cites: Vec<String>,
    /// `sent` or `dismissed`, and when. Absent while it waits.
    pub settled: Option<(String, String)>,
}

fn reading_fault(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a Judge reading")(why))
}

fn keeping_fault(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("keeping a Judge reading")(why))
}

const READING: &str = "id, kind, artifact_id, title, prompt, state, asked_at, finished_at, failed, read, cost_micros";
const NOTE: &str = "id, about, claim, place, expected, found, impact, cites, how, settled_at";

fn reading(row: &Row<'_>) -> rusqlite::Result<KeptJudgeReading> {
    Ok(KeptJudgeReading {
        id: row.get(0)?,
        kind: row.get(1)?,
        artifact_id: row.get(2)?,
        title: row.get(3)?,
        prompt: row.get(4)?,
        state: row.get(5)?,
        asked_at: row.get(6)?,
        finished_at: row.get(7)?,
        failed: row.get(8)?,
        read: row.get::<_, i64>(9)? != 0,
        cost_micros: row.get::<_, Option<i64>>(10)?.and_then(|micros| micros.try_into().ok()),
        notes: Vec::new(),
    })
}

fn note(row: &Row<'_>) -> rusqlite::Result<KeptJudgeNote> {
    let cites: String = row.get(7)?;
    let how: Option<String> = row.get(8)?;
    let at: Option<String> = row.get(9)?;
    Ok(KeptJudgeNote {
        id: row.get(0)?,
        about: row.get(1)?,
        claim: row.get(2)?,
        place: row.get(3)?,
        expected: row.get(4)?,
        found: row.get(5)?,
        impact: row.get(6)?,
        cites: cites.lines().filter(|line| !line.is_empty()).map(str::to_string).collect(),
        settled: how.zip(at),
    })
}

impl Store {
    /// Keep a reading whole, replacing the one kept for its `(kind, artifact)`, notes and all, in one
    /// transaction. A reading kept under the same `id` again (a state moving) is replaced the same way.
    pub fn keep_judge_reading(&mut self, session_id: &str, kept: &KeptJudgeReading) -> Result<(), WriteError> {
        let held = self.conn.transaction().map_err(keeping_fault)?;
        held.execute(
            "DELETE FROM judge_notes WHERE session_id = ?1 AND reading IN
                 (SELECT id FROM judge_readings WHERE session_id = ?1 AND kind = ?2 AND artifact_id = ?3)",
            params![session_id, kept.kind, kept.artifact_id],
        )
        .map_err(keeping_fault)?;
        held.execute(
            "DELETE FROM judge_readings WHERE session_id = ?1 AND kind = ?2 AND artifact_id = ?3",
            params![session_id, kept.kind, kept.artifact_id],
        )
        .map_err(keeping_fault)?;
        held.execute(
            &format!("INSERT INTO judge_readings (session_id, {READING}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"),
            params![
                session_id,
                kept.id,
                kept.kind,
                kept.artifact_id,
                kept.title,
                kept.prompt,
                kept.state,
                kept.asked_at,
                kept.finished_at,
                kept.failed,
                kept.read as i64,
                kept.cost_micros.and_then(|micros| i64::try_from(micros).ok()),
            ],
        )
        .map_err(keeping_fault)?;
        for (position, one) in kept.notes.iter().enumerate() {
            held.execute(
                &format!(
                    "INSERT INTO judge_notes (session_id, reading, position, {NOTE})
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"
                ),
                params![
                    session_id,
                    kept.id,
                    position as i64,
                    one.id,
                    one.about,
                    one.claim,
                    one.place,
                    one.expected,
                    one.found,
                    one.impact,
                    one.cites.join("\n"),
                    one.settled.as_ref().map(|(how, _)| how.as_str()),
                    one.settled.as_ref().map(|(_, at)| at.as_str()),
                ],
            )
            .map_err(keeping_fault)?;
        }
        held.commit().map_err(keeping_fault)
    }

    /// Every reading the Job holds, the oldest asked first.
    pub fn judge_readings(&self, session_id: &str) -> Result<Vec<KeptJudgeReading>, WriteError> {
        self.readings_where(session_id, "", None, None)
    }

    /// The reading `id` of the Job, if it still holds it. A reading asked again is gone.
    pub fn judge_reading(&self, session_id: &str, id: &str) -> Result<Option<KeptJudgeReading>, WriteError> {
        Ok(self.readings_where(session_id, "AND id = ?2", Some(id), None)?.pop())
    }

    /// The reading the Job holds of one artifact.
    pub fn judge_reading_of(&self, session_id: &str, kind: &str, artifact_id: &str) -> Result<Option<KeptJudgeReading>, WriteError> {
        Ok(self.readings_where(session_id, "AND kind = ?2 AND artifact_id = ?3", Some(kind), Some(artifact_id))?.pop())
    }

    fn readings_where(
        &self,
        session_id: &str,
        and: &str,
        first: Option<&str>,
        second: Option<&str>,
    ) -> Result<Vec<KeptJudgeReading>, WriteError> {
        let sql = format!("SELECT {READING} FROM judge_readings WHERE session_id = ?1 {and} ORDER BY asked_at, rowid");
        let mut read = self.conn.prepare(&sql).map_err(reading_fault)?;
        let mut kept: Vec<KeptJudgeReading> = match (first, second) {
            (None, _) => read.query_map(params![session_id], reading),
            (Some(first), None) => read.query_map(params![session_id, first], reading),
            (Some(first), Some(second)) => read.query_map(params![session_id, first, second], reading),
        }
        .map_err(reading_fault)?
        .collect::<Result<_, _>>()
        .map_err(reading_fault)?;
        let mut notes = self
            .conn
            .prepare(&format!("SELECT {NOTE} FROM judge_notes WHERE session_id = ?1 AND reading = ?2 ORDER BY position"))
            .map_err(reading_fault)?;
        for one in &mut kept {
            one.notes = notes
                .query_map(params![session_id, one.id], note)
                .map_err(reading_fault)?
                .collect::<Result<_, _>>()
                .map_err(reading_fault)?;
        }
        Ok(kept)
    }

    /// Settle note `note` of reading `reading` as `how` (`sent` or `dismissed`) at `at`, if it waits.
    /// Whether it did.
    pub fn settle_judge_note(&mut self, session_id: &str, reading: &str, note: &str, how: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE judge_notes SET how = ?4, settled_at = ?5
                 WHERE session_id = ?1 AND reading = ?2 AND id = ?3 AND how IS NULL",
                params![session_id, reading, note, how, at],
            )
            .map(|changed| changed == 1)
            .map_err(keeping_fault)
    }

    /// The person opened reading `reading`'s notes. Whether the Job holds that reading.
    pub fn mark_judge_read(&mut self, session_id: &str, reading: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE judge_readings SET read = 1 WHERE session_id = ?1 AND id = ?2", params![session_id, reading])
            .map(|changed| changed == 1)
            .map_err(keeping_fault)
    }
}
