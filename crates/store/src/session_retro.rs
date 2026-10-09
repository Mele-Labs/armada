//! A Session's retros and restarts. `docs/concepts/retro.md`, *A Session's
//! retro*.
//!
//! **Beside a Job's, not in it.** A Job has one retro keyed by its id; a
//! Session has several, each covering from where the last ended, so an item
//! is named by its retro's number and its place in it.

use core_model::{LessonState, Timestamp};
use rusqlite::{params, OptionalExtension};

use crate::error::{fault, RowError, WriteError};
use crate::open::Store;
use crate::retro::{line_of, RetroLine, Texts};
use crate::row::{column, enum_value};

/// A written retro of a Session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSessionRetro {
    pub retro_id: i64,
    pub session_id: String,
    /// The end of the retro before it. `None` on a Session's first.
    pub covers_from: Option<String>,
    pub covers_to: String,
    pub model: String,
    /// What the agent said got in its way when Fleet asked it.
    pub note: Option<String>,
    pub at: Timestamp,
    pub items: Vec<RetroLine>,
}

/// One item of a Session's retro, as the Lessons list reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSessionLesson {
    pub session_id: String,
    pub retro_id: i64,
    pub ordinal: u32,
    pub at: Timestamp,
    pub line: RetroLine,
    pub state: LessonState,
    pub job_proposed: Option<core_model::JobId>,
    pub applied: bool,
}

/// A process of a Session's coming back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptRestart {
    pub at: String,
    pub kind: String,
    pub said: Option<String>,
}

const COLUMNS: &str = "r.session_id, i.retro_id, i.ordinal, r.at, i.whose, i.said, i.evidence, \
     i.lands_in, i.title, i.what, i.fix, i.state, i.job_proposed, i.change_kind, \
     i.change_command, i.applied";

impl Store {
    /// Keep a written retro and say its number.
    pub fn record_session_retro(
        &mut self,
        session_id: &str,
        covers_from: Option<&str>,
        covers_to: &str,
        model: &str,
        note: Option<&str>,
        items: &[RetroLine],
        at: &Timestamp,
    ) -> Result<i64, WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting a session's retro"))
            .map_err(WriteError::Database)?;
        tx.execute(
            "INSERT INTO session_retros (session_id, covers_from, covers_to, model, note, at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, covers_from, covers_to, model, note, at.as_str()],
        )
        .map_err(fault("keeping a session's retro"))
        .map_err(WriteError::Database)?;
        let retro_id = tx.last_insert_rowid();
        for (ordinal, item) in items.iter().enumerate() {
            tx.execute(
                "INSERT INTO session_retro_items \
                 (retro_id, ordinal, whose, said, evidence, lands_in, title, what, fix, \
                  change_kind, change_command) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    retro_id,
                    ordinal as i64,
                    item.whose.as_wire(),
                    item.said,
                    item.evidence.join("\n"),
                    item.lands_in.map(|lands| lands.as_wire()),
                    item.title,
                    item.what,
                    item.fix,
                    item.change.as_ref().map(core_model::Change::kind),
                    item.change.as_ref().map(|change| match change {
                        core_model::Change::AllowCommand { command } => command.as_str(),
                    }),
                ],
            )
            .map_err(fault("keeping a session retro's item"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing a session's retro"))
            .map_err(WriteError::Database)?;
        Ok(retro_id)
    }

    /// The newest retro of a Session, with its items.
    pub fn latest_session_retro(
        &self,
        session_id: &str,
    ) -> Result<Option<KeptSessionRetro>, RowError> {
        let head = self
            .conn
            .query_row(
                "SELECT retro_id, covers_from, covers_to, model, at, note FROM session_retros \
                 WHERE session_id = ?1 ORDER BY retro_id DESC LIMIT 1",
                [session_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(fault("reading a session's retro"))
            .map_err(RowError::Database)?;
        let Some((retro_id, covers_from, covers_to, model, at, note)) = head else {
            return Ok(None);
        };
        let items = self
            .session_lessons_where(
                &format!("SELECT {COLUMNS} FROM session_retro_items AS i JOIN session_retros AS r ON r.retro_id = i.retro_id WHERE i.retro_id = ?1 ORDER BY i.ordinal"),
                params![retro_id],
            )?
            .into_iter()
            .map(|lesson| lesson.line)
            .collect();
        Ok(Some(KeptSessionRetro {
            retro_id,
            session_id: session_id.to_string(),
            covers_from,
            covers_to,
            model,
            note,
            at: Timestamp::from_rfc3339(at),
            items,
        }))
    }

    /// The items of a retro with their answers, in order.
    pub fn session_retro_lessons(
        &self,
        retro_id: i64,
    ) -> Result<Vec<KeptSessionLesson>, RowError> {
        self.session_lessons_where(
            &format!("SELECT {COLUMNS} FROM session_retro_items AS i JOIN session_retros AS r ON r.retro_id = i.retro_id WHERE i.retro_id = ?1 ORDER BY i.ordinal"),
            params![retro_id],
        )
    }

    /// Up to `most` items across every Session's retros, newest first, narrowed as
    /// [`Store::lessons`] is.
    pub fn session_lessons(
        &self,
        most: u32,
        lands_in: Option<core_model::LandsIn>,
        state: Option<LessonState>,
    ) -> Result<Vec<KeptSessionLesson>, RowError> {
        self.session_lessons_where(
            &format!(
                "SELECT {COLUMNS} FROM session_retro_items AS i \
                 JOIN session_retros AS r ON r.retro_id = i.retro_id \
                 WHERE (?2 IS NULL OR i.lands_in = ?2) AND (?3 IS NULL OR i.state = ?3) \
                 ORDER BY r.at DESC, i.retro_id DESC, i.ordinal LIMIT ?1"
            ),
            params![
                i64::from(most),
                lands_in.map(|lands| lands.as_wire()),
                state.map(|state| state.as_wire())
            ],
        )
    }

    /// One item, by its retro's number and its place.
    pub fn session_lesson(
        &self,
        retro_id: i64,
        ordinal: u32,
    ) -> Result<Option<KeptSessionLesson>, RowError> {
        Ok(self
            .session_lessons_where(
                &format!("SELECT {COLUMNS} FROM session_retro_items AS i JOIN session_retros AS r ON r.retro_id = i.retro_id WHERE i.retro_id = ?1 AND i.ordinal = ?2"),
                params![retro_id, i64::from(ordinal)],
            )?
            .into_iter()
            .next())
    }

    /// `answer_lesson`'s rule for a Session's item: the write is the claim.
    pub fn answer_session_lesson(
        &mut self,
        retro_id: i64,
        ordinal: u32,
        to: LessonState,
    ) -> Result<bool, WriteError> {
        self.session_item_write(
            "UPDATE session_retro_items SET state = ?3 \
             WHERE retro_id = ?1 AND ordinal = ?2 AND state = 'open'",
            params![retro_id, i64::from(ordinal), to.as_wire()],
        )
    }

    /// `accept_lesson_applied` for a Session's item.
    pub fn accept_session_lesson_applied(
        &mut self,
        retro_id: i64,
        ordinal: u32,
    ) -> Result<bool, WriteError> {
        self.session_item_write(
            "UPDATE session_retro_items SET state = 'accepted', applied = 1 \
             WHERE retro_id = ?1 AND ordinal = ?2 AND state = 'open' AND change_kind IS NOT NULL",
            params![retro_id, i64::from(ordinal)],
        )
    }

    /// Keep the Job proposed for an agreed Session item.
    pub fn keep_session_lesson_job(
        &mut self,
        retro_id: i64,
        ordinal: u32,
        proposed: &core_model::JobId,
    ) -> Result<(), WriteError> {
        self.session_item_write(
            "UPDATE session_retro_items SET job_proposed = ?3 \
             WHERE retro_id = ?1 AND ordinal = ?2 AND state = 'agreed'",
            params![retro_id, i64::from(ordinal), proposed.as_str()],
        )
        .map(|_| ())
    }

    /// Give an agreed Session item back to `open` where no Job came of it.
    pub fn reopen_session_lesson(&mut self, retro_id: i64, ordinal: u32) -> Result<(), WriteError> {
        self.session_item_write(
            "UPDATE session_retro_items SET state = 'open' \
             WHERE retro_id = ?1 AND ordinal = ?2 AND state = 'agreed' AND job_proposed IS NULL",
            params![retro_id, i64::from(ordinal)],
        )
        .map(|_| ())
    }

    /// Keep that a Session's process came back.
    pub fn keep_session_restart(
        &mut self,
        session_id: &str,
        at: &str,
        kind: &str,
        said: Option<&str>,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_restarts (session_id, at, kind, said) VALUES (?1, ?2, ?3, ?4)",
                params![session_id, at, kind, said],
            )
            .map(|_| ())
            .map_err(fault("keeping a session's restart"))
            .map_err(WriteError::Database)
    }

    /// Every restart of a Session, oldest first.
    pub fn session_restarts(&self, session_id: &str) -> Result<Vec<KeptRestart>, RowError> {
        let mut statement = self
            .conn
            .prepare("SELECT at, kind, said FROM session_restarts WHERE session_id = ?1 ORDER BY at")
            .map_err(fault("preparing a session's restarts"))
            .map_err(RowError::Database)?;
        let rows = statement
            .query_map([session_id], |row| {
                Ok(KeptRestart {
                    at: row.get(0)?,
                    kind: row.get(1)?,
                    said: row.get(2)?,
                })
            })
            .map_err(fault("reading a session's restarts"))
            .map_err(RowError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault("reading a session's restart"))
            .map_err(RowError::Database)
    }

    /// Every ended Session with no retro covering its end and none given up
    /// on, oldest first.
    pub fn sessions_owed_a_retro(&self) -> Result<Vec<String>, RowError> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT id FROM sessions WHERE state = 'ended' \
                 AND id NOT IN (SELECT session_id FROM session_retro_skips) \
                 AND NOT EXISTS (SELECT 1 FROM session_retros AS r \
                     WHERE r.session_id = sessions.id \
                     AND r.covers_to >= COALESCE(sessions.ended_at, '')) \
                 ORDER BY COALESCE(ended_at, started_at), id",
            )
            .map_err(fault("preparing the Sessions owed a retro"))
            .map_err(RowError::Database)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(fault("reading the Sessions owed a retro"))
            .map_err(RowError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault("reading a Session owed a retro"))
            .map_err(RowError::Database)
    }

    /// Give up on the retro an ended Session is owed.
    pub fn skip_session_retro(&mut self, session_id: &str, at: &Timestamp) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO session_retro_skips (session_id, at) VALUES (?1, ?2)",
                params![session_id, at.as_str()],
            )
            .map(|_| ())
            .map_err(fault("giving up on a session's retro"))
            .map_err(WriteError::Database)
    }

    fn session_item_write(
        &mut self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(sql, params)
            .map(|changed| changed == 1)
            .map_err(fault("answering a session's lesson"))
            .map_err(WriteError::Database)
    }

    fn session_lessons_where(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<KeptSessionLesson>, RowError> {
        let mut statement = self
            .conn
            .prepare(sql)
            .map_err(fault("preparing a session's lessons"))
            .map_err(RowError::Database)?;
        let mut rows = statement
            .query(params)
            .map_err(fault("reading a session's lessons"))
            .map_err(RowError::Database)?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(fault("reading a session's lesson"))
            .map_err(RowError::Database)?
        {
            let text = |at: usize, name: &'static str| -> Result<String, RowError> {
                row.get(at).map_err(column("session_retro_items", name))
            };
            let optional = |at: usize, name: &'static str| -> Result<Option<String>, RowError> {
                row.get(at).map_err(column("session_retro_items", name))
            };
            let texts = Texts {
                title: optional(8, "title")?,
                what: optional(9, "what")?,
                fix: optional(10, "fix")?,
                change_kind: optional(13, "change_kind")?,
                change_command: optional(14, "change_command")?,
            };
            let lands_in = optional(7, "lands_in")?;
            let state = text(11, "state")?;
            out.push(KeptSessionLesson {
                session_id: text(0, "session_id")?,
                retro_id: row.get(1).map_err(column("session_retro_items", "retro_id"))?,
                ordinal: u32::try_from(
                    row.get::<_, i64>(2)
                        .map_err(column("session_retro_items", "ordinal"))?,
                )
                .unwrap_or(0),
                at: Timestamp::from_rfc3339(text(3, "at")?),
                line: line_of(
                    &text(4, "whose")?,
                    text(5, "said")?,
                    &text(6, "evidence")?,
                    lands_in.as_deref(),
                    texts,
                )?,
                state: enum_value(
                    LessonState::from_wire,
                    "session_retro_items",
                    "state",
                    &state,
                )?,
                job_proposed: optional(12, "job_proposed")?
                    .map(|id| core_model::JobId::carried(core_model::Ulid::carried(id))),
                applied: row
                    .get::<_, i64>(15)
                    .map_err(column("session_retro_items", "applied"))?
                    == 1,
            });
        }
        Ok(out)
    }
}
