//! The versions a workflow has had in one place, and what a copy of one of Armada's was based on. Protocol
//! `docs/practices/protocol/protocol-23-96-a-workflow-with-versions-and-a-base.md`.
//!
//! **A place is one copy's.** It is keyed as a draft is: the source, the project where that is a project's, the repository
//! only where the source is `repository`, and the workflow's id. **A version is a whole text**, numbered from 1 in the
//! order it was written, so stepping back is putting an earlier version's text in the draft. **A base is the carried text a
//! copy began from**, with the `version` Armada's definition declared then; it is absent where the copy was made before
//! this was kept, and nothing is guessed for it. The hash is the caller's, so this crate takes no hashing dependency.

use rusqlite::{params, OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::workflow_drafts::DraftKey;

/// A version written by a publish.
pub const ORIGIN_PUBLISHED: &str = "published";
/// A version that is the text found in the place when its first publish came.
pub const ORIGIN_FOUND: &str = "found";

/// Which copy: the place and the workflow's id.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct WorkflowPlace {
    /// `armada`, `kit`, `repository` or `project`.
    pub source: String,
    /// The project, where `source` is `project`; empty otherwise.
    pub project: String,
    /// The repository, only where `source` is `repository`; empty otherwise.
    pub manifest_id: String,
    pub item_id: String,
}

impl From<&DraftKey> for WorkflowPlace {
    fn from(key: &DraftKey) -> WorkflowPlace {
        WorkflowPlace { source: key.source.clone(), project: key.project.clone(), manifest_id: key.manifest_id.clone(), item_id: key.item_id.clone() }
    }
}

/// One version, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptVersion {
    pub version: u32,
    pub text: String,
    /// The sha256 of `text`, in hex.
    pub hash: String,
    pub at: String,
    /// [`ORIGIN_PUBLISHED`] or [`ORIGIN_FOUND`].
    pub origin: String,
}

/// A version about to be kept: the number is the store's.
pub struct NewVersion<'a> {
    pub text: &'a str,
    pub hash: &'a str,
    pub at: &'a str,
}

/// What a copy of one of Armada's workflows began from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptBase {
    /// The `version` Armada's carried definition declared.
    pub version: u32,
    /// The sha256 of `text`, in hex.
    pub hash: String,
    pub text: String,
}

fn version_of(row: &Row<'_>) -> rusqlite::Result<KeptVersion> {
    Ok(KeptVersion { version: row.get(0)?, text: row.get(1)?, hash: row.get(2)?, at: row.get(3)?, origin: row.get(4)? })
}

impl Store {
    /// The versions in `place`, newest first. Empty where nothing was published there.
    pub fn workflow_versions(&self, place: &WorkflowPlace) -> Result<Vec<KeptVersion>, WriteError> {
        let doing = "reading a workflow's versions";
        let mut statement = self
            .conn
            .prepare(
                "SELECT version, text, hash, at, origin FROM workflow_versions
                 WHERE source = ?1 AND project = ?2 AND manifest_id = ?3 AND item_id = ?4 ORDER BY version DESC",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let versions = statement
            .query_map(params![place.source, place.project, place.manifest_id, place.item_id], version_of)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(versions)
    }

    /// One version of `place`, or `None` where there is no such number.
    pub fn workflow_version(&self, place: &WorkflowPlace, version: u32) -> Result<Option<KeptVersion>, WriteError> {
        self.conn
            .query_row(
                "SELECT version, text, hash, at, origin FROM workflow_versions
                 WHERE source = ?1 AND project = ?2 AND manifest_id = ?3 AND item_id = ?4 AND version = ?5",
                params![place.source, place.project, place.manifest_id, place.item_id, version],
                version_of,
            )
            .optional()
            .map_err(fault("reading a workflow's version"))
            .map_err(WriteError::Database)
    }

    /// Keep a publish: `published` as the next version, **preceded by `found` where it is not the latest version kept** (the
    /// place has none yet, or the file was edited by hand since), so the history has no hole; and `base`, where one is given,
    /// in place of any the copy had. All or none. The number `published` got.
    pub fn record_workflow_publish(
        &mut self,
        place: &WorkflowPlace,
        found: Option<&NewVersion<'_>>,
        published: &NewVersion<'_>,
        base: Option<&KeptBase>,
    ) -> Result<u32, WriteError> {
        let doing = "keeping a published workflow";
        let tx = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        let held: u32 = tx
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM workflow_versions WHERE source = ?1 AND project = ?2 AND manifest_id = ?3 AND item_id = ?4",
                params![place.source, place.project, place.manifest_id, place.item_id],
                |row| row.get(0),
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let latest: Option<String> = tx
            .query_row(
                "SELECT hash FROM workflow_versions WHERE source = ?1 AND project = ?2 AND manifest_id = ?3 AND item_id = ?4 AND version = ?5",
                params![place.source, place.project, place.manifest_id, place.item_id, held],
                |row| row.get(0),
            )
            .optional()
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let mut number = held;
        let insert = "INSERT INTO workflow_versions (source, project, manifest_id, item_id, version, text, hash, at, origin)
                      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
        let unkept = |found: &&NewVersion<'_>| latest.as_deref() != Some(found.hash);
        for (new, origin) in found.filter(unkept).map(|found| (found, ORIGIN_FOUND)).into_iter().chain([(published, ORIGIN_PUBLISHED)]) {
            number += 1;
            tx.execute(insert, params![place.source, place.project, place.manifest_id, place.item_id, number, new.text, new.hash, new.at, origin])
                .map_err(fault(doing))
                .map_err(WriteError::Database)?;
        }
        if let Some(base) = base {
            put_base(&tx, place, base).map_err(fault(doing)).map_err(WriteError::Database)?;
        }
        tx.commit().map_err(fault(doing)).map_err(WriteError::Database)?;
        Ok(number)
    }

    /// What the copy in `place` began from, or `None` where that is not known.
    pub fn workflow_base(&self, place: &WorkflowPlace) -> Result<Option<KeptBase>, WriteError> {
        self.conn
            .query_row(
                "SELECT version, hash, text FROM workflow_bases WHERE source = ?1 AND project = ?2 AND manifest_id = ?3 AND item_id = ?4",
                params![place.source, place.project, place.manifest_id, place.item_id],
                |row| Ok(KeptBase { version: row.get(0)?, hash: row.get(1)?, text: row.get(2)? }),
            )
            .optional()
            .map_err(fault("reading what a workflow was based on"))
            .map_err(WriteError::Database)
    }

    /// Every base, as the place, the carried `version` and the `hash`, without the text, for a list that marks the copies Armada's
    /// has moved from.
    pub fn workflow_base_heads(&self) -> Result<Vec<(WorkflowPlace, u32, String)>, WriteError> {
        let doing = "reading what workflows were based on";
        let mut statement = self
            .conn
            .prepare("SELECT source, project, manifest_id, item_id, version, hash FROM workflow_bases")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let heads = statement
            .query_map([], |row| {
                Ok((WorkflowPlace { source: row.get(0)?, project: row.get(1)?, manifest_id: row.get(2)?, item_id: row.get(3)? }, row.get(4)?, row.get(5)?))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(heads)
    }

    /// Keep `base` as what the copy in `place` began from, in place of any it had.
    pub fn put_workflow_base(&mut self, place: &WorkflowPlace, base: &KeptBase) -> Result<(), WriteError> {
        put_base(&self.conn, place, base).map_err(fault("keeping what a workflow was based on")).map_err(WriteError::Database)
    }

    /// Drop the versions of `place` and its base: the copy was deleted, or is begun again. `base`, where given, is kept in
    /// their place, as a reset does. All or none.
    pub fn drop_workflow_place(&mut self, place: &WorkflowPlace, base: Option<&KeptBase>) -> Result<(), WriteError> {
        let doing = "dropping a workflow's versions";
        let tx = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        for table in ["workflow_versions", "workflow_bases"] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE source = ?1 AND project = ?2 AND manifest_id = ?3 AND item_id = ?4"),
                params![place.source, place.project, place.manifest_id, place.item_id],
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        }
        if let Some(base) = base {
            put_base(&tx, place, base).map_err(fault(doing)).map_err(WriteError::Database)?;
        }
        tx.commit().map_err(fault(doing)).map_err(WriteError::Database)
    }
}

fn put_base(conn: &rusqlite::Connection, place: &WorkflowPlace, base: &KeptBase) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO workflow_bases (source, project, manifest_id, item_id, version, hash, text) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![place.source, place.project, place.manifest_id, place.item_id, base.version, base.hash, base.text],
    )
    .map(|_| ())
}
