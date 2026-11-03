//! Which comments on a pull request Fleet told a Job of, and at what edit.
//! `docs/concepts/fleet.md`, *Telling the owner of a pull request*.
//!
//! **One row a (pull request, recipient, comment)**, holding the comment's `edited_at` when told:
//! a comment with no row is new, and one whose edit time moved is edited. A row with an empty
//! comment marks a pull request read with nothing said on it.

use std::collections::BTreeMap;

use core_model::Timestamp;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Each comment `recipient` was told of on `pull`, by id, with its edit time when told.
    pub fn pull_comments_told(
        &self,
        repository: &str,
        pull: u64,
        recipient: &str,
    ) -> Result<BTreeMap<String, Option<String>>, WriteError> {
        let doing = "reading the comments told of a pull request";
        let mut asking = self
            .conn
            .prepare(
                "SELECT comment, edited_at FROM pull_comments_told \
                 WHERE repository = ?1 AND pull = ?2 AND recipient = ?3",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = asking
            .query_map((repository, pull as i64, recipient), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .and_then(|rows| rows.collect::<Result<BTreeMap<_, _>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }

    /// Keep that `recipient` was told of each of `told`, a comment's id and its edit time, on
    /// `pull`. **A comment already kept takes the new edit time.**
    pub fn keep_pull_comments_told(
        &mut self,
        repository: &str,
        pull: u64,
        recipient: &str,
        told: &[(&str, Option<&str>)],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let doing = "keeping the comments told of a pull request";
        let tx = self
            .conn
            .transaction()
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        for (comment, edited_at) in told {
            tx.execute(
                "INSERT INTO pull_comments_told \
                 (repository, pull, recipient, comment, edited_at, told_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT (repository, pull, recipient, comment) \
                 DO UPDATE SET edited_at = excluded.edited_at, told_at = excluded.told_at",
                (repository, pull as i64, recipient, *comment, *edited_at, at.as_str()),
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
