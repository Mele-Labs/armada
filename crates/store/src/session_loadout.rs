//! A session's Loadout, kept: the latest list of what its agent knows and was told, and what it used,
//! a bounded window of turns. `docs/practices/protocol.md`, *A Session's Loadout*.
//!
//! **Strings, not the wire's types**: this crate does not know `ipc`. Both are the wire's own JSON.
//! A turn is named by the instant it began, as the mod reported it, and turns are ordered by when
//! Fleet first heard of each.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a job's Loadout")(why))
}

fn keeping(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("keeping a job's Loadout")(why))
}

impl Store {
    /// The latest list a session reported.
    pub fn session_loadout(&self, session_id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT items FROM session_loadout WHERE session_id = ?1", [session_id], |row| row.get(0))
            .optional()
            .map_err(reading)
    }

    /// Keep the latest list. Whether it differs from the one held.
    pub fn keep_session_loadout(&mut self, session_id: &str, items: &str) -> Result<bool, WriteError> {
        if self.session_loadout(session_id)?.as_deref() == Some(items) {
            return Ok(false);
        }
        self.conn
            .execute(
                "INSERT OR REPLACE INTO session_loadout (session_id, items) VALUES (?1, ?2)",
                params![session_id, items],
            )
            .map(|_| true)
            .map_err(keeping)
    }

    /// Keep one use, and let go of every turn but the latest `turns`. **The same use again in the same
    /// turn is not kept twice: its count goes up.** Whether it was new.
    pub fn keep_session_use(&mut self, session_id: &str, turn_started_at: &str, used: &str, turns: usize) -> Result<bool, WriteError> {
        let kept = self.conn.transaction().map_err(keeping)?;
        let new = kept
            .execute(
                "INSERT OR IGNORE INTO session_loadout_uses (session_id, turn_started_at, used) VALUES (?1, ?2, ?3)",
                params![session_id, turn_started_at, used],
            )
            .map_err(keeping)?
            > 0;
        if !new {
            kept.execute(
                "UPDATE session_loadout_uses SET count = count + 1
                 WHERE session_id = ?1 AND turn_started_at = ?2 AND used = ?3",
                params![session_id, turn_started_at, used],
            )
            .map_err(keeping)?;
        }
        if new {
            kept.execute(
                "DELETE FROM session_loadout_uses WHERE session_id = ?1 AND turn_started_at NOT IN (
                     SELECT turn_started_at FROM session_loadout_uses WHERE session_id = ?1
                     GROUP BY turn_started_at ORDER BY MIN(seq) DESC LIMIT ?2)",
                params![session_id, turns as i64],
            )
            .map_err(keeping)?;
        }
        kept.commit().map_err(keeping)?;
        Ok(new)
    }

    /// Every use kept, as `(turn_started_at, use, times heard)`, turn by turn in the order each was
    /// first heard of, and in the order they were heard within one.
    pub fn session_uses(&self, session_id: &str) -> Result<Vec<(String, String, u32)>, WriteError> {
        let mut read = self
            .conn
            .prepare(
                "SELECT uses.turn_started_at, uses.used, uses.count FROM session_loadout_uses AS uses
                 JOIN (SELECT turn_started_at, MIN(seq) AS first FROM session_loadout_uses
                       WHERE session_id = ?1 GROUP BY turn_started_at) AS turns
                   ON turns.turn_started_at = uses.turn_started_at
                 WHERE uses.session_id = ?1
                 ORDER BY turns.first, uses.seq",
            )
            .map_err(reading)?;
        let uses = read
            .query_map([session_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(reading)?
            .collect::<Result<_, _>>()
            .map_err(reading);
        uses
    }

    /// The latest turn's uses: the turn first heard of last, and its uses in order, each with the
    /// times it was heard.
    pub fn session_latest_uses(&self, session_id: &str) -> Result<Option<(String, Vec<(String, u32)>)>, WriteError> {
        let latest: Option<String> = self
            .conn
            .query_row(
                "SELECT turn_started_at FROM session_loadout_uses WHERE session_id = ?1
                 GROUP BY turn_started_at ORDER BY MIN(seq) DESC LIMIT 1",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(reading)?;
        let Some(turn) = latest else {
            return Ok(None);
        };
        let mut read = self
            .conn
            .prepare("SELECT used, count FROM session_loadout_uses WHERE session_id = ?1 AND turn_started_at = ?2 ORDER BY seq")
            .map_err(reading)?;
        let uses = read
            .query_map(params![session_id, turn], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(reading)?
            .collect::<Result<Vec<(String, u32)>, _>>()
            .map_err(reading)?;
        Ok(Some((turn, uses)))
    }
}
