//! Acts across his Macs, kept: what waits for a Mac that cannot be reached, in the one queue the edits of a project
//! copy use, and what this Mac ran for another, with its answer. **Opaque here**: an act is the text the wire
//! carried. `docs/practices/protocol/his-other-macs-fleets-half.md`, *Acting across Macs*.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What a retry could still arrive after is a week; an older run is forgotten when the next is kept.
const RUNS_KEPT_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// One thing queued for Mac `mac`, of either kind. The next stage's Job message to another Mac is one more
/// `kind` of send behind `/api/mac/act`, kept in this queue and drained with the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptQueuedAct {
    /// Its place in the queue, which only grows.
    pub seq: i64,
    pub mac: String,
    /// `project`, a task edit of a copy, or `job`, an act on a Job.
    pub kind: String,
    /// The sender's id for a job act, empty for a project edit.
    pub id: String,
    /// When the person made it, in this Mac's milliseconds since the epoch.
    pub at: i64,
    /// A `ProjectAct` or a `JobAct`, by `kind`.
    pub act: String,
    /// What the Mac said when it refused the act on its turn. Such a row is kept, and is not sent again.
    pub refused: Option<String>,
}

/// An act this Mac ran for Mac `mac`, and what it answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptActRun {
    pub mac: String,
    pub id: String,
    /// When it ran, in this Mac's milliseconds.
    pub at: i64,
    /// The door it ran as, in the wire's spelling.
    pub via: String,
    /// The act's kind.
    pub what: String,
    pub status: u16,
    pub body: String,
}

impl Store {
    /// A job act queued for Mac `mac` under the sender's `id`, after every act already queued. Its `seq`.
    pub fn queue_job_act(&mut self, mac: &str, id: &str, at: i64, act: &str) -> Result<i64, WriteError> {
        let doing = "queueing an act for another Mac";
        self.conn
            .execute("INSERT INTO other_mac_project_acts (mac, kind, id, at, act) VALUES (?1, 'job', ?2, ?3, ?4)", params![mac, id, at, act])
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Every act queued of either kind, for Mac `mac` or for every Mac, oldest first. `unsent` leaves out those the Mac refused.
    pub fn queued_acts(&self, mac: Option<&str>, unsent: bool) -> Result<Vec<KeptQueuedAct>, WriteError> {
        let doing = "reading the acts queued for other Macs";
        let mut statement = self
            .conn
            .prepare(
                "SELECT seq, mac, kind, id, at, act, refused FROM other_mac_project_acts \
                 WHERE (?1 IS NULL OR mac = ?1) AND (?2 = 0 OR refused IS NULL) ORDER BY seq",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(params![mac, unsent], |row| {
                Ok(KeptQueuedAct {
                    seq: row.get(0)?,
                    mac: row.get(1)?,
                    kind: row.get(2)?,
                    id: row.get(3)?,
                    at: row.get(4)?,
                    act: row.get(5)?,
                    refused: row.get(6)?,
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// The job act queued under `id`.
    pub fn queued_job_act(&self, id: &str) -> Result<Option<KeptQueuedAct>, WriteError> {
        Ok(self.queued_acts(None, false)?.into_iter().find(|one| one.kind == "job" && one.id == id))
    }

    /// The act `seq` kept, and not sent again, for what the Mac said when it refused it.
    pub fn refuse_queued_act(&mut self, seq: i64, why: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE other_mac_project_acts SET refused = ?2 WHERE seq = ?1", params![seq, why])
            .map(|_| ())
            .map_err(fault("keeping an act another Mac refused"))
            .map_err(WriteError::Database)
    }

    /// What this Mac answered Mac `mac`'s act `id`, where it ran it.
    pub fn act_run(&self, mac: &str, id: &str) -> Result<Option<KeptActRun>, WriteError> {
        let doing = "reading an act another Mac sent";
        self.conn
            .query_row("SELECT mac, id, at, via, what, status, body FROM other_mac_acts_run WHERE mac = ?1 AND id = ?2", params![mac, id], run)
            .optional()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// `ran` kept under its Mac and id, and every run older than a week forgotten.
    pub fn keep_act_run(&mut self, ran: &KeptActRun) -> Result<(), WriteError> {
        let doing = "keeping an act another Mac sent";
        let tx = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        tx.execute("DELETE FROM other_mac_acts_run WHERE at < ?1", [ran.at - RUNS_KEPT_MS]).map_err(fault(doing)).map_err(WriteError::Database)?;
        tx.execute(
            "INSERT OR REPLACE INTO other_mac_acts_run (mac, id, at, via, what, status, body) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![ran.mac, ran.id, ran.at, ran.via, ran.what, ran.status, ran.body],
        )
        .map_err(fault(doing))
        .map_err(WriteError::Database)?;
        tx.commit().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Every act this Mac ran for another, oldest first.
    pub fn acts_run(&self) -> Result<Vec<KeptActRun>, WriteError> {
        let doing = "reading the acts other Macs sent";
        let mut statement = self
            .conn
            .prepare("SELECT mac, id, at, via, what, status, body FROM other_mac_acts_run ORDER BY at, mac, id")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement.query_map([], run).map_err(fault(doing)).map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }
}

fn run(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptActRun> {
    Ok(KeptActRun {
        mac: row.get(0)?,
        id: row.get(1)?,
        at: row.get(2)?,
        via: row.get(3)?,
        what: row.get(4)?,
        status: row.get(5)?,
        body: row.get(6)?,
    })
}
