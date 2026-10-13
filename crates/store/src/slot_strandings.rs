//! What Fleet has done about a stranded slot on its own: told its last holder, and started a rescue
//! Scout. `docs/concepts/fleet.md`, *Rescuing a stranded slot*.
//!
//! **One row a slot, for one stranding.** A stranding is the lease that ended without the slot
//! being given back, named by its branch and `since`; a row for another is of an earlier one and is
//! replaced. The row is what makes each act once-only across a Fleet restart.

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One slot's stranding, and what Fleet did about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptStranding {
    pub manifest_id: String,
    pub slot: u32,
    /// The branch the slot was on. Empty where it was on none.
    pub branch: String,
    /// When the lease that stranded it began, as the wire spells an instant. Empty where the
    /// record named none.
    pub since: String,
    /// When its last holder was told. Absent until it was.
    pub told_at: Option<String>,
    /// A rescue Scout was started without a press.
    pub scouted: bool,
}

impl KeptStranding {
    /// Whether this row is of the stranding on `branch` that began at `since`.
    pub fn is_of(&self, branch: &str, since: &str) -> bool {
        self.branch == branch && self.since == since
    }
}

impl Store {
    /// Keep `stranding`, replacing the slot's last.
    pub fn keep_stranding(&mut self, stranding: &KeptStranding) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO slot_strandings \
                 (manifest_id, slot, branch, since, told_at, scouted) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    stranding.manifest_id,
                    i64::from(stranding.slot),
                    stranding.branch,
                    stranding.since,
                    stranding.told_at,
                    stranding.scouted,
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping what was done about a stranded slot"))
            .map_err(WriteError::Database)
    }

    /// Every stranding kept, across every repository.
    pub fn strandings(&self) -> Result<Vec<KeptStranding>, WriteError> {
        let doing = "reading what was done about stranded slots";
        let mut statement = self
            .conn
            .prepare(
                "SELECT manifest_id, slot, branch, since, told_at, scouted FROM slot_strandings \
                 ORDER BY manifest_id, slot",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| {
                Ok(KeptStranding {
                    manifest_id: row.get(0)?,
                    slot: row.get::<_, i64>(1)? as u32,
                    branch: row.get(2)?,
                    since: row.get(3)?,
                    told_at: row.get(4)?,
                    scouted: row.get(5)?,
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// The slot's stranding gone, for a slot that is no longer stranded. Nothing, where it had none.
    pub fn forget_stranding(&mut self, manifest_id: &str, slot: u32) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM slot_strandings WHERE manifest_id = ?1 AND slot = ?2",
                rusqlite::params![manifest_id, i64::from(slot)],
            )
            .map(|_| ())
            .map_err(fault("forgetting what was done about a stranded slot"))
            .map_err(WriteError::Database)
    }
}
