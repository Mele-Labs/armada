//! How long a Job waited for a slot: `line`, while every slot was leased, and `move`, from the lease
//! to the Job being put in the slot. Two stamps a wait, kept for good where the timeline prunes.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One wait. `ended_at` is empty while it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSlotWait {
    /// `line` or `move`.
    pub phase: String,
    pub began_at: String,
    pub ended_at: Option<String>,
}

impl Store {
    /// A wait of `phase` began. One standing wait of a phase at a time: a second begin is no change.
    pub fn begin_slot_wait(&mut self, session_id: &str, phase: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_slot_waits (session_id, phase, began_at)
                 SELECT ?1, ?2, ?3
                 WHERE NOT EXISTS (SELECT 1 FROM session_slot_waits
                                   WHERE session_id = ?1 AND phase = ?2 AND ended_at IS NULL)",
                params![session_id, phase, at],
            )
            .map(|_| ())
            .map_err(fault("keeping that a job began waiting for a slot"))
            .map_err(WriteError::Database)
    }

    /// The standing wait of `phase` ended; every standing one where `phase` is `None`.
    pub fn end_slot_waits(&mut self, session_id: &str, phase: Option<&str>, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE session_slot_waits SET ended_at = ?3
                 WHERE session_id = ?1 AND ended_at IS NULL AND (?2 IS NULL OR phase = ?2)",
                params![session_id, phase, at],
            )
            .map(|_| ())
            .map_err(fault("keeping that a job stopped waiting for a slot"))
            .map_err(WriteError::Database)
    }

    /// Every standing wait of every Job ended at `at`, but the `move` of each Job in `owed`: Fleet came
    /// up, and the line and the moves it held in memory are gone. A move owed to a process a keeper
    /// carried across is made all the same, so its wait stands.
    pub fn end_all_slot_waits(&mut self, at: &str, owed: &[String]) -> Result<usize, WriteError> {
        let marks = vec!["?"; owed.len()].join(", ");
        let ending = format!(
            "UPDATE session_slot_waits SET ended_at = ?1
             WHERE ended_at IS NULL AND NOT (phase = 'move' AND session_id IN ({marks}))"
        );
        let values = std::iter::once(at).chain(owed.iter().map(String::as_str));
        self.conn
            .execute(&ending, rusqlite::params_from_iter(values))
            .map_err(fault("ending the slot waits a restart left standing"))
            .map_err(WriteError::Database)
    }

    /// A Job's waits, oldest first.
    pub fn slot_waits_of(&self, session_id: &str) -> Result<Vec<KeptSlotWait>, WriteError> {
        let doing = "reading how long a job waited for a slot";
        let mut statement = self
            .conn
            .prepare("SELECT phase, began_at, ended_at FROM session_slot_waits WHERE session_id = ?1 ORDER BY began_at")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        statement
            .query_map([session_id], |row| {
                Ok(KeptSlotWait { phase: row.get(0)?, began_at: row.get(1)?, ended_at: row.get(2)? })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
