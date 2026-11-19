//! The open pull requests of the Reviews lists with the files each changes, for *Who's on what*.
//! `docs/concepts/feature-catalog.md`. The files are read from the forge once a head; a pull request that is no
//! longer on the lists is dropped.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One row of `open_pull_files`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptOpenPull {
    pub manifest_id: String,
    pub number: u64,
    /// `owner/name`, as the forge names the repository.
    pub repository: String,
    pub title: String,
    pub author: String,
    pub draft: bool,
    /// The commit the files were read at, whole.
    pub head_sha: String,
    pub files: Vec<String>,
    /// The pull request changes more files than `files` holds.
    pub files_capped: bool,
    /// RFC 3339, when the files were read.
    pub read_at: String,
}

impl Store {
    /// Keep a pull request, replacing the row before it.
    pub fn keep_open_pull(&mut self, pull: &KeptOpenPull) -> Result<(), WriteError> {
        let files = serde_json::to_string(&pull.files).unwrap_or_else(|_| String::from("[]"));
        self.conn
            .execute(
                "INSERT OR REPLACE INTO open_pull_files \
                 (manifest_id, number, repository, title, author, draft, head_sha, files, files_capped, read_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![pull.manifest_id, pull.number, pull.repository, pull.title, pull.author, pull.draft, pull.head_sha, files, pull.files_capped, pull.read_at],
            )
            .map(|_| ())
            .map_err(fault("keeping an open pull request's files"))
            .map_err(WriteError::Database)
    }

    /// Every pull request kept, oldest number first.
    pub fn open_pulls_kept(&self) -> Result<Vec<KeptOpenPull>, WriteError> {
        let doing = "reading the open pull requests' files";
        let mut statement = self
            .conn
            .prepare(
                "SELECT manifest_id, number, repository, title, author, draft, head_sha, files, files_capped, read_at \
                 FROM open_pull_files ORDER BY manifest_id, number",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| {
                let files: String = row.get(7)?;
                Ok(KeptOpenPull {
                    manifest_id: row.get(0)?,
                    number: row.get(1)?,
                    repository: row.get(2)?,
                    title: row.get(3)?,
                    author: row.get(4)?,
                    draft: row.get(5)?,
                    head_sha: row.get(6)?,
                    files: serde_json::from_str(&files).unwrap_or_default(),
                    files_capped: row.get(8)?,
                    read_at: row.get(9)?,
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Drop the rows of `manifest_id` whose number is not in `open`: they are no longer open. Answers how many went.
    pub fn drop_open_pulls_except(&mut self, manifest_id: &str, open: &[u64]) -> Result<usize, WriteError> {
        let doing = "dropping the closed pull requests' files";
        let mut dropped = 0;
        for kept in self.open_pulls_kept()?.into_iter().filter(|kept| kept.manifest_id == manifest_id && !open.contains(&kept.number)) {
            dropped += self
                .conn
                .execute("DELETE FROM open_pull_files WHERE manifest_id = ?1 AND number = ?2", params![kept.manifest_id, kept.number])
                .map_err(fault(doing))
                .map_err(WriteError::Database)?;
        }
        Ok(dropped)
    }
}
