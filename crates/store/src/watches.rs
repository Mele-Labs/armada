//! Watches Fleet runs for a Job, their last [`READINGS_KEPT`] readings, the parts set aside, the
//! verdicts, and what was told of each. `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
//!
//! **Words, not decisions.** A state is the word `core_model::WatchState` spells; what a reading
//! comes to is Fleet's to work out, and nothing here reads one.
//!
//! **A notice is told once**: a row a (watch, what was said), as `pull_notices` keeps a pull
//! request's. `unsaid` names the notice a watch still owes, so one its Job could not take is asked
//! again on the next pass and a restart forgets neither.

use rusqlite::{params, OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// How many readings a watch keeps, the newest.
pub const READINGS_KEPT: i64 = 50;

/// One watch, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWatch {
    pub id: String,
    pub session_id: String,
    pub name: String,
    /// Whole argument items, every value filled.
    pub command: Vec<String>,
    pub recipe: Option<String>,
    /// The Requests environment whose variables the command is given.
    pub environment: Option<String>,
    pub every_seconds: u32,
    pub gives_up_after_seconds: u32,
    pub follows: Option<u64>,
    pub state: String,
    pub live: bool,
    pub started_at: String,
    /// Every part name a reading has carried.
    pub reported: Vec<String>,
    /// When what it reads last changed: its start, or a reading that read differently.
    pub changed_at: String,
    pub ended_at: Option<String>,
    /// The notice it owes its Job and has not told.
    pub unsaid: Option<String>,
    /// What a terminal watch waits for in its Job's terminal. Its `command` is empty.
    pub in_terminal: Option<String>,
    /// `in_terminal` is a regular expression.
    pub regex: bool,
}

/// What one reading was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptWatchTaken {
    Read {
        state: String,
        finished: bool,
        says: String,
        link: Option<String>,
        /// Each part's name and state word.
        parts: Vec<(String, String)>,
    },
    Broken { why: String, output: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWatchReading {
    pub at: String,
    pub taken: KeptWatchTaken,
    /// The file the run's whole output was kept in, for a reading that failed or broke.
    pub log: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWatchSetAside {
    pub part: String,
    pub sentence: String,
    pub at: String,
}

/// The agent's weighing of a failed reading, with its sentence. `ours` sets no parts aside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWatchVerdict {
    pub at: String,
    pub ours: bool,
    pub sentence: String,
    pub parts: Vec<String>,
}

/// A watch with everything kept of it: readings oldest first, verdicts in the order given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWatchWhole {
    pub watch: KeptWatch,
    pub readings: Vec<KeptWatchReading>,
    pub set_aside: Vec<KeptWatchSetAside>,
    pub verdicts: Vec<KeptWatchVerdict>,
}

const COLUMNS: &str = "id, session_id, name, command, recipe, environment, every_seconds, \
    gives_up_after_seconds, follows, state, live, started_at, reported, changed_at, ended_at, unsaid, \
    in_terminal, regex";

fn texts(written: String) -> Vec<String> {
    serde_json::from_str(&written).unwrap_or_default()
}

fn written(items: &[String]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".into())
}

fn written_parts(parts: &[(String, String)]) -> String {
    serde_json::to_string(parts).unwrap_or_else(|_| "[]".into())
}

fn watch_of(row: &Row<'_>) -> rusqlite::Result<KeptWatch> {
    Ok(KeptWatch {
        id: row.get(0)?,
        session_id: row.get(1)?,
        name: row.get(2)?,
        command: texts(row.get(3)?),
        recipe: row.get(4)?,
        environment: row.get(5)?,
        every_seconds: row.get(6)?,
        gives_up_after_seconds: row.get(7)?,
        follows: row.get::<_, Option<i64>>(8)?.map(|number| number as u64),
        state: row.get(9)?,
        live: row.get(10)?,
        started_at: row.get(11)?,
        reported: texts(row.get(12)?),
        changed_at: row.get(13)?,
        ended_at: row.get(14)?,
        unsaid: row.get(15)?,
        in_terminal: row.get(16)?,
        regex: row.get(17)?,
    })
}

impl Store {
    /// Keep `watch` whole, over what was kept of it.
    pub fn keep_watch(&mut self, watch: &KeptWatch) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT INTO watches ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18) \
                     ON CONFLICT (id) DO UPDATE SET state = ?10, live = ?11, reported = ?13, changed_at = ?14, \
                     ended_at = ?15, unsaid = ?16"
                ),
                params![
                    watch.id,
                    watch.session_id,
                    watch.name,
                    written(&watch.command),
                    watch.recipe,
                    watch.environment,
                    watch.every_seconds,
                    watch.gives_up_after_seconds,
                    watch.follows.map(|number| number as i64),
                    watch.state,
                    watch.live,
                    watch.started_at,
                    written(&watch.reported),
                    watch.changed_at,
                    watch.ended_at,
                    watch.unsaid,
                    watch.in_terminal,
                    watch.regex,
                ],
            )
            .map_err(fault("keeping a watch"))?;
        Ok(())
    }

    /// Add `reading` to `watch`'s, and forget all but the newest [`READINGS_KEPT`].
    pub fn keep_watch_reading(&mut self, watch: &str, reading: &KeptWatchReading) -> Result<(), WriteError> {
        let doing = "keeping a watch's reading";
        let tx = self.conn.transaction().map_err(fault(doing))?;
        let (kind, state, finished, says, link, parts, why, output) = match &reading.taken {
            KeptWatchTaken::Read { state, finished, says, link, parts } => {
                ("read", Some(state.clone()), Some(*finished), Some(says.clone()), link.clone(), Some(written_parts(parts)), None, None)
            }
            KeptWatchTaken::Broken { why, output } => {
                ("broken", None, None, None, None, None, Some(why.clone()), output.clone())
            }
        };
        tx.execute(
            "INSERT INTO watch_readings (watch_id, n, at, kind, state, finished, says, link, parts, why, output, log) \
             VALUES (?1, (SELECT COALESCE(MAX(n), 0) + 1 FROM watch_readings WHERE watch_id = ?1), ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![watch, reading.at, kind, state, finished, says, link, parts, why, output, reading.log],
        )
        .map_err(fault(doing))?;
        tx.execute(
            "DELETE FROM watch_readings WHERE watch_id = ?1 AND n <= \
             (SELECT MAX(n) FROM watch_readings WHERE watch_id = ?1) - ?2",
            params![watch, READINGS_KEPT],
        )
        .map_err(fault(doing))?;
        tx.commit().map_err(fault(doing))?;
        Ok(())
    }

    /// Set `aside.part` aside on `watch`, replacing the sentence it was set aside with before.
    pub fn keep_watch_set_aside(&mut self, watch: &str, aside: &KeptWatchSetAside) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO watch_set_aside (watch_id, part, sentence, at) VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT (watch_id, part) DO UPDATE SET sentence = ?3, at = ?4",
                params![watch, aside.part, aside.sentence, aside.at],
            )
            .map_err(fault("keeping a part set aside"))?;
        Ok(())
    }

    pub fn keep_watch_verdict(&mut self, watch: &str, verdict: &KeptWatchVerdict) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO watch_verdicts (watch_id, n, at, verdict, sentence, parts) \
                 VALUES (?1, (SELECT COALESCE(MAX(n), 0) + 1 FROM watch_verdicts WHERE watch_id = ?1), ?2, ?3, ?4, ?5)",
                params![
                    watch,
                    verdict.at,
                    if verdict.ours { "ours" } else { "not_ours" },
                    verdict.sentence,
                    written(&verdict.parts),
                ],
            )
            .map_err(fault("keeping a watch's verdict"))?;
        Ok(())
    }

    /// Keep that `said` was told of `watch`. **`false` where it already was.**
    pub fn keep_watch_notice(&mut self, watch: &str, said: &str, at: &str) -> Result<bool, WriteError> {
        let kept = self
            .conn
            .execute(
                "INSERT INTO watch_notices (watch_id, said, told_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                params![watch, said, at],
            )
            .map_err(fault("keeping a watch's notice"))?;
        Ok(kept == 1)
    }

    pub fn watch_notice_kept(&self, watch: &str, said: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM watch_notices WHERE watch_id = ?1 AND said = ?2)",
                params![watch, said],
                |row| row.get::<_, bool>(0),
            )
            .map_err(fault("reading a watch's notice"))
            .map_err(WriteError::Database)
    }

    pub fn watch_whole(&self, id: &str) -> Result<Option<KeptWatchWhole>, WriteError> {
        let doing = "reading a watch";
        let found = self
            .conn
            .query_row(&format!("SELECT {COLUMNS} FROM watches WHERE id = ?1"), [id], watch_of)
            .optional()
            .map_err(fault(doing))?;
        found.map(|watch| self.whole(watch)).transpose()
    }

    /// `session`'s watches, live and ended, the newest first.
    pub fn watches_of(&self, session: &str) -> Result<Vec<KeptWatchWhole>, WriteError> {
        self.watches_where("session_id = ?1 ORDER BY rowid DESC", Some(session))
    }

    /// `session`'s ended watches whose state is the word `state`, the oldest first. A Job's Needs you
    /// list reads it on every draw, so only the watches in that end are read whole.
    pub fn ended_watches_in(&self, session: &str, state: &str) -> Result<Vec<KeptWatchWhole>, WriteError> {
        let doing = "reading a Job's ended watches";
        let mut asking = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM watches WHERE session_id = ?1 AND live = 0 AND state = ?2 ORDER BY rowid"))
            .map_err(fault(doing))?;
        let watches = asking
            .query_map(params![session, state], watch_of)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))?;
        watches.into_iter().map(|watch| self.whole(watch)).collect()
    }

    /// Every live watch, the oldest first.
    pub fn live_watches(&self) -> Result<Vec<KeptWatchWhole>, WriteError> {
        self.watches_where("live = 1 ORDER BY rowid", None)
    }

    /// The name of the live watch of `session` that started first, for a row that says what holds the
    /// Job open. It reads no readings: a row is drawn far more often than a watch is read.
    pub fn first_live_watch_name(&self, session: &str) -> Result<Option<String>, WriteError> {
        Ok(self
            .conn
            .query_row(
                "SELECT name FROM watches WHERE session_id = ?1 AND live = 1 ORDER BY rowid LIMIT 1",
                [session],
                |row| row.get(0),
            )
            .optional()
            .map_err(fault("reading a Job's first live watch"))?)
    }

    /// Forget `id` and everything kept of it: readings, set-asides, verdicts and notices. `false`
    /// where nothing held it. Whether it may go, live or not, is Fleet's.
    pub fn delete_watch(&mut self, id: &str) -> Result<bool, WriteError> {
        let doing = "deleting a watch";
        let tx = self.conn.transaction().map_err(fault(doing))?;
        for table in ["watch_readings", "watch_set_aside", "watch_verdicts", "watch_notices"] {
            tx.execute(&format!("DELETE FROM {table} WHERE watch_id = ?1"), [id]).map_err(fault(doing))?;
        }
        let deleted = tx.execute("DELETE FROM watches WHERE id = ?1", [id]).map_err(fault(doing))?;
        tx.commit().map_err(fault(doing))?;
        Ok(deleted == 1)
    }

    /// Every watch that owes its Job a notice, the oldest first.
    pub fn unsaid_watches(&self) -> Result<Vec<KeptWatchWhole>, WriteError> {
        self.watches_where("unsaid IS NOT NULL ORDER BY rowid", None)
    }

    fn watches_where(&self, clause: &str, session: Option<&str>) -> Result<Vec<KeptWatchWhole>, WriteError> {
        let doing = "reading watches";
        let mut asking = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM watches WHERE {clause}"))
            .map_err(fault(doing))?;
        let watches = match session {
            Some(session) => asking.query_map([session], watch_of),
            None => asking.query_map([], watch_of),
        }
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(fault(doing))?;
        watches.into_iter().map(|watch| self.whole(watch)).collect()
    }

    fn whole(&self, watch: KeptWatch) -> Result<KeptWatchWhole, WriteError> {
        let doing = "reading what a watch kept";
        let mut asking = self
            .conn
            .prepare(
                "SELECT at, kind, state, finished, says, link, parts, why, output, log FROM watch_readings \
                 WHERE watch_id = ?1 ORDER BY n",
            )
            .map_err(fault(doing))?;
        let readings = asking
            .query_map([&watch.id], |row| {
                let taken = match row.get::<_, String>(1)?.as_str() {
                    "read" => KeptWatchTaken::Read {
                        state: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        finished: row.get::<_, Option<bool>>(3)?.unwrap_or(false),
                        says: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                        link: row.get(5)?,
                        parts: row
                            .get::<_, Option<String>>(6)?
                            .and_then(|parts| serde_json::from_str(&parts).ok())
                            .unwrap_or_default(),
                    },
                    _ => KeptWatchTaken::Broken {
                        why: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                        output: row.get(8)?,
                    },
                };
                Ok(KeptWatchReading { at: row.get(0)?, taken, log: row.get(9)? })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))?;
        let mut asking = self
            .conn
            .prepare("SELECT part, sentence, at FROM watch_set_aside WHERE watch_id = ?1 ORDER BY at, part")
            .map_err(fault(doing))?;
        let set_aside = asking
            .query_map([&watch.id], |row| {
                Ok(KeptWatchSetAside { part: row.get(0)?, sentence: row.get(1)?, at: row.get(2)? })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))?;
        let mut asking = self
            .conn
            .prepare("SELECT at, verdict, sentence, parts FROM watch_verdicts WHERE watch_id = ?1 ORDER BY n")
            .map_err(fault(doing))?;
        let verdicts = asking
            .query_map([&watch.id], |row| {
                Ok(KeptWatchVerdict {
                    at: row.get(0)?,
                    ours: row.get::<_, String>(1)? == "ours",
                    sentence: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    parts: texts(row.get(3)?),
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))?;
        Ok(KeptWatchWhole { watch, readings, set_aside, verdicts })
    }
}
