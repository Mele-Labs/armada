//! What a Session's rail has done, kept. `docs/concepts/session.md`, *The rail*.
//!
//! **Strings and numbers, not the wire's types**: this crate does not know `ipc`. The step is an index
//! into the Session's workflow; the lists of steps are kept as comma-separated indexes.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What the rail has done for a Session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptRail {
    /// The step the Session last stood on, as the timeline was told.
    pub step: Option<u32>,
    /// Steps Fleet has nudged the agent about.
    pub nudged: Vec<u32>,
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

fn listed(steps: &[u32]) -> String {
    steps.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

fn unlisted(text: &str) -> Vec<u32> {
    text.split(',').filter_map(|one| one.trim().parse().ok()).collect()
}

fn visits_text(visits: &[(u32, String)]) -> String {
    visits.iter().map(|(step, since)| format!("{step}\t{since}")).collect::<Vec<_>>().join("\n")
}

fn visits_of(text: &str) -> Vec<(u32, String)> {
    text.lines().filter_map(|line| line.split_once('\t').and_then(|(step, since)| Some((step.parse().ok()?, since.to_string())))).collect()
}

impl Store {
    /// What the rail has done for the Session. A Session it has done nothing for reads as default.
    pub fn session_rail(&self, session_id: &str) -> Result<KeptRail, WriteError> {
        let found = self
            .conn
            .query_row("SELECT step, nudged, judged, visits, stalled, skipped FROM session_rail WHERE session_id = ?1", [session_id], |row| {
                let (nudged, judged, visits, stalled, skipped): (String, String, String, String, String) =
                    (row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?);
                Ok(KeptRail {
                    step: row.get(0)?,
                    nudged: unlisted(&nudged),
                    judged: unlisted(&judged),
                    stalled: unlisted(&stalled),
                    visits: visits_of(&visits),
                    skipped: unlisted(&skipped),
                })
            })
            .optional()
            .map_err(reading)?;
        Ok(found.unwrap_or_default())
    }

    pub fn keep_session_rail(&mut self, session_id: &str, rail: &KeptRail) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_rail (session_id, step, nudged, judged, visits, stalled, skipped) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (session_id) DO UPDATE SET step = ?2, nudged = ?3, judged = ?4, visits = ?5, stalled = ?6, skipped = ?7",
                params![
                    session_id,
                    rail.step,
                    listed(&rail.nudged),
                    listed(&rail.judged),
                    visits_text(&rail.visits),
                    listed(&rail.stalled),
                    listed(&rail.skipped)
                ],
            )
            .map(|_| ())
            .map_err(keeping)
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
