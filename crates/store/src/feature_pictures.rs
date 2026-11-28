//! The pictures of what a feature looks like in the running app. `docs/concepts/feature-catalog.md`, *Pictures*.
//!
//! **Rows only, the file is Fleet's**, under its support folder by project and feature. A picture he removed from a feature
//! keeps its row with when and why, so what he taught stays.

use std::collections::BTreeMap;

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Who took a picture, as the table names it.
pub mod picture_by {
    /// A Job, with the act that attached it.
    pub const JOB: &str = "job";
    /// The librarian, on a run.
    pub const LIBRARIAN: &str = "librarian";
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPicture {
    pub id: String,
    pub feature_id: String,
    pub project: String,
    /// Where the file is under Fleet's pictures folder.
    pub file: String,
    /// The page's address, empty for a screen that has none.
    pub url: String,
    /// The screen's name where it has no address, as a native walk is named. Empty for a page.
    pub screen: String,
    /// The commit the app was built from, empty where it could not be read.
    pub at_commit: String,
    pub taken_at: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    /// The Job that took it, empty for the librarian.
    pub job: String,
    /// One of [`picture_by`].
    pub taken_by: String,
    /// When he said it is not of this feature. Absent while it stands.
    pub removed_at: Option<String>,
    /// What he gave with it, empty where nothing.
    pub removed_why: String,
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

const COLUMNS: &str = "id, feature_id, project, file, url, screen, at_commit, taken_at, width, height, bytes, job, taken_by, removed_at, removed_why";

fn picture_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptPicture> {
    Ok(KeptPicture {
        id: row.get(0)?,
        feature_id: row.get(1)?,
        project: row.get(2)?,
        file: row.get(3)?,
        url: row.get(4)?,
        screen: row.get(5)?,
        at_commit: row.get(6)?,
        taken_at: row.get(7)?,
        width: row.get(8)?,
        height: row.get(9)?,
        bytes: row.get::<_, i64>(10)?.try_into().unwrap_or(0),
        job: row.get(11)?,
        taken_by: row.get(12)?,
        removed_at: row.get(13)?,
        removed_why: row.get(14)?,
    })
}

impl Store {
    /// Keep a picture taken of a feature. `picture.removed_at` and `removed_why` are not read: a picture is kept standing.
    pub fn keep_feature_picture(&mut self, picture: &KeptPicture) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO feature_pictures (id, feature_id, project, file, url, screen, at_commit, taken_at, width, height, bytes, job, taken_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    picture.id,
                    picture.feature_id,
                    picture.project,
                    picture.file,
                    picture.url,
                    picture.screen,
                    picture.at_commit,
                    picture.taken_at,
                    picture.width,
                    picture.height,
                    i64::try_from(picture.bytes).unwrap_or(i64::MAX),
                    picture.job,
                    picture.taken_by,
                ],
            )
            .map(drop)
            .map_err(failed("keeping a picture of a feature"))
    }

    /// Every picture of `feature_id`, removed ones too, newest first.
    pub fn feature_pictures(&self, feature_id: &str) -> Result<Vec<KeptPicture>, WriteError> {
        let doing = "reading the pictures of a feature";
        let mut statement =
            self.conn.prepare(&format!("SELECT {COLUMNS} FROM feature_pictures WHERE feature_id = ?1 ORDER BY taken_at DESC, id DESC")).map_err(failed(doing))?;
        let rows = statement.query_map([feature_id], picture_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// One picture by its id, removed or not.
    pub fn feature_picture(&self, id: &str) -> Result<Option<KeptPicture>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM feature_pictures WHERE id = ?1"), [id], picture_of)
            .optional()
            .map_err(failed("reading a picture of a feature"))
    }

    /// The cover of each feature of `project` that has a standing picture: its newest, by feature id.
    pub fn feature_covers(&self, project: &str) -> Result<BTreeMap<String, String>, WriteError> {
        let doing = "reading the covers of a project's features";
        let mut statement = self
            .conn
            .prepare("SELECT feature_id, id FROM feature_pictures WHERE project = ?1 AND removed_at IS NULL ORDER BY taken_at ASC, id ASC")
            .map_err(failed(doing))?;
        let rows = statement.query_map([project], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(failed(doing))?;
        // Oldest first, so the newest of a feature is the one left in the map.
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// He said picture `id` is not of its feature, at `at`, with `why`. Whether it stood.
    pub fn remove_feature_picture(&mut self, id: &str, at: &str, why: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE feature_pictures SET removed_at = ?2, removed_why = ?3 WHERE id = ?1 AND removed_at IS NULL", params![id, at, why])
            .map(|changed| changed > 0)
            .map_err(failed("removing a picture from a feature"))
    }

    /// The row of a picture the librarian took again, gone with its file. **Only the librarian's own standing take is
    /// forgotten**: a Job's picture and a removed one stay.
    pub fn forget_librarian_picture(&mut self, id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM feature_pictures WHERE id = ?1 AND taken_by = ?2 AND removed_at IS NULL", params![id, picture_by::LIBRARIAN])
            .map(|changed| changed > 0)
            .map_err(failed("forgetting a picture the librarian took again"))
    }
}
