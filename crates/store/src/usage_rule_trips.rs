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
use crate::usage_summary::STAMP;

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
            .prepare(
                "SELECT session_id, rule, at, figure, burn_per_hour_before FROM usage_rule_trips
                 WHERE session_id = ?1 ORDER BY at, rule",
            )
            .map_err(reading)?;
        let rows = statement
            .query_map(params![session_id], |row| {
                Ok(KeptTrip {
                    session_id: row.get(0)?,
                    rule: row.get(1)?,
                    at: row.get(2)?,
                    figure: row.get(3)?,
                    burn_per_hour_before: row.get(4)?,
                })
            })
            .map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }
}
