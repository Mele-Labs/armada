//! What his other Macs share with this one: their Job rows, their bug reports and their projects. **Each is Fleet's, opaque
//! here**: the text the wire carried, keyed by the Mac that holds it and its id, so a Mac that is asleep still
//! has its rows to draw after a restart.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One thing held from another Mac.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFromMac {
    pub mac: String,
    pub id: String,
    pub body: String,
}

/// Which list of another Mac's a call reads or writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FromMac {
    Sessions,
    BugReports,
    Projects,
}

impl FromMac {
    fn table(self) -> &'static str {
        match self {
            FromMac::Sessions => "other_mac_sessions",
            FromMac::BugReports => "other_mac_bug_reports",
            FromMac::Projects => "other_mac_projects",
        }
    }

    fn column(self) -> &'static str {
        match self {
            FromMac::Sessions => "record",
            FromMac::BugReports => "report",
            FromMac::Projects => "project",
        }
    }
}

impl Store {
    /// Everything of `list` held from every Mac, by Mac then id.
    pub fn held_from_macs(&self, list: FromMac) -> Result<Vec<KeptFromMac>, WriteError> {
        let doing = "reading what other Macs share";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT mac, id, {} FROM {} ORDER BY mac, id", list.column(), list.table()))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| Ok(KeptFromMac { mac: row.get(0)?, id: row.get(1)?, body: row.get(2)? }))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Everything of `list` held from Mac `mac` replaced by `rows`, `(id, body)` each, in one transaction.
    pub fn replace_from_mac(&mut self, list: FromMac, mac: &str, rows: &[(String, String)]) -> Result<(), WriteError> {
        let doing = "keeping what another Mac shares";
        let tx = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        tx.execute(&format!("DELETE FROM {} WHERE mac = ?1", list.table()), [mac])
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        for (id, body) in rows {
            tx.execute(
                &format!("INSERT OR REPLACE INTO {} (mac, id, {}) VALUES (?1, ?2, ?3)", list.table(), list.column()),
                params![mac, id, body],
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        }
        tx.commit().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// One thing of `list` held from Mac `mac` kept as `body`, over what was kept under its id.
    pub fn keep_from_mac(&mut self, list: FromMac, mac: &str, id: &str, body: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!("INSERT OR REPLACE INTO {} (mac, id, {}) VALUES (?1, ?2, ?3)", list.table(), list.column()),
                params![mac, id, body],
            )
            .map(|_| ())
            .map_err(fault("keeping one thing another Mac shares"))
            .map_err(WriteError::Database)
    }

    /// Every row, report and project held from Mac `mac` dropped, and the edits queued for it.
    pub fn forget_mac(&mut self, mac: &str) -> Result<(), WriteError> {
        let doing = "forgetting what another Mac shared";
        let tx = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        for list in [FromMac::Sessions, FromMac::BugReports, FromMac::Projects] {
            tx.execute(&format!("DELETE FROM {} WHERE mac = ?1", list.table()), [mac])
                .map_err(fault(doing))
                .map_err(WriteError::Database)?;
        }
        tx.execute("DELETE FROM other_mac_project_acts WHERE mac = ?1", [mac]).map_err(fault(doing)).map_err(WriteError::Database)?;
        tx.commit().map_err(fault(doing)).map_err(WriteError::Database)
    }
}

/// A task act made on a copy of a project of Mac `mac` while that Mac could not be reached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptProjectAct {
    /// Its place in the queue, which only grows.
    pub seq: i64,
    pub mac: String,
    /// When the person made it, in this Mac's milliseconds since the epoch.
    pub at: i64,
    /// A `ProjectAct`, opaque here.
    pub act: String,
}

impl Store {
    /// `act` queued for Mac `mac`, made at `at`, after every act already queued. Its `seq`.
    pub fn queue_project_act(&mut self, mac: &str, at: i64, act: &str) -> Result<i64, WriteError> {
        let doing = "queueing an edit for another Mac";
        self.conn
            .execute("INSERT INTO other_mac_project_acts (mac, at, act) VALUES (?1, ?2, ?3)", params![mac, at, act])
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Every act queued, for Mac `mac` or for every Mac, oldest first.
    pub fn queued_project_acts(&self, mac: Option<&str>) -> Result<Vec<KeptProjectAct>, WriteError> {
        let doing = "reading the edits queued for other Macs";
        let mut statement = self
            .conn
            .prepare("SELECT seq, mac, at, act FROM other_mac_project_acts WHERE ?1 IS NULL OR mac = ?1 ORDER BY seq")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([mac], |row| Ok(KeptProjectAct { seq: row.get(0)?, mac: row.get(1)?, at: row.get(2)?, act: row.get(3)? }))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// The act `seq` dropped from the queue: Mac answered it, or refused it.
    pub fn drop_project_act(&mut self, seq: i64) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM other_mac_project_acts WHERE seq = ?1", [seq])
            .map(|_| ())
            .map_err(fault("dropping an edit queued for another Mac"))
            .map_err(WriteError::Database)
    }
}
