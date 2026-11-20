//! Which pull requests of a repository Fleet has looked at, so a new one is offered review tools once.
//! `docs/practices/protocol/projects.md`, *Review on a new pull request*.

use core_model::Timestamp;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The row that says a repository's open pull requests were kept at the first look.
const BASELINE: i64 = 0;

impl Store {
    /// The numbers of `numbers` this repository has not kept before, and keeps them all. **The first look at a
    /// repository keeps every number and answers none**: what was open before anyone asked is not new.
    pub fn review_unseen(&mut self, manifest: &str, numbers: &[u64], at: &Timestamp) -> Result<Vec<u64>, WriteError> {
        let doing = "keeping which pull requests were looked at";
        let db = |why| WriteError::Database(fault(doing)(why));
        let tx = self.conn.transaction().map_err(db)?;
        let started: bool = tx
            .query_row("SELECT EXISTS (SELECT 1 FROM review_offered WHERE manifest_id = ?1 AND number = ?2)", (manifest, BASELINE), |row| row.get(0))
            .map_err(db)?;
        let mut fresh = Vec::new();
        for number in numbers {
            let kept = tx
                .execute(
                    "INSERT INTO review_offered (manifest_id, number, seen_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                    (manifest, i64::try_from(*number).unwrap_or(i64::MAX), at.as_str()),
                )
                .map_err(db)?;
            if kept == 1 && started {
                fresh.push(*number);
            }
        }
        tx.execute("INSERT INTO review_offered (manifest_id, number, seen_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING", (manifest, BASELINE, at.as_str()))
            .map_err(db)?;
        tx.commit().map_err(db)?;
        Ok(fresh)
    }
}
