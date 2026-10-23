//! The issue queries a person saved, kept per repository on this Mac.
//!
//! **Theirs, and Fleet's only to run**: the query is the tracker's own language as written, so it
//! is kept as text and never read. A save by an id already kept edits that row in place, which is
//! how a hand edit lands, and the row keeps its place in the list.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Joins the row fields in their column. A field name is a tracker's display name and holds no control character.
const JOIN: char = '\u{1f}';

/// One saved query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptIssueQuery {
    pub id: String,
    pub manifest_id: String,
    pub name: String,
    /// What the person said, which the query was written from. Empty for one typed by hand.
    pub words: String,
    /// In the repository's tracker's own language.
    pub query: String,
    /// The tracker's field names a row shows, in order.
    pub row_fields: Vec<String>,
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptIssueQuery> {
    let joined: String = row.get(5)?;
    Ok(KeptIssueQuery {
        id: row.get(0)?,
        manifest_id: row.get(1)?,
        name: row.get(2)?,
        words: row.get(3)?,
        query: row.get(4)?,
        row_fields: joined.split(JOIN).filter(|one| !one.is_empty()).map(String::from).collect(),
    })
}

const COLUMNS: &str = "id, manifest_id, name, words, query, row_fields";

impl Store {
    /// Keep `saved`: a new row at the end of its repository's list, or the row with this id edited
    /// where it is. `at` is when it was first kept.
    pub fn keep_issue_query(&mut self, saved: &KeptIssueQuery, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO issue_queries (id, manifest_id, name, words, query, row_fields, kept_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (id) DO UPDATE SET name = ?3, words = ?4, query = ?5, row_fields = ?6",
                params![
                    saved.id,
                    saved.manifest_id,
                    saved.name,
                    saved.words,
                    saved.query,
                    saved.row_fields.join(&JOIN.to_string()),
                    at
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping an issue query"))
            .map_err(WriteError::Database)
    }

    /// The queries saved for one repository, oldest first.
    pub fn issue_queries(&self, manifest_id: &str) -> Result<Vec<KeptIssueQuery>, WriteError> {
        let doing = "reading the saved issue queries";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM issue_queries WHERE manifest_id = ?1 ORDER BY kept_at, id"
            ))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(params![manifest_id], read)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }

    /// One saved query of one repository.
    pub fn issue_query(&self, manifest_id: &str, id: &str) -> Result<Option<KeptIssueQuery>, WriteError> {
        let doing = "reading a saved issue query";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM issue_queries WHERE manifest_id = ?1 AND id = ?2"))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let mut rows = statement
            .query_map(params![manifest_id, id], read)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows.pop())
    }

    /// Drop a saved query. Whether there was one.
    pub fn forget_issue_query(&mut self, manifest_id: &str, id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM issue_queries WHERE manifest_id = ?1 AND id = ?2", params![manifest_id, id])
            .map(|gone| gone > 0)
            .map_err(fault("forgetting a saved issue query"))
            .map_err(WriteError::Database)
    }
}
