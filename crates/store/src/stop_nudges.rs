//! The stop each Job was told about and the stop passed over. `docs/concepts/session/what-a-job-is-doing.md`.

use rusqlite::OptionalExtension;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What is kept of a Job's stops. A stop is named by when its turn ended.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptStop {
    /// The stop the nudge was about, and when it went. Clear once anything else woke the Job.
    pub told: Option<(String, String)>,
    /// The nudge reached nobody: it was muted, withheld or refused.
    pub unheard: bool,
    /// The stop never read as one: the person's Stop press, or one standing when this was first kept.
    pub passed_over: Option<String>,
    /// The Job started to bring it back after its restart failed.
    pub repair_job: Option<String>,
    /// An unhealthy Fleet started no repair Job.
    pub repair_held: bool,
    /// On a repair Job: the Job it repairs. It never gets one of its own.
    pub repairs: Option<String>,
}

impl Store {
    /// Every live session, hosted or in a terminal.
    pub fn live_session_ids(&self) -> Result<Vec<String>, WriteError> {
        let doing = "reading the live sessions";
        let mut statement =
            self.conn.prepare("SELECT id FROM sessions WHERE state = 'live'").map_err(fault(doing)).map_err(WriteError::Database)?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(fault(doing)).map_err(WriteError::Database)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    pub fn session_stop(&self, session: &str) -> Result<KeptStop, WriteError> {
        self.conn
            .query_row(
                "SELECT told_stop, told_at, unheard, passed_over, repair_job, repair_held, repairs FROM stop_nudges WHERE session_id = ?1",
                [session],
                |row| {
                    let told: (Option<String>, Option<String>) = (row.get(0)?, row.get(1)?);
                    Ok(KeptStop {
                        told: told.0.zip(told.1),
                        unheard: row.get(2)?,
                        passed_over: row.get(3)?,
                        repair_job: row.get(4)?,
                        repair_held: row.get(5)?,
                        repairs: row.get(6)?,
                    })
                },
            )
            .optional()
            .map(Option::unwrap_or_default)
            .map_err(fault("reading a Job's stop"))
            .map_err(WriteError::Database)
    }

    /// Keep that `session` was told about the stop `stop` at `at`.
    pub fn keep_stop_told(&mut self, session: &str, stop: &str, at: &str, unheard: bool) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO stop_nudges (session_id, told_stop, told_at, unheard) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (session_id) DO UPDATE SET told_stop = ?2, told_at = ?3, unheard = ?4",
                (session, stop, at, unheard),
            )
            .map(|_| ())
            .map_err(fault("keeping that a Job was told of its stop"))
            .map_err(WriteError::Database)
    }

    /// Something else woke `session`, so the stop after it gets a nudge of its own.
    pub fn clear_stop_told(&mut self, session: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE stop_nudges SET told_stop = NULL, told_at = NULL, unheard = 0 WHERE session_id = ?1 AND told_stop IS NOT NULL",
                [session],
            )
            .map(|_| ())
            .map_err(fault("clearing that a Job was told of its stop"))
            .map_err(WriteError::Database)
    }

    /// Keep that `repair` was started to bring `session` back.
    pub fn keep_repair_started(&mut self, session: &str, repair: &str) -> Result<(), WriteError> {
        let doing = "keeping a repair Job";
        self.conn
            .execute(
                "INSERT INTO stop_nudges (session_id, repair_job) VALUES (?1, ?2)
                 ON CONFLICT (session_id) DO UPDATE SET repair_job = ?2, repair_held = 0",
                (session, repair),
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        self.conn
            .execute(
                "INSERT INTO stop_nudges (session_id, repairs) VALUES (?1, ?2)
                 ON CONFLICT (session_id) DO UPDATE SET repairs = ?2",
                (repair, session),
            )
            .map(|_| ())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Keep whether an unhealthy Fleet held back a repair Job for `session`.
    pub fn keep_repair_held(&mut self, session: &str, held: bool) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO stop_nudges (session_id, repair_held) VALUES (?1, ?2)
                 ON CONFLICT (session_id) DO UPDATE SET repair_held = ?2",
                (session, held),
            )
            .map(|_| ())
            .map_err(fault("keeping that a repair Job was held back"))
            .map_err(WriteError::Database)
    }

    /// Its turn began again, so a later death is a new one and gets a repair of its own.
    pub fn clear_repair(&mut self, session: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE stop_nudges SET repair_job = NULL, repair_held = 0
                 WHERE session_id = ?1 AND (repair_job IS NOT NULL OR repair_held = 1)",
                [session],
            )
            .map(|_| ())
            .map_err(fault("clearing a Job's repair"))
            .map_err(WriteError::Database)
    }

    /// Whether a repair Job is live now. Fleet runs one at a time.
    pub fn repair_running(&self) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM stop_nudges n JOIN sessions s ON s.id = n.session_id
                 WHERE n.repairs IS NOT NULL AND s.state = 'live')",
                [],
                |row| row.get(0),
            )
            .map_err(fault("reading whether a repair Job is live"))
            .map_err(WriteError::Database)
    }

    /// Never read `stop` as a stop of `session`.
    pub fn pass_over_stop(&mut self, session: &str, stop: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO stop_nudges (session_id, passed_over) VALUES (?1, ?2)
                 ON CONFLICT (session_id) DO UPDATE SET passed_over = ?2",
                (session, stop),
            )
            .map(|_| ())
            .map_err(fault("passing over a Job's stop"))
            .map_err(WriteError::Database)
    }
}
