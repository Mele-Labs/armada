//! What his other Macs share with this one: their Job rows and their bug reports. **Each is Fleet's, opaque
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
}

impl FromMac {
    fn table(self) -> &'static str {
        match self {
            FromMac::Sessions => "other_mac_sessions",
            FromMac::BugReports => "other_mac_bug_reports",
        }
    }

    fn column(self) -> &'static str {
        match self {
            FromMac::Sessions => "record",
            FromMac::BugReports => "report",
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

    /// Every row and report held from Mac `mac` dropped.
    pub fn forget_mac(&mut self, mac: &str) -> Result<(), WriteError> {
        let doing = "forgetting what another Mac shared";
        let tx = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        for list in [FromMac::Sessions, FromMac::BugReports] {
            tx.execute(&format!("DELETE FROM {} WHERE mac = ?1", list.table()), [mac])
                .map_err(fault(doing))
                .map_err(WriteError::Database)?;
        }
        tx.commit().map_err(fault(doing)).map_err(WriteError::Database)
    }
}
