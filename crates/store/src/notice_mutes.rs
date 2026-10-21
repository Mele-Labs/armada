//! Fleet notices a person turned off, kept across a restart.
//!
//! A mute is for one job (`scope` `job`, key a session id) or a repository (`repo`, key a manifest id),
//! and ends at `until` when it has one. Timestamps are the fixed-width RFC3339 the store keeps
//! elsewhere, so comparing them as text is comparing time.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The mutes of one notice kind that have not ended, read once so a pass over many slots asks the
/// store once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LiveMutes {
    jobs: Vec<String>,
    repos: Vec<String>,
}

impl LiveMutes {
    /// Whether the notice is off for this session or for the repository it works in.
    pub fn covers(&self, session_id: &str, manifest_id: &str) -> bool {
        self.jobs.iter().any(|key| key == session_id) || self.repos.iter().any(|key| key == manifest_id)
    }
}

impl Store {
    /// Keep that notice `kind` is off for `key` under `scope`, until `until` when given, in place of any earlier mute.
    pub fn keep_mute(&mut self, kind: &str, scope: &str, key: &str, until: Option<&str>) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO notice_mutes (kind, scope, key, until) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (kind, scope, key) DO UPDATE SET until = ?4",
                params![kind, scope, key, until],
            )
            .map(|_| ())
            .map_err(fault("keeping a notice mute"))
            .map_err(WriteError::Database)
    }

    /// Lift a mute. One that was not kept is not an error.
    pub fn lift_mute(&mut self, kind: &str, scope: &str, key: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM notice_mutes WHERE kind = ?1 AND scope = ?2 AND key = ?3",
                params![kind, scope, key],
            )
            .map(|_| ())
            .map_err(fault("lifting a notice mute"))
            .map_err(WriteError::Database)
    }

    /// The mutes of `kind` still in force at `now`.
    pub fn live_mutes(&self, kind: &str, now: &str) -> Result<LiveMutes, WriteError> {
        let read = |conn: &rusqlite::Connection| -> rusqlite::Result<LiveMutes> {
            let mut statement = conn.prepare(
                "SELECT scope, key FROM notice_mutes WHERE kind = ?1 AND (until IS NULL OR until > ?2)",
            )?;
            let rows = statement.query_map(params![kind, now], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
            let mut mutes = LiveMutes::default();
            for row in rows {
                let (scope, key) = row?;
                match scope.as_str() {
                    "job" => mutes.jobs.push(key),
                    "repo" => mutes.repos.push(key),
                    _ => {}
                }
            }
            Ok(mutes)
        };
        read(&self.conn)
            .map_err(fault("reading the notice mutes in force"))
            .map_err(WriteError::Database)
    }

    /// Whether notice `kind` is off at `now` for this session or its repository.
    pub fn is_muted(&self, kind: &str, session_id: &str, manifest_id: &str, now: &str) -> Result<bool, WriteError> {
        Ok(self.live_mutes(kind, now)?.covers(session_id, manifest_id))
    }
}
