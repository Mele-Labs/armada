//! What the person remembers of a pull request between visits: the head he last left it at, the
//! files he marked viewed, the review Fleet last wrote for him, and the Reviews layouts he named. `docs/concepts/pull-review.md`,
//! *Since you looked*.
//!
//! **Plain text here**: the arrangement is opaque JSON, and which file is still viewed is Fleet's.

use rusqlite::{OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The head a person last left a pull request at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSeenHead {
    pub manifest_id: String,
    /// As the forge names it: `owner/name`.
    pub repository: String,
    pub number: u64,
    pub head: String,
    pub seen_at: String,
}

/// A file a person marked viewed on a pull request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptViewedFile {
    pub manifest_id: String,
    pub repository: String,
    pub number: u64,
    pub path: String,
    /// The commit he read it at, whole.
    pub commit: String,
    /// The forge's blob for the file at `commit`, where Bridge gave it.
    pub blob: Option<String>,
    pub viewed_at: String,
}

/// A named Reviews layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptReviewLayout {
    pub name: String,
    /// The pane-tree arrangement, as Bridge wrote it.
    pub arrangement: String,
    pub saved_at: String,
}

fn number_of(number: u64) -> i64 {
    i64::try_from(number).unwrap_or(i64::MAX)
}

fn seen(row: &Row<'_>) -> rusqlite::Result<KeptSeenHead> {
    Ok(KeptSeenHead {
        manifest_id: row.get(0)?,
        repository: row.get(1)?,
        number: row.get::<_, i64>(2)?.try_into().unwrap_or_default(),
        head: row.get(3)?,
        seen_at: row.get(4)?,
    })
}

fn viewed(row: &Row<'_>) -> rusqlite::Result<KeptViewedFile> {
    Ok(KeptViewedFile {
        manifest_id: row.get(0)?,
        repository: row.get(1)?,
        number: row.get::<_, i64>(2)?.try_into().unwrap_or_default(),
        path: row.get(3)?,
        commit: row.get(4)?,
        blob: row.get(5)?,
        viewed_at: row.get(6)?,
    })
}

/// The review Fleet last wrote for a person on a pull request, as it wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWrittenReview {
    pub manifest_id: String,
    /// As the forge names it: `owner/name`.
    pub repository: String,
    pub number: u64,
    /// `approved`, `changes_requested` or `commented`.
    pub said: String,
    /// The commit it was written on, whole, where Fleet knew it.
    pub commit: Option<String>,
    pub submitted_at: String,
}

fn written(row: &Row<'_>) -> rusqlite::Result<KeptWrittenReview> {
    Ok(KeptWrittenReview {
        manifest_id: row.get(0)?,
        repository: row.get(1)?,
        number: row.get::<_, i64>(2)?.try_into().unwrap_or_default(),
        said: row.get(3)?,
        commit: row.get(4)?,
        submitted_at: row.get(5)?,
    })
}

fn layout(row: &Row<'_>) -> rusqlite::Result<KeptReviewLayout> {
    Ok(KeptReviewLayout { name: row.get(0)?, arrangement: row.get(1)?, saved_at: row.get(2)? })
}

impl Store {
    /// Keep the head a person left a pull request at, replacing the one before.
    pub fn keep_seen_head(&mut self, kept: &KeptSeenHead) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO pull_request_seen (manifest_id, repository, number, head, seen_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![kept.manifest_id, kept.repository, number_of(kept.number), kept.head, kept.seen_at],
            )
            .map(|_| ())
            .map_err(fault("keeping a seen head"))
            .map_err(WriteError::Database)
    }

    /// The head a person last left one pull request at, if he has looked.
    pub fn seen_head(&self, manifest_id: &str, repository: &str, number: u64) -> Result<Option<KeptSeenHead>, WriteError> {
        self.conn
            .query_row(
                "SELECT manifest_id, repository, number, head, seen_at FROM pull_request_seen \
                 WHERE manifest_id = ?1 AND repository = ?2 AND number = ?3",
                rusqlite::params![manifest_id, repository, number_of(number)],
                seen,
            )
            .optional()
            .map_err(fault("reading a seen head"))
            .map_err(WriteError::Database)
    }

    /// Every seen head kept, for the lists to look each row up in.
    pub fn seen_heads(&self) -> Result<Vec<KeptSeenHead>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading seen heads")(why));
        let mut statement = self
            .conn
            .prepare("SELECT manifest_id, repository, number, head, seen_at FROM pull_request_seen")
            .map_err(unreadable)?;
        let rows = statement.query_map([], seen).map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// Keep a file as viewed, replacing an older mark on the same path.
    pub fn keep_viewed_file(&mut self, kept: &KeptViewedFile) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO pull_request_viewed \
                 (manifest_id, repository, number, path, commit_sha, blob, viewed_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    kept.manifest_id,
                    kept.repository,
                    number_of(kept.number),
                    kept.path,
                    kept.commit,
                    kept.blob,
                    kept.viewed_at
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a viewed file"))
            .map_err(WriteError::Database)
    }

    /// Forget a file's viewed mark. One not marked is not an error.
    pub fn forget_viewed_file(&mut self, manifest_id: &str, repository: &str, number: u64, path: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM pull_request_viewed WHERE manifest_id = ?1 AND repository = ?2 AND number = ?3 AND path = ?4",
                rusqlite::params![manifest_id, repository, number_of(number), path],
            )
            .map(|_| ())
            .map_err(fault("forgetting a viewed file"))
            .map_err(WriteError::Database)
    }

    /// Every file marked viewed on one pull request, by path.
    pub fn viewed_files(&self, manifest_id: &str, repository: &str, number: u64) -> Result<Vec<KeptViewedFile>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading viewed files")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT manifest_id, repository, number, path, commit_sha, blob, viewed_at FROM pull_request_viewed \
                 WHERE manifest_id = ?1 AND repository = ?2 AND number = ?3 ORDER BY path",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map(rusqlite::params![manifest_id, repository, number_of(number)], viewed)
            .map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// Keep a named layout, replacing one of the same name.
    pub fn keep_review_layout(&mut self, kept: &KeptReviewLayout) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO review_layouts (name, arrangement, saved_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![kept.name, kept.arrangement, kept.saved_at],
            )
            .map(|_| ())
            .map_err(fault("keeping a review layout"))
            .map_err(WriteError::Database)
    }

    /// Forget a named layout, and the pick where it named this one. One not kept is not an error.
    pub fn forget_review_layout(&mut self, name: &str) -> Result<(), WriteError> {
        let wrote = |why: rusqlite::Error| WriteError::Database(fault("forgetting a review layout")(why));
        let transaction = self.conn.transaction().map_err(wrote)?;
        transaction
            .execute("DELETE FROM review_layouts WHERE name = ?1", rusqlite::params![name])
            .map_err(wrote)?;
        transaction
            .execute("DELETE FROM review_layout_picked WHERE name = ?1", rusqlite::params![name])
            .map_err(wrote)?;
        transaction.commit().map_err(wrote)
    }

    /// Every named layout kept, by name.
    pub fn review_layouts(&self) -> Result<Vec<KeptReviewLayout>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading review layouts")(why));
        let mut statement = self
            .conn
            .prepare("SELECT name, arrangement, saved_at FROM review_layouts ORDER BY name")
            .map_err(unreadable)?;
        let rows = statement.query_map([], layout).map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// Name the layout every pull request opens on.
    pub fn pick_review_layout(&mut self, name: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO review_layout_picked (id, name) VALUES (1, ?1)",
                rusqlite::params![name],
            )
            .map(|_| ())
            .map_err(fault("picking a review layout"))
            .map_err(WriteError::Database)
    }

    /// Keep the review Fleet just wrote for a person, replacing the one before.
    pub fn keep_written_review(&mut self, kept: &KeptWrittenReview) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO pull_request_reviewed \
                 (manifest_id, repository, number, said, commit_sha, submitted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    kept.manifest_id,
                    kept.repository,
                    number_of(kept.number),
                    kept.said,
                    kept.commit,
                    kept.submitted_at
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a written review"))
            .map_err(WriteError::Database)
    }

    /// Every review Fleet wrote, the latest for each pull request.
    pub fn written_reviews(&self) -> Result<Vec<KeptWrittenReview>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading written reviews")(why));
        let mut statement = self
            .conn
            .prepare("SELECT manifest_id, repository, number, said, commit_sha, submitted_at FROM pull_request_reviewed")
            .map_err(unreadable)?;
        let rows = statement.query_map([], written).map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// The layout picked, if one was.
    pub fn picked_review_layout(&self) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT name FROM review_layout_picked WHERE id = 1", [], |row| row.get(0))
            .optional()
            .map_err(fault("reading the picked review layout"))
            .map_err(WriteError::Database)
    }
}
