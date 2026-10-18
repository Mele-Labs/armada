//! The turns a token rule is read from, and the rows that record a Job being told.
//! `docs/concepts/session.md`, *Where the tokens went*.
//!
//! **Bounded by the session's index.** A reading takes the last hour of one session's turns and
//! its last few main-thread ones, each a range or a backwards walk of `(session_id, at)`, so the
//! cost of reading is the Job's own recent turns and never the table.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::session_turn_usage::{kept_turn, KeptTurnUsage, COLUMNS};
use crate::usage_summary::{text, DEFAULT_DAYS, STAMP};

/// A Job told that it crossed a rule.
#[derive(Clone, Debug, PartialEq)]
pub struct KeptTrip {
    pub session_id: String,
    pub rule: String,
    pub at: String,
    /// What crossed: tokens, a percent or a count, by rule.
    pub figure: f64,
    /// Burn in cost units over the 60 minutes before `at`.
    pub burn_per_hour_before: f64,
    /// Burn per hour of active time over the 60 minutes after `at`. Empty until those minutes are
    /// covered, and after, where the Job made fewer than 5 turns in them.
    pub burn_per_hour_after: Option<f64>,
    /// How many turns the Job made in the 60 minutes after. Empty until measured.
    pub after_turns: Option<u32>,
}

const TRIP_COLUMNS: &str = "session_id, rule, at, figure, burn_per_hour_before, burn_per_hour_after, after_turns";

fn kept_trip(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptTrip> {
    Ok(KeptTrip {
        session_id: row.get(0)?,
        rule: row.get(1)?,
        at: row.get(2)?,
        figure: row.get(3)?,
        burn_per_hour_before: row.get(4)?,
        burn_per_hour_after: row.get(5)?,
        after_turns: row.get::<_, Option<i64>>(6)?.map(|turns| turns as u32),
    })
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a job's turns for its token rules")(why))
}

impl Store {
    /// A session's turns from the 60 minutes before its newest turn, and its newest `main_turns`
    /// main-thread turns where those reach further back, oldest first.
    pub fn turns_for_rules(&self, session_id: &str, main_turns: usize) -> Result<Vec<KeptTurnUsage>, WriteError> {
        let mut hour = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM session_turn_usage
                 WHERE session_id = ?1
                   AND at >= strftime({STAMP}, (SELECT MAX(at) FROM session_turn_usage WHERE session_id = ?1), '-60 minutes')"
            ))
            .map_err(reading)?;
        let mut turns: Vec<KeptTurnUsage> =
            hour.query_map(params![session_id], kept_turn).map_err(reading)?.collect::<Result<_, _>>().map_err(reading)?;
        let mut last = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM session_turn_usage WHERE session_id = ?1 AND subagent = 0
                 ORDER BY at DESC, message_id DESC LIMIT ?2"
            ))
            .map_err(reading)?;
        let more: Vec<KeptTurnUsage> = last
            .query_map(params![session_id, main_turns as i64], kept_turn)
            .map_err(reading)?
            .collect::<Result<_, _>>()
            .map_err(reading)?;
        for one in more {
            if !turns.iter().any(|held| held.message_id == one.message_id) {
                turns.push(one);
            }
        }
        turns.sort_by(|a, b| (&a.at, &a.message_id).cmp(&(&b.at, &b.message_id)));
        Ok(turns)
    }

    /// Keep that a Job was told. A second row for the same Job, rule and instant is the first.
    pub fn keep_usage_rule_trip(&mut self, trip: &KeptTrip) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO usage_rule_trips (session_id, rule, at, figure, burn_per_hour_before)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![trip.session_id, trip.rule, trip.at, trip.figure, trip.burn_per_hour_before],
            )
            .map(|_| ())
            .map_err(fault("keeping a token rule trip"))
            .map_err(WriteError::Database)
    }

    /// When the Job was last told of `rule`, if it ever was.
    pub fn last_usage_rule_trip(&self, session_id: &str, rule: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row(
                "SELECT at FROM usage_rule_trips WHERE session_id = ?1 AND rule = ?2 ORDER BY at DESC LIMIT 1",
                params![session_id, rule],
                |row| row.get(0),
            )
            .optional()
            .map_err(reading)
    }

    /// Every trip of a Job, oldest first.
    pub fn usage_rule_trips(&self, session_id: &str) -> Result<Vec<KeptTrip>, WriteError> {
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {TRIP_COLUMNS} FROM usage_rule_trips WHERE session_id = ?1 ORDER BY at, rule"))
            .map_err(reading)?;
        let rows = statement.query_map(params![session_id], kept_trip).map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }

    /// The trips in `[since, until)`, newest first, by the time index. `until` is `now` and `since` a
    /// week before it where left out. `None` where a bound is not a time. The bounds come back as
    /// resolved.
    pub fn usage_rule_trips_between(
        &self,
        since: Option<&str>,
        until: Option<&str>,
        now: &str,
    ) -> Result<Option<(String, String, Vec<KeptTrip>)>, WriteError> {
        let Some(until) = text(&self.conn, &format!("SELECT strftime({STAMP}, ?1)"), &[&until.unwrap_or(now)])? else {
            return Ok(None);
        };
        let since = match since {
            Some(since) => text(&self.conn, &format!("SELECT strftime({STAMP}, ?1)"), &[&since])?,
            None => text(&self.conn, &format!("SELECT strftime({STAMP}, ?1, '-{DEFAULT_DAYS} days')"), &[&until])?,
        };
        let Some(since) = since else { return Ok(None) };
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {TRIP_COLUMNS} FROM usage_rule_trips WHERE at >= ?1 AND at < ?2 ORDER BY at DESC, session_id, rule"
            ))
            .map_err(reading)?;
        let rows = statement.query_map(params![since, until], kept_trip).map_err(reading)?;
        let trips = rows.collect::<Result<_, _>>().map_err(reading)?;
        Ok(Some((since, until, trips)))
    }

    /// The Job's trips not yet measured, oldest first.
    pub fn unmeasured_usage_rule_trips(&self, session_id: &str) -> Result<Vec<KeptTrip>, WriteError> {
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {TRIP_COLUMNS} FROM usage_rule_trips WHERE session_id = ?1 AND after_turns IS NULL ORDER BY at, rule"
            ))
            .map_err(reading)?;
        let rows = statement.query_map(params![session_id], kept_trip).map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }

    /// The Job's turns after `at` and up to 60 minutes on from it, oldest first.
    pub fn turns_in_the_hour_after(&self, session_id: &str, at: &str) -> Result<Vec<KeptTurnUsage>, WriteError> {
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM session_turn_usage
                 WHERE session_id = ?1 AND at > ?2 AND at <= strftime({STAMP}, ?2, '+60 minutes')
                 ORDER BY at, message_id"
            ))
            .map_err(reading)?;
        let rows = statement.query_map(params![session_id, at], kept_turn).map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }

    /// Note what the hour after a trip came to.
    pub fn keep_usage_rule_trip_measured(
        &mut self,
        session_id: &str,
        rule: &str,
        at: &str,
        burn_per_hour_after: Option<f64>,
        after_turns: u32,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE usage_rule_trips SET burn_per_hour_after = ?4, after_turns = ?5
                 WHERE session_id = ?1 AND rule = ?2 AND at = ?3",
                params![session_id, rule, at, burn_per_hour_after, i64::from(after_turns)],
            )
            .map(|_| ())
            .map_err(fault("keeping what followed a token rule trip"))
            .map_err(WriteError::Database)
    }
}
