//! Where changes to an area of a repository run, and the bays held for walks.
//! `docs/concepts/pull-review.md`, *Walk*.
//!
//! **Plain text here**: `how` is the wire's word and validation is Fleet's, as it is for a run's
//! `kind`.

use rusqlite::Row;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One row of `walk_places`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWalkPlace {
    pub id: String,
    pub manifest_id: String,
    /// A directory, two deep at most, no trailing slash. Empty is the repository's root.
    pub area: String,
    pub how: String,
    pub address: String,
    pub command: String,
    pub api: String,
    pub saved_at: String,
}

/// One row of `review_walks`: a bay a pulled-down walk holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldReviewWalk {
    pub id: String,
    pub manifest_id: String,
    pub number: u64,
    pub slot: u32,
    pub holder: String,
    /// The bay's path, which is what the walk's server is held by.
    pub path: String,
    pub opened_at: String,
}

const PLACE: &str = "id, manifest_id, area, how, address, command, api, saved_at";
const WALK: &str = "id, manifest_id, number, slot, holder, path, opened_at";

fn place(row: &Row<'_>) -> rusqlite::Result<KeptWalkPlace> {
    Ok(KeptWalkPlace {
        id: row.get(0)?,
        manifest_id: row.get(1)?,
        area: row.get(2)?,
        how: row.get(3)?,
        address: row.get(4)?,
        command: row.get(5)?,
        api: row.get(6)?,
        saved_at: row.get(7)?,
    })
}

fn walk(row: &Row<'_>) -> rusqlite::Result<HeldReviewWalk> {
    Ok(HeldReviewWalk {
        id: row.get(0)?,
        manifest_id: row.get(1)?,
        number: row.get::<_, i64>(2)?.try_into().unwrap_or_default(),
        slot: row.get::<_, i64>(3)?.try_into().unwrap_or_default(),
        holder: row.get(4)?,
        path: row.get(5)?,
        opened_at: row.get(6)?,
    })
}

impl Store {
    /// Keep a place, **replacing the one at its repository and area** and leaving that row's id
    /// where it was. Answers the row as kept.
    pub fn keep_walk_place(&mut self, kept: &KeptWalkPlace) -> Result<KeptWalkPlace, WriteError> {
        let wrote = |why: rusqlite::Error| WriteError::Database(fault("keeping a walk place")(why));
        self.conn
            .execute(
                "INSERT INTO walk_places (id, manifest_id, area, how, address, command, api, saved_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT (manifest_id, area) DO UPDATE SET \
                 how = ?4, address = ?5, command = ?6, api = ?7, saved_at = ?8",
                rusqlite::params![
                    kept.id,
                    kept.manifest_id,
                    kept.area,
                    kept.how,
                    kept.address,
                    kept.command,
                    kept.api,
                    kept.saved_at
                ],
            )
            .map_err(wrote)?;
        self.conn
            .query_row(
                &format!("SELECT {PLACE} FROM walk_places WHERE manifest_id = ?1 AND area = ?2"),
                rusqlite::params![kept.manifest_id, kept.area],
                place,
            )
            .map_err(wrote)
    }

    /// Every place kept for one repository, by area.
    pub fn walk_places_of(&self, manifest_id: &str) -> Result<Vec<KeptWalkPlace>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading walk places")(why));
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {PLACE} FROM walk_places WHERE manifest_id = ?1 ORDER BY area"))
            .map_err(unreadable)?;
        let rows = statement.query_map(rusqlite::params![manifest_id], place).map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// Every place kept, by repository then area.
    pub fn walk_places(&self) -> Result<Vec<KeptWalkPlace>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading walk places")(why));
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {PLACE} FROM walk_places ORDER BY manifest_id, area"))
            .map_err(unreadable)?;
        let rows = statement.query_map([], place).map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// Forget a place. One already gone is not an error.
    pub fn forget_walk_place(&mut self, manifest_id: &str, id: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM walk_places WHERE manifest_id = ?1 AND id = ?2",
                rusqlite::params![manifest_id, id],
            )
            .map(|_| ())
            .map_err(fault("forgetting a walk place"))
            .map_err(WriteError::Database)
    }

    /// Hold a bay for a walk.
    pub fn keep_review_walk(&mut self, held: &HeldReviewWalk) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!("INSERT OR REPLACE INTO review_walks ({WALK}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"),
                rusqlite::params![
                    held.id,
                    held.manifest_id,
                    i64::try_from(held.number).unwrap_or(i64::MAX),
                    i64::from(held.slot),
                    held.holder,
                    held.path,
                    held.opened_at
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a review walk"))
            .map_err(WriteError::Database)
    }

    /// Every bay a walk holds, oldest first.
    pub fn review_walks_held(&self) -> Result<Vec<HeldReviewWalk>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading review walks")(why));
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {WALK} FROM review_walks ORDER BY opened_at, id"))
            .map_err(unreadable)?;
        let rows = statement.query_map([], walk).map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// A walk's bay is given back. One already gone is not an error.
    pub fn release_review_walk(&mut self, id: &str) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM review_walks WHERE id = ?1", rusqlite::params![id])
            .map(|_| ())
            .map_err(fault("releasing a review walk"))
            .map_err(WriteError::Database)
    }
}
