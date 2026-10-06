//! A Job's retro, and the two facts Fleet keeps for one that nothing else
//! kept: which door each move came through, and what a Drone said got in its
//! way. `docs/concepts/retro.md`.
//!
//! **Everything else a retro reads is already on the record.** Refusals,
//! failed Checks, a Judge's `not_met`, restarts and a person's waits are rows
//! and files that exist for their own reasons; Fleet reads them when it writes
//! a retro and keeps no second copy.

use core_model::{Change, JobId, JobStatus, LandsIn, LessonState, StepId, Timestamp, Via, Whose};

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

/// Version 104 — an item's headline, what happened and what to change, and
/// where it stands with the person.
///
/// **The three texts are null on every item kept before**, which has `said`
/// alone, and nothing is backfilled. **Every existing item starts `open`.**
pub(crate) const V105: &str = r#"
ALTER TABLE job_retro_items ADD COLUMN title TEXT CHECK (title IS NULL OR trim(title) <> '');
ALTER TABLE job_retro_items ADD COLUMN what TEXT CHECK (what IS NULL OR trim(what) <> '');
ALTER TABLE job_retro_items ADD COLUMN fix TEXT CHECK (fix IS NULL OR trim(fix) <> '');
ALTER TABLE job_retro_items ADD COLUMN state TEXT NOT NULL DEFAULT 'open'
    CHECK (state IN ('open', 'agreed', 'accepted', 'discarded'));
ALTER TABLE job_retro_items ADD COLUMN job_proposed TEXT;
"#;

/// Version 109 — the change to Kit an item carries, and whether pressing
/// Accept applied it.
///
/// **Null on every item kept before, and nothing is backfilled**: a change is
/// copied off a refusal the record showed, and an item written before the model
/// was asked has none to copy. `applied` is what makes a second press apply
/// nothing, and is only ever set by the same write that moves the item to
/// `accepted`.
pub(crate) const V110: &str = r#"
ALTER TABLE job_retro_items ADD COLUMN change_kind TEXT
    CHECK (change_kind IS NULL OR change_kind IN ('allow_command'));
ALTER TABLE job_retro_items ADD COLUMN change_command TEXT
    CHECK (change_command IS NULL OR trim(change_command) <> '');
ALTER TABLE job_retro_items ADD COLUMN applied INTEGER NOT NULL DEFAULT 0
    CHECK (applied IN (0, 1));
"#;

/// One item of a retro: what got in the way, whose way, and the record rows
/// that show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetroLine {
    pub whose: Whose,
    /// A headline. **`None` only on an item kept before V105**, as `what` and
    /// `fix` are.
    pub title: Option<String>,
    pub what: Option<String>,
    pub fix: Option<String>,
    pub said: String,
    /// References into the Job's assembled record, `refusal:1` and the like.
    /// **Never empty on a written item**: Fleet drops an item that cites
    /// nothing before it is kept.
    pub evidence: Vec<String>,
    /// Where its fix lands. **`None` only on an item kept before V102**: Fleet
    /// drops a written item that names none.
    pub lands_in: Option<LandsIn>,
    /// The change to Kit that pressing Accept applies. **Only on a Kit item**,
    /// and only where Fleet copied it off a refusal the record shows. Since
    /// V110.
    pub change: Option<Change>,
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
    /// The item's place in its retro. With `job_id` it is the item's id.
    pub ordinal: u32,
    pub at: Timestamp,
    pub line: RetroLine,
    pub state: LessonState,
    /// The Job proposed for it, once one was.
    pub job_proposed: Option<JobId>,
    /// Whether Accept applied the item's change. **Never true of an item with
    /// none.** Since V110.
    pub applied: bool,
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
                "INSERT INTO job_retro_items \
                 (job_id, ordinal, whose, said, evidence, lands_in, title, what, fix, \
                  change_kind, change_command) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                (
                    job_id.as_str(),
                    ordinal as i64,
                    item.whose.as_wire(),
                    item.said.as_str(),
                    item.evidence.join("\n"),
                    item.lands_in.map(|lands| lands.as_wire()),
                    item.title.as_deref(),
                    item.what.as_deref(),
                    item.fix.as_deref(),
                    item.change.as_ref().map(Change::kind),
                    item.change.as_ref().map(|change| match change {
                        Change::AllowCommand { command } => command.as_str(),
                    }),
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
    /// matches none**: it is listed only where nothing narrows. `state`
    /// narrows to the items a person has answered that way.
    pub fn lessons(
        &self,
        most: u32,
        lands_in: Option<LandsIn>,
        state: Option<LessonState>,
    ) -> Result<Vec<KeptLesson>, RowError> {
        let sql = format!(
            "SELECT {LESSON_COLUMNS} \
             FROM job_retro_items AS i JOIN job_retros AS r ON r.job_id = i.job_id \
             WHERE r.state = 'written' AND (?2 IS NULL OR i.lands_in = ?2) \
             AND (?3 IS NULL OR i.state = ?3) \
             ORDER BY r.at DESC, i.job_id, i.ordinal LIMIT ?1"
        );
        self.lessons_where(
            &sql,
            (
                i64::from(most),
                lands_in.map(|lands| lands.as_wire()),
                state.map(|state| state.as_wire()),
            ),
        )
    }

    /// One item, by its Job and its place in the retro. `None` where there is
    /// none, or where the retro was not written.
    pub fn lesson(&self, job_id: &JobId, ordinal: u32) -> Result<Option<KeptLesson>, RowError> {
        let sql = format!(
            "SELECT {LESSON_COLUMNS} \
             FROM job_retro_items AS i JOIN job_retros AS r ON r.job_id = i.job_id \
             WHERE r.state = 'written' AND i.job_id = ?1 AND i.ordinal = ?2"
        );
        Ok(self
            .lessons_where(&sql, (job_id.as_str(), i64::from(ordinal)))?
            .into_iter()
            .next())
    }

    /// Move an item off `open`, and say whether this call did it. **The write
    /// is the claim**: two presses race on one row, one changes it and the
    /// other reads `false`, so a Job is proposed once however many press.
    pub fn answer_lesson(
        &mut self,
        job_id: &JobId,
        ordinal: u32,
        to: LessonState,
    ) -> Result<bool, WriteError> {
        let changed = self
            .conn
            .execute(
                "UPDATE job_retro_items SET state = ?3 \
                 WHERE job_id = ?1 AND ordinal = ?2 AND state = 'open'",
                (job_id.as_str(), i64::from(ordinal), to.as_wire()),
            )
            .map_err(fault("answering a lesson"))
            .map_err(WriteError::Database)?;
        Ok(changed == 1)
    }

    /// Move an item with a change off `open` to `accepted` and mark the change
    /// applied, in one write, and say whether this call did it. **The write is
    /// the claim**, `answer_lesson`'s rule: of two presses one reads `true`.
    /// An item with no change reads `false` and is left as it was.
    pub fn accept_lesson_applied(
        &mut self,
        job_id: &JobId,
        ordinal: u32,
    ) -> Result<bool, WriteError> {
        let changed = self
            .conn
            .execute(
                "UPDATE job_retro_items SET state = 'accepted', applied = 1 \
                 WHERE job_id = ?1 AND ordinal = ?2 AND state = 'open' \
                 AND change_kind IS NOT NULL",
                (job_id.as_str(), i64::from(ordinal)),
            )
            .map_err(fault("accepting a lesson and applying its change"))
            .map_err(WriteError::Database)?;
        Ok(changed == 1)
    }

    /// Keep the Job proposed for an agreed item.
    pub fn keep_lesson_job(
        &mut self,
        job_id: &JobId,
        ordinal: u32,
        proposed: &JobId,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_retro_items SET job_proposed = ?3 \
                 WHERE job_id = ?1 AND ordinal = ?2 AND state = 'agreed'",
                (job_id.as_str(), i64::from(ordinal), proposed.as_str()),
            )
            .map_err(fault("keeping the Job proposed for a lesson"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Give an agreed item back to `open`, where no Job could be proposed for
    /// it. **Only one that has none**: a Job that exists is never forgotten.
    pub fn reopen_lesson(&mut self, job_id: &JobId, ordinal: u32) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_retro_items SET state = 'open' \
                 WHERE job_id = ?1 AND ordinal = ?2 AND state = 'agreed' \
                 AND job_proposed IS NULL",
                (job_id.as_str(), i64::from(ordinal)),
            )
            .map_err(fault("giving a lesson back"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    fn lessons_where(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<KeptLesson>, RowError> {
        let mut statement = self
            .conn
            .prepare(sql)
            .map_err(fault("preparing the lessons"))
            .map_err(RowError::Database)?;
        let rows = statement
            .query_map(params, |row| {
                Ok(Stored {
                    job_id: row.get(0)?,
                    ordinal: row.get(1)?,
                    at: row.get(2)?,
                    whose: row.get(3)?,
                    said: row.get(4)?,
                    evidence: row.get(5)?,
                    lands_in: row.get(6)?,
                    texts: Texts {
                        title: row.get(7)?,
                        what: row.get(8)?,
                        fix: row.get(9)?,
                        change_kind: row.get(12)?,
                        change_command: row.get(13)?,
                    },
                    state: row.get(10)?,
                    proposed: row.get(11)?,
                    applied: row.get::<_, i64>(14)? == 1,
                })
            })
            .map_err(fault("reading the lessons"))
            .map_err(RowError::Database)?;
        let mut lessons = Vec::new();
        for row in rows {
            let stored = row
                .map_err(fault("reading a lesson"))
                .map_err(RowError::Database)?;
            lessons.push(KeptLesson {
                job_id: JobId::carried(core_model::Ulid::carried(stored.job_id)),
                ordinal: u32::try_from(stored.ordinal).unwrap_or(0),
                at: Timestamp::from_rfc3339(stored.at),
                line: line_of(
                    &stored.whose,
                    stored.said,
                    &stored.evidence,
                    stored.lands_in.as_deref(),
                    stored.texts,
                )?,
                state: enum_value(
                    LessonState::from_wire,
                    "job_retro_items",
                    "state",
                    &stored.state,
                )?,
                job_proposed: stored
                    .proposed
                    .map(|id| JobId::carried(core_model::Ulid::carried(id))),
                applied: stored.applied,
            });
        }
        Ok(lessons)
    }

    fn retro_lines(&self, job_id: &JobId) -> Result<Vec<RetroLine>, RowError> {
        self.rows(
            "SELECT whose, said, evidence, lands_in, title, what, fix, change_kind, \
             change_command FROM job_retro_items \
             WHERE job_id = ?1 ORDER BY ordinal",
            job_id,
            |row| {
                let text = |name: &'static str| -> Result<String, RowError> {
                    row.get(name).map_err(column("job_retro_items", name))
                };
                let lands_in: Option<String> = row
                    .get("lands_in")
                    .map_err(column("job_retro_items", "lands_in"))?;
                let optional = |name: &'static str| -> Result<Option<String>, RowError> {
                    row.get(name).map_err(column("job_retro_items", name))
                };
                let texts = Texts {
                    title: optional("title")?,
                    what: optional("what")?,
                    fix: optional("fix")?,
                    change_kind: optional("change_kind")?,
                    change_command: optional("change_command")?,
                };
                line_of(
                    &text("whose")?,
                    text("said")?,
                    &text("evidence")?,
                    lands_in.as_deref(),
                    texts,
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

/// The columns [`Store::lessons`] and [`Store::lesson`] read, in the order
/// [`Store::lessons_where`] reads them.
const LESSON_COLUMNS: &str = "i.job_id, i.ordinal, r.at, i.whose, i.said, i.evidence, \
     i.lands_in, i.title, i.what, i.fix, i.state, i.job_proposed, i.change_kind, \
     i.change_command, i.applied";

/// The three texts V105 added, which are all absent on an older item.
struct Texts {
    title: Option<String>,
    what: Option<String>,
    fix: Option<String>,
    change_kind: Option<String>,
    change_command: Option<String>,
}

/// One row of [`LESSON_COLUMNS`], before its spellings are checked.
struct Stored {
    job_id: String,
    ordinal: i64,
    at: String,
    whose: String,
    said: String,
    evidence: String,
    lands_in: Option<String>,
    texts: Texts,
    state: String,
    proposed: Option<String>,
    applied: bool,
}

fn line_of(
    whose: &str,
    said: String,
    evidence: &str,
    lands_in: Option<&str>,
    texts: Texts,
) -> Result<RetroLine, RowError> {
    let change = match (texts.change_kind, texts.change_command) {
        (Some(kind), Some(command)) => Some(Change::from_parts(&kind, command).ok_or(
            RowError::UnknownEnumValue {
                table: "job_retro_items",
                column: "change_kind",
                value: kind,
            },
        )?),
        _ => None,
    };
    Ok(RetroLine {
        change,
        title: texts.title,
        what: texts.what,
        fix: texts.fix,
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
