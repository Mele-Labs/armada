//! The presses of the actions a Job's pages declare. `docs/concepts/artifact.md`, *Page actions*.
//!
//! **Apart from the page's attachment**, which a new revision rewrites whole, so a press outlives the revision
//! that held its action.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One press.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPress {
    /// The page's artifact id: its path.
    pub artifact: String,
    pub revision: u32,
    /// The action's id.
    pub action: String,
    pub item: Option<String>,
    /// `message`, `start` or `answer`.
    pub act: String,
    pub at: String,
    /// `sent`, `started` or `answered`.
    pub did: String,
    /// The Job it started, or the ask it answered.
    pub target: Option<String>,
}

impl Store {
    /// Every press on `session`'s pages, oldest first.
    pub fn page_presses(&self, session: &str) -> Result<Vec<KeptPress>, WriteError> {
        let doing = "reading the presses on a job's pages";
        let mut statement = self
            .conn
            .prepare("SELECT artifact, revision, action, item, act, at, did, target FROM page_presses WHERE session_id = ?1 ORDER BY rowid")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session], |row| {
                Ok(KeptPress {
                    artifact: row.get(0)?,
                    revision: row.get(1)?,
                    action: row.get(2)?,
                    item: row.get(3)?,
                    act: row.get(4)?,
                    at: row.get(5)?,
                    did: row.get(6)?,
                    target: row.get(7)?,
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Keep one press.
    pub fn keep_page_press(&mut self, session: &str, kept: &KeptPress) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO page_presses (session_id, artifact, revision, action, item, act, at, did, target) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![session, kept.artifact, kept.revision, kept.action, kept.item, kept.act, kept.at, kept.did, kept.target],
            )
            .map(drop)
            .map_err(fault("keeping a press on a job's page"))
            .map_err(WriteError::Database)
    }
}
