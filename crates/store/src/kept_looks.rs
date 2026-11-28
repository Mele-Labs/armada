//! The look the owner approved on a native walk, one row a (Job, walk). `docs/concepts/session/showing-a-window.md`,
//! *The look he approved*.

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The pictures of one walk as the owner approved it, and the commit they were taken from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptLook {
    pub session_id: String,
    /// The walk's name, the last part of `armada://walk/<name>`.
    pub walk: String,
    /// The address the walk was shown on, which his press named.
    pub url: String,
    pub commit: String,
    pub at: String,
    /// Where the pictures are, one a step.
    pub folder: String,
}

impl Store {
    /// Keep a look, **in place of the Job's older one of the same walk**.
    pub fn keep_look(&mut self, look: &KeptLook) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO kept_looks (session_id, walk, url, commit_sha, at, folder) VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT (session_id, walk) DO UPDATE SET url = ?3, commit_sha = ?4, at = ?5, folder = ?6",
                (&look.session_id, &look.walk, &look.url, &look.commit, &look.at, &look.folder),
            )
            .map(|_| ())
            .map_err(fault("keeping the look he approved"))
            .map_err(WriteError::Database)
    }

    /// Every look the Job has kept, by walk name.
    pub fn kept_looks(&self, session_id: &str) -> Result<Vec<KeptLook>, WriteError> {
        let mut statement = self
            .conn
            .prepare("SELECT walk, url, commit_sha, at, folder FROM kept_looks WHERE session_id = ?1 ORDER BY walk")
            .map_err(fault("reading the looks he approved"))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session_id], |row| {
                Ok(KeptLook {
                    session_id: session_id.to_string(),
                    walk: row.get(0)?,
                    url: row.get(1)?,
                    commit: row.get(2)?,
                    at: row.get(3)?,
                    folder: row.get(4)?,
                })
            })
            .map_err(fault("reading the looks he approved"))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(fault("reading the looks he approved")).map_err(WriteError::Database)
    }
}
