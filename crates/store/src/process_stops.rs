//! When a Job's agent stopped on its own and when Fleet was away, for the split of a Job's time.
//! `docs/concepts/session.md`.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// A Job's agent process ended on its own, as opposed to Fleet letting it go.
    pub fn keep_session_stop(&mut self, session_id: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT OR IGNORE INTO session_stops (session_id, at) VALUES (?1, ?2)", params![session_id, at])
            .map(|_| ())
            .map_err(failed("keeping that a job's process stopped"))
    }

    /// When a Job's process stopped on its own, oldest first.
    pub fn session_stops_of(&self, session_id: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading when a job's process stopped";
        let mut statement = self
            .conn
            .prepare("SELECT at FROM session_stops WHERE session_id = ?1 ORDER BY at")
            .map_err(failed(doing))?;
        statement
            .query_map([session_id], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<String>, _>>())
            .map_err(failed(doing))
    }

    /// Fleet is up at `at`.
    pub fn keep_fleet_alive(&mut self, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO fleet_alive (only_row, at) VALUES (1, ?1)
                 ON CONFLICT (only_row) DO UPDATE SET at = excluded.at",
                [at],
            )
            .map(|_| ())
            .map_err(failed("keeping when Fleet was last up"))
    }

    /// Fleet starts at `now`. If it was last seen up more than `most_seconds` before, that stretch
    /// is kept as an outage, and it is seen up now. The outage, if one was kept.
    pub fn fleet_started(&mut self, now: &str, most_seconds: i64) -> Result<Option<(String, String)>, WriteError> {
        let doing = "keeping that Fleet was away";
        let seen: Option<String> = self
            .conn
            .query_row("SELECT at FROM fleet_alive WHERE only_row = 1", [], |row| row.get(0))
            .optional()
            .map_err(failed(doing))?;
        let away = match seen {
            Some(seen) => {
                let long: bool = self
                    .conn
                    .query_row("SELECT (julianday(?1) - julianday(?2)) * 86400 > ?3", params![now, seen, most_seconds], |row| {
                        row.get(0)
                    })
                    .map_err(failed(doing))?;
                long.then_some((seen, now.to_string()))
            }
            None => None,
        };
        if let Some((began, ended)) = &away {
            self.conn
                .execute("INSERT OR IGNORE INTO fleet_outages (began_at, ended_at) VALUES (?1, ?2)", params![began, ended])
                .map_err(failed(doing))?;
        }
        self.keep_fleet_alive(now)?;
        Ok(away)
    }

    /// The outages that overlap `from`..`until`, oldest first.
    pub fn fleet_outages_between(&self, from: &str, until: &str) -> Result<Vec<(String, String)>, WriteError> {
        let doing = "reading when Fleet was away";
        let mut statement = self
            .conn
            .prepare("SELECT began_at, ended_at FROM fleet_outages WHERE ended_at > ?1 AND began_at < ?2 ORDER BY began_at")
            .map_err(failed(doing))?;
        statement
            .query_map(params![from, until], |row| Ok((row.get(0)?, row.get(1)?)))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failed(doing))
    }
}
