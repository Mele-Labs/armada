//! The Operator page's feed: what happened that needed no decision from the person. Written by `fleet::operator`.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One line of the feed, as it was kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFeed {
    pub at: String,
    /// `dispatched`, `asked`, `told` or `settled`.
    pub kind: String,
    pub title: String,
    pub session_id: Option<String>,
    pub said: Option<String>,
    /// The project of the Operator the line belongs to. Absent on a line kept before each project had one.
    pub project: Option<String>,
}

impl Store {
    /// Keep one line of the feed.
    pub fn put_operator_feed(&mut self, line: &KeptFeed) -> Result<(), WriteError> {
        let doing = "keeping a line of the Operator's feed";
        self.conn
            .execute(
                "INSERT INTO operator_feed (at, kind, title, session_id, said, project) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![line.at, line.kind, line.title, line.session_id, line.said, line.project],
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// The lines at or after `since`, newest first.
    pub fn operator_feed_since(&self, since: &str) -> Result<Vec<KeptFeed>, WriteError> {
        let doing = "reading the Operator's feed";
        let mut statement = self
            .conn
            .prepare("SELECT at, kind, title, session_id, said, project FROM operator_feed WHERE at >= ?1 ORDER BY at DESC, n DESC LIMIT 200")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(params![since], |row| Ok(KeptFeed { at: row.get(0)?, kind: row.get(1)?, title: row.get(2)?, session_id: row.get(3)?, said: row.get(4)?, project: row.get(5)? }))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }
}
