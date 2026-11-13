//! What a Session's rail has done, kept. `docs/concepts/session.md`, *The rail*.
//!
//! **Strings and numbers, not the wire's types**: this crate does not know `ipc`. A step is kept by its id, so a
//! workflow that gains, loses or reorders steps does not move the rail; the caller passes the ids the workflow has now
//! and reads and writes indexes into them. A row kept before that (`by_id = 0`) holds indexes and reads as they are.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What the rail has done for a Session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptRail {
    /// The step the Session last stood on, as the timeline was told.
    pub step: Option<u32>,
    /// Steps a boundary judge read, back when one did. Kept as it was; nothing adds to it.
    pub judged: Vec<u32>,
    /// Steps Fleet told the agent it had stopped on, where only its own act ends them.
    pub stalled: Vec<u32>,
    /// Each step the Session began, in order, with the time. A step it went back to appears again.
    pub visits: Vec<(u32, String)>,
    /// Steps the person skipped, ascending.
    pub skipped: Vec<u32>,
}

/// How a Session was started, as it stood when its process first ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptStarted {
    /// The workflow's name as it read then. `None` is a custom Session.
    pub workflow: Option<String>,
    /// `auto_land`, `open_pr` or `keep_local`, where one was chosen by then.
    pub delivery: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// `ask`, `auto`, `accept_edits` or `plan`.
    pub mode: String,
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a Job's rail")(why))
}

fn keeping(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("keeping a Job's rail")(why))
}

fn unlisted(text: &str) -> Vec<u32> {
    text.split(',').filter_map(|one| one.trim().parse().ok()).collect()
}

fn visits_of(text: &str) -> Vec<(u32, String)> {
    text.lines().filter_map(|line| line.split_once('\t').and_then(|(step, since)| Some((step.parse().ok()?, since.to_string())))).collect()
}

/// The ids of the steps at these indexes. An index past the end names no step and is not kept.
fn ids_of(indexes: &[u32], steps: &[String]) -> Vec<String> {
    indexes.iter().filter_map(|at| steps.get(*at as usize).cloned()).collect()
}

fn index_of(id: &str, steps: &[String]) -> Option<u32> {
    steps.iter().position(|one| one == id).map(|at| at as u32)
}

/// A kept list of ids as indexes into `steps`; an id it no longer has is dropped. Ids are free text, so the list is JSON.
fn indexes_of(text: &str, steps: &[String]) -> Vec<u32> {
    let ids: Vec<String> = if text.is_empty() { Vec::new() } else { serde_json::from_str(text).unwrap_or_default() };
    ids.iter().filter_map(|id| index_of(id, steps)).collect()
}

fn visit_indexes_of(text: &str, steps: &[String]) -> Vec<(u32, String)> {
    let visits: Vec<(String, String)> = if text.is_empty() { Vec::new() } else { serde_json::from_str(text).unwrap_or_default() };
    visits.into_iter().filter_map(|(id, since)| Some((index_of(&id, steps)?, since))).collect()
}

impl Store {
    /// What the rail has done for the Session, as indexes into `steps`, the ids of the steps its workflow has now. A step
    /// the workflow no longer has is left out. A Session it has done nothing for reads as default. A row kept before
    /// ids were kept holds indexes already and is read as they are, whatever `steps` says.
    pub fn session_rail(&self, session_id: &str, steps: &[String]) -> Result<KeptRail, WriteError> {
        let found = self
            .conn
            .query_row(
                "SELECT step, judged, visits, stalled, skipped, by_id, step_id FROM session_rail WHERE session_id = ?1",
                [session_id],
                |row| {
                    let (judged, visits, stalled, skipped): (String, String, String, String) =
                        (row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?);
                    let (by_id, step_id): (bool, Option<String>) = (row.get(5)?, row.get(6)?);
                    Ok(if by_id {
                        KeptRail {
                            step: step_id.and_then(|id| index_of(&id, steps)),
                            judged: indexes_of(&judged, steps),
                            stalled: indexes_of(&stalled, steps),
                            visits: visit_indexes_of(&visits, steps),
                            skipped: indexes_of(&skipped, steps),
                        }
                    } else {
                        KeptRail {
                            step: row.get(0)?,
                            judged: unlisted(&judged),
                            stalled: unlisted(&stalled),
                            visits: visits_of(&visits),
                            skipped: unlisted(&skipped),
                        }
                    })
                },
            )
            .optional()
            .map_err(reading)?;
        Ok(found.unwrap_or_default())
    }

    /// Keep what the rail has done, written as the ids of `steps`, which are the ones `rail`'s indexes are into.
    pub fn keep_session_rail(&mut self, session_id: &str, rail: &KeptRail, steps: &[String]) -> Result<(), WriteError> {
        let step_id = rail.step.and_then(|at| steps.get(at as usize));
        let visits: Vec<(&String, &String)> = rail.visits.iter().filter_map(|(at, since)| Some((steps.get(*at as usize)?, since))).collect();
        self.conn
            .execute(
                "INSERT INTO session_rail (session_id, step, step_id, judged, visits, stalled, skipped, by_id) VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, 1)
                 ON CONFLICT (session_id) DO UPDATE SET step = NULL, step_id = ?2, judged = ?3, visits = ?4, stalled = ?5, skipped = ?6, by_id = 1",
                params![
                    session_id,
                    step_id,
                    serde_json::json!(ids_of(&rail.judged, steps)).to_string(),
                    serde_json::json!(visits).to_string(),
                    serde_json::json!(ids_of(&rail.stalled, steps)).to_string(),
                    serde_json::json!(ids_of(&rail.skipped, steps)).to_string()
                ],
            )
            .map(|_| ())
            .map_err(keeping)
    }

    /// Forget every skip the Session kept, whichever way the row was kept.
    pub fn clear_session_skips(&mut self, session_id: &str) -> Result<(), WriteError> {
        self.conn.execute("UPDATE session_rail SET skipped = '' WHERE session_id = ?1", [session_id]).map(|_| ()).map_err(keeping)
    }

    /// The Sessions whose rail was kept as indexes, which a start-up pass rewrites by id.
    pub fn rails_kept_by_index(&self) -> Result<Vec<String>, WriteError> {
        let mut query = self.conn.prepare("SELECT session_id FROM session_rail WHERE by_id = 0").map_err(reading)?;
        let rows = query.query_map([], |row| row.get(0)).map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }

    /// How the Session was started, where it has begun.
    pub fn session_started(&self, session_id: &str) -> Result<Option<KeptStarted>, WriteError> {
        self.conn
            .query_row("SELECT workflow, delivery, model, effort, mode FROM session_started WHERE session_id = ?1", [session_id], |row| {
                Ok(KeptStarted { workflow: row.get(0)?, delivery: row.get(1)?, model: row.get(2)?, effort: row.get(3)?, mode: row.get(4)? })
            })
            .optional()
            .map_err(reading)
    }

    /// Keep how the Session was started. **The first write stands**: a Session that starts a process
    /// again keeps the start it had.
    pub fn keep_session_started(&mut self, session_id: &str, started: &KeptStarted) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO session_started (session_id, workflow, delivery, model, effort, mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![session_id, started.workflow, started.delivery, started.model, started.effort, started.mode],
            )
            .map(|_| ())
            .map_err(keeping)
    }

    /// Whether the Session has run `armada check`: a Check run it asked for exists.
    pub fn session_has_run_a_check(&self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row("SELECT EXISTS (SELECT 1 FROM session_check_runs WHERE session_id = ?1)", [session_id], |row| {
                row.get(0)
            })
            .map_err(reading)
    }
}
