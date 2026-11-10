//! The repairs Fleet hosted beside a broken Job: when each started, what broke, how it ended and what
//! it said. `docs/concepts/session/what-a-job-is-doing.md`.

use rusqlite::OptionalExtension;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One repair, running or over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptRepair {
    pub id: String,
    pub session_id: String,
    /// The kind of break: `quiet`, `hung`, `died`, `checkout`, `red_pull` or `stranded`.
    pub broke: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    /// `repaired`, `not_repaired`, `unmarked`, `timed_out`, `stopped`, `failed` or `abandoned`.
    pub outcome: Option<String>,
    /// The sentence its first line carried, or why it did not finish.
    pub said: Option<String>,
    /// The pool slot leased for it and not yet given back.
    pub slot: Option<u32>,
    /// Whether the branch in that slot was cut for the repair, and so goes with it.
    pub cut: bool,
}

const COLUMNS: &str = "id, session_id, broke, started_at, ended_at, outcome, said, slot, cut";

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptRepair> {
    Ok(KeptRepair {
        id: row.get(0)?,
        session_id: row.get(1)?,
        broke: row.get(2)?,
        started_at: row.get(3)?,
        ended_at: row.get(4)?,
        outcome: row.get(5)?,
        said: row.get(6)?,
        slot: row.get(7)?,
        cut: row.get::<_, i64>(8)? != 0,
    })
}

impl Store {
    /// Keep that repair `repair` started for `session` because of `broke`, and that it is the one
    /// `session`'s row names now.
    pub fn keep_repair_started(&mut self, session: &str, repair: &str, broke: &str, at: &str) -> Result<(), WriteError> {
        let doing = "keeping a repair";
        self.conn
            .execute("INSERT INTO job_repairs (id, session_id, broke, started_at) VALUES (?1, ?2, ?3, ?4)", (repair, session, broke, at))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        self.conn
            .execute(
                "INSERT INTO stop_nudges (session_id, repair_job) VALUES (?1, ?2)
                 ON CONFLICT (session_id) DO UPDATE SET repair_job = ?2, repair_held = 0",
                (session, repair),
            )
            .map(|_| ())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Keep that `repair` holds pool slot `slot`, on a branch `cut` for it or not.
    pub fn keep_repair_slot(&mut self, repair: &str, slot: u32, cut: bool) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE job_repairs SET slot = ?2, cut = ?3 WHERE id = ?1", (repair, slot, cut))
            .map(|_| ())
            .map_err(fault("keeping the slot of a repair"))
            .map_err(WriteError::Database)
    }

    /// Keep that `repair` gave its slot back.
    pub fn keep_repair_slot_given_back(&mut self, repair: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE job_repairs SET slot = NULL, cut = 0 WHERE id = ?1", [repair])
            .map(|_| ())
            .map_err(fault("keeping that a repair gave its slot back"))
            .map_err(WriteError::Database)
    }

    /// Every repair with no end kept: what Fleet left running when it stopped.
    pub fn open_repairs(&self) -> Result<Vec<KeptRepair>, WriteError> {
        let doing = "reading the repairs left open";
        let mut query = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM job_repairs WHERE ended_at IS NULL ORDER BY started_at"))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = query.query_map([], kept).map_err(fault(doing)).map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Keep how `repair` ended. Kept once: an end already kept stays.
    pub fn keep_repair_ended(&mut self, repair: &str, at: &str, outcome: &str, said: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_repairs SET ended_at = ?2, outcome = ?3, said = ?4 WHERE id = ?1 AND ended_at IS NULL",
                (repair, at, outcome, said),
            )
            .map(|_| ())
            .map_err(fault("keeping how a repair ended"))
            .map_err(WriteError::Database)
    }

    /// Every repair that has no end kept, ended as `abandoned`: Fleet is starting, and a repair does
    /// not outlive the Fleet that read it.
    pub fn end_open_repairs(&mut self, at: &str) -> Result<usize, WriteError> {
        self.conn
            .execute(
                "UPDATE job_repairs SET ended_at = ?1, outcome = 'abandoned', said = 'Fleet restarted while it ran'
                 WHERE ended_at IS NULL",
                [at],
            )
            .map_err(fault("ending the repairs Fleet left open"))
            .map_err(WriteError::Database)
    }

    pub fn repair(&self, repair: &str) -> Result<Option<KeptRepair>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM job_repairs WHERE id = ?1"), [repair], kept)
            .optional()
            .map_err(fault("reading a repair"))
            .map_err(WriteError::Database)
    }

    /// Whether `repair` has ended. One this store has no record of has, since nothing is running it.
    pub fn repair_over(&self, repair: &str) -> Result<bool, WriteError> {
        Ok(self.repair(repair)?.is_none_or(|one| one.ended_at.is_some()))
    }
}
