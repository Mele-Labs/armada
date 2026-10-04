//! A Job's retro, and the two facts Fleet keeps for one that nothing else
//! kept: which door each move came through, and what a Drone said got in its
//! way. `docs/concepts/retro.md`.
//!
//! **Everything else a retro reads is already on the record.** Refusals,
//! failed Checks, a Judge's `not_met`, restarts and a person's waits are rows
//! and files that exist for their own reasons; Fleet reads them when it writes
//! a retro and keeps no second copy.

use core_model::{JobId, JobStatus, LandsIn, StepId, Timestamp, Via, Whose};

use crate::error::{fault, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, enum_value};

/// Version 100 — the door a move came through, a Drone's note on what got in
/// its way, and a Job's retro.
///
/// **Every Job already ended is marked `skipped`.** A retro is owed by a Job
/// that ends from here on, and without the mark the first turn after this
/// migration would spend one model call on every Job the file has finished.
pub(crate) const V100: &str = r#"
CREATE TABLE job_event_via (
    seq    INTEGER PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(job_id),
    via    TEXT NOT NULL CHECK (via IN ('bridge', 'helm', 'door', 'http'))
) STRICT;

CREATE TABLE job_drone_notes (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    said    TEXT NOT NULL CHECK (trim(said) <> ''),
    at      TEXT NOT NULL
) STRICT;

CREATE TABLE job_retros (
    job_id TEXT PRIMARY KEY REFERENCES jobs(job_id),
    state  TEXT NOT NULL CHECK (state IN ('written', 'failed', 'skipped')),
    model  TEXT CHECK ((state = 'written') = (model IS NOT NULL)),
    why    TEXT CHECK ((state = 'written') = (why IS NULL)),
    at     TEXT NOT NULL
) STRICT;

CREATE TABLE job_retro_items (
    job_id   TEXT NOT NULL REFERENCES jobs(job_id),
    ordinal  INTEGER NOT NULL CHECK (ordinal >= 0),
    whose    TEXT NOT NULL CHECK (whose IN ('drone', 'owner', 'fleet')),
    said     TEXT NOT NULL CHECK (trim(said) <> ''),
    evidence TEXT NOT NULL,
    PRIMARY KEY (job_id, ordinal)
) STRICT;

INSERT INTO job_retros (job_id, state, why, at)
SELECT job_id, 'skipped', 'it ended before Fleet wrote retros',
       strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM jobs
WHERE status IN ('completed_success', 'completed_failed', 'killed', 'rejected', 'superseded');
"#;

/// Version 102 — where each retro item's fix lands. **Nothing is backfilled**:
/// a null is an item written before the model was asked, and guessing one
/// after the fact would put it under a place nobody chose.
pub(crate) const V102: &str = r#"
ALTER TABLE job_retro_items ADD COLUMN lands_in TEXT
    CHECK (lands_in IS NULL OR lands_in IN ('armada', 'kit', 'manifest'));
"#;

/// One item of a retro: what got in the way, whose way, and the record rows
/// that show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetroLine {
    pub whose: Whose,
    pub said: String,
    /// References into the Job's assembled record, `refusal:1` and the like.
    /// **Never empty on a written item**: Fleet drops an item that cites
    /// nothing before it is kept.
    pub evidence: Vec<String>,
    /// Where its fix lands. **`None` only on an item kept before V102**: Fleet
    /// drops a written item that names none.
    pub lands_in: Option<LandsIn>,
}

/// What became of a Job's retro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reflected {
    /// The model wrote one. `items` may be empty: a Job nothing got in the way
    /// of is a real answer.
    Written {
        model: String,
        items: Vec<RetroLine>,
    },
    /// The call failed, or its answer would not read. Not tried again.
    Failed { why: String },
    /// None was owed: the Job ended before retros, or no Drone ever ran on it.
    Skipped { why: String },
}

/// A Job's retro as it was kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptRetro {
    pub reflected: Reflected,
    pub at: Timestamp,
}

/// One item, with the Job and retro it came from. The Lessons page's row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptLesson {
    pub job_id: JobId,
    pub at: Timestamp,
    pub line: RetroLine,
}

/// What a Drone said got in its way, on one submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneNote {
    pub step_id: StepId,
    pub said: String,
    pub at: Timestamp,
}

impl Store {
    /// Keep the door the move at `seq` came through.
    pub fn record_via(&mut self, job_id: &JobId, seq: i64, via: Via) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_event_via (seq, job_id, via) VALUES (?1, ?2, ?3)",
                (seq, job_id.as_str(), via.as_wire()),
            )
            .map_err(fault("keeping the door a move came through"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every door this Job's moves came through, by `seq`. A move with none is
    /// Fleet's own, or older than the column.
    pub fn vias_for(&self, job_id: &JobId) -> Result<Vec<(i64, Via)>, RowError> {
        self.rows(
            "SELECT seq, via FROM job_event_via WHERE job_id = ?1 ORDER BY seq",
            job_id,
            |row| {
                let seq: i64 = row.get("seq").map_err(column("job_event_via", "seq"))?;
                let via: String = row.get("via").map_err(column("job_event_via", "via"))?;
                Ok((
                    seq,
                    enum_value(Via::from_wire, "job_event_via", "via", &via)?,
                ))
            },
        )
    }

    /// Keep what a Drone said got in its way. Blank is not kept: the field is
    /// optional and an empty one said nothing.
    pub fn record_drone_note(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        said: &str,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        if said.trim().is_empty() {
            return Ok(());
        }
        self.conn
            .execute(
                "INSERT INTO job_drone_notes (job_id, step_id, said, at) VALUES (?1, ?2, ?3, ?4)",
                (job_id.as_str(), step_id.as_str(), said.trim(), at.as_str()),
            )
            .map_err(fault("keeping what a Drone said got in its way"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every note this Job's Drones left, in the order they left them.
    pub fn drone_notes_for(&self, job_id: &JobId) -> Result<Vec<DroneNote>, RowError> {
        self.rows(
            "SELECT step_id, said, at FROM job_drone_notes WHERE job_id = ?1 ORDER BY rowid",
            job_id,
            |row| {
                let text = |name: &'static str| -> Result<String, RowError> {
                    row.get(name).map_err(column("job_drone_notes", name))
                };
                Ok(DroneNote {
                    step_id: StepId::new(text("step_id")?),
                    said: text("said")?,
                    at: Timestamp::from_rfc3339(text("at")?),
                })
            },
        )
    }

    /// Every ended Job with no retro kept, oldest first.
    pub fn retros_owed(&self) -> Result<Vec<JobId>, RowError> {
        let ended: Vec<String> = JobStatus::ALL
            .iter()
            .filter(|status| status.is_terminal())
            .map(|status| format!("'{}'", status.as_wire()))
            .collect();
        let sql = format!(
            "SELECT job_id FROM jobs WHERE status IN ({}) \
             AND job_id NOT IN (SELECT job_id FROM job_retros) \
             ORDER BY created_at, job_id",
            ended.join(", ")
        );
        let mut statement = self
            .conn
            .prepare(&sql)
            .map_err(fault("preparing the retros owed"))
            .map_err(RowError::Database)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(fault("reading the retros owed"))
            .map_err(RowError::Database)?;
        let mut owed = Vec::new();
        for id in ids {
            let id = id
                .map_err(fault("reading a Job owed a retro"))
                .map_err(RowError::Database)?;
            owed.push(JobId::carried(core_model::Ulid::carried(id)));
        }
        Ok(owed)
    }

    /// Keep a Job's retro, replacing any kept before.
    pub fn record_retro(
        &mut self,
        job_id: &JobId,
        reflected: &Reflected,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let (state, model, why, items): (&str, Option<&str>, Option<&str>, &[RetroLine]) =
            match reflected {
                Reflected::Written { model, items } => ("written", Some(model), None, items),
                Reflected::Failed { why } => ("failed", None, Some(why), &[]),
                Reflected::Skipped { why } => ("skipped", None, Some(why), &[]),
            };
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the retro"))
            .map_err(WriteError::Database)?;
        tx.execute(
            "DELETE FROM job_retro_items WHERE job_id = ?1",
            (job_id.as_str(),),
        )
        .map_err(fault("clearing a retro's items"))
        .map_err(WriteError::Database)?;
        tx.execute(
            "INSERT OR REPLACE INTO job_retros (job_id, state, model, why, at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (job_id.as_str(), state, model, why, at.as_str()),
        )
        .map_err(fault("keeping a retro"))
        .map_err(WriteError::Database)?;
        for (ordinal, item) in items.iter().enumerate() {
            tx.execute(
                "INSERT INTO job_retro_items (job_id, ordinal, whose, said, evidence, lands_in) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (
                    job_id.as_str(),
                    ordinal as i64,
                    item.whose.as_wire(),
                    item.said.as_str(),
                    item.evidence.join("\n"),
                    item.lands_in.map(|lands| lands.as_wire()),
                ),
            )
            .map_err(fault("keeping a retro's item"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing a retro"))
            .map_err(WriteError::Database)
    }

    /// The retro kept for this Job, or `None` where none has been.
    pub fn retro_for(&self, job_id: &JobId) -> Result<Option<KeptRetro>, RowError> {
        let kept = self.rows(
            "SELECT state, model, why, at FROM job_retros WHERE job_id = ?1",
            job_id,
            |row| {
                let text = |name: &'static str| -> Result<Option<String>, RowError> {
                    row.get(name).map_err(column("job_retros", name))
                };
                Ok((
                    text("state")?.unwrap_or_default(),
                    text("model")?,
                    text("why")?.unwrap_or_default(),
                    Timestamp::from_rfc3339(text("at")?.unwrap_or_default()),
                ))
            },
        )?;
        let Some((state, model, why, at)) = kept.into_iter().next() else {
            return Ok(None);
        };
        let reflected = match (state.as_str(), model) {
            ("written", Some(model)) => Reflected::Written {
                model,
                items: self.retro_lines(job_id)?,
            },
            ("failed", _) => Reflected::Failed { why },
            ("skipped", _) => Reflected::Skipped { why },
            (other, _) => {
                return Err(RowError::UnknownEnumValue {
                    table: "job_retros",
                    column: "state",
                    value: other.to_string(),
                })
            }
        };
        Ok(Some(KeptRetro { reflected, at }))
    }

    /// Up to `most` items across every written retro, newest retro first and
    /// each retro's items in the order they were written. `lands_in` narrows
    /// to the items whose fix lands there, and **an item kept before V102
    /// matches none**: it is listed only where nothing narrows.
    pub fn lessons(
        &self,
        most: u32,
        lands_in: Option<LandsIn>,
    ) -> Result<Vec<KeptLesson>, RowError> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT i.job_id, r.at, i.whose, i.said, i.evidence, i.lands_in \
                 FROM job_retro_items AS i JOIN job_retros AS r ON r.job_id = i.job_id \
                 WHERE r.state = 'written' AND (?2 IS NULL OR i.lands_in = ?2) \
                 ORDER BY r.at DESC, i.job_id, i.ordinal LIMIT ?1",
            )
            .map_err(fault("preparing the lessons"))
            .map_err(RowError::Database)?;
        let rows = statement
            .query_map(
                (i64::from(most), lands_in.map(|lands| lands.as_wire())),
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .map_err(fault("reading the lessons"))
            .map_err(RowError::Database)?;
        let mut lessons = Vec::new();
        for row in rows {
            let (job_id, at, whose, said, evidence, lands_in) = row
                .map_err(fault("reading a lesson"))
                .map_err(RowError::Database)?;
            lessons.push(KeptLesson {
                job_id: JobId::carried(core_model::Ulid::carried(job_id)),
                at: Timestamp::from_rfc3339(at),
                line: line_of(&whose, said, &evidence, lands_in.as_deref())?,
            });
        }
        Ok(lessons)
    }

    fn retro_lines(&self, job_id: &JobId) -> Result<Vec<RetroLine>, RowError> {
        self.rows(
            "SELECT whose, said, evidence, lands_in FROM job_retro_items WHERE job_id = ?1 \
             ORDER BY ordinal",
            job_id,
            |row| {
                let text = |name: &'static str| -> Result<String, RowError> {
                    row.get(name).map_err(column("job_retro_items", name))
                };
                let lands_in: Option<String> = row
                    .get("lands_in")
                    .map_err(column("job_retro_items", "lands_in"))?;
                line_of(
                    &text("whose")?,
                    text("said")?,
                    &text("evidence")?,
                    lands_in.as_deref(),
                )
            },
        )
    }

    /// Every row one query over one Job answers, each read by `read`.
    fn rows<T>(
        &self,
        sql: &str,
        job_id: &JobId,
        read: impl Fn(&rusqlite::Row<'_>) -> Result<T, RowError>,
    ) -> Result<Vec<T>, RowError> {
        let mut statement = self
            .conn
            .prepare(sql)
            .map_err(fault("preparing a retro read"))
            .map_err(RowError::Database)?;
        let mut rows = statement
            .query((job_id.as_str(),))
            .map_err(fault("reading a retro row"))
            .map_err(RowError::Database)?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(fault("reading a retro row"))
            .map_err(RowError::Database)?
        {
            out.push(read(row)?);
        }
        Ok(out)
    }
}

fn line_of(
    whose: &str,
    said: String,
    evidence: &str,
    lands_in: Option<&str>,
) -> Result<RetroLine, RowError> {
    Ok(RetroLine {
        whose: enum_value(Whose::from_wire, "job_retro_items", "whose", whose)?,
        lands_in: lands_in
            .map(|lands| enum_value(LandsIn::from_wire, "job_retro_items", "lands_in", lands))
            .transpose()?,
        said,
        evidence: evidence
            .lines()
            .filter(|cited| !cited.is_empty())
            .map(str::to_string)
            .collect(),
    })
}
