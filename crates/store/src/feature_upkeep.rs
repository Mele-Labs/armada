//! What the librarian's upkeep step found of the features and the linked docs: a feature planned or proposed for Retire, a
//! doc gone. `docs/concepts/feature-catalog.md`, *Librarian*. **Fleet never retires a feature**: the owner's answer does,
//! through [`Store::retire_feature`].

use std::collections::BTreeSet;

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What a feature's mark says, as the table names it.
pub mod upkeep_kind {
    /// Its paths match no file at HEAD and never did.
    pub const PLANNED: &str = "planned";
    /// They matched some once and match none now.
    pub const RETIRE_GONE: &str = "retire_gone";
    /// Files, but nothing touched them and no Job was on it for months.
    pub const RETIRE_UNUSED: &str = "retire_unused";
}

/// One feature's mark as the last check found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptUpkeep {
    pub slug: String,
    pub kind: String,
    /// Empty for `planned`.
    pub evidence: String,
    pub checked_at: String,
    /// When the owner answered Keep to this kind and evidence. Absent until he does.
    pub kept_at: Option<String>,
}

/// What one check found of the feature `slug`: nothing, or a kind with its evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpkeepFound {
    pub slug: String,
    pub found: Option<(String, String)>,
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// Keep what a check of `project` found at `at`. **One row a feature**: no finding clears it, a finding of the kind and
    /// evidence already held keeps the owner's Keep, and any other replaces the row with no Keep. A row of a slug the check
    /// did not name is cleared, so a renamed feature leaves none behind. One transaction.
    pub fn keep_upkeep(&mut self, project: &str, at: &str, checked: &[UpkeepFound]) -> Result<(), WriteError> {
        let doing = "keeping what the upkeep check found of the features";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        let named: BTreeSet<&str> = checked.iter().map(|one| one.slug.as_str()).collect();
        let held: Vec<String> = {
            let mut statement = tx.prepare("SELECT slug FROM feature_upkeep WHERE project = ?1").map_err(failed(doing))?;
            let rows = statement.query_map([project], |row| row.get::<_, String>(0)).map_err(failed(doing))?;
            rows.collect::<Result<_, _>>().map_err(failed(doing))?
        };
        for slug in held.iter().filter(|slug| !named.contains(slug.as_str())) {
            tx.execute("DELETE FROM feature_upkeep WHERE project = ?1 AND slug = ?2", params![project, slug]).map_err(failed(doing))?;
        }
        for one in checked {
            let Some((kind, evidence)) = &one.found else {
                tx.execute("DELETE FROM feature_upkeep WHERE project = ?1 AND slug = ?2", params![project, one.slug]).map_err(failed(doing))?;
                continue;
            };
            tx.execute(
                "INSERT INTO feature_upkeep (project, slug, kind, evidence, checked_at, kept_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL)
                 ON CONFLICT (project, slug) DO UPDATE SET
                    kept_at = CASE WHEN kind = excluded.kind AND evidence = excluded.evidence THEN kept_at ELSE NULL END,
                    kind = excluded.kind, evidence = excluded.evidence, checked_at = excluded.checked_at",
                params![project, one.slug, kind, evidence, at],
            )
            .map_err(failed(doing))?;
        }
        tx.commit().map_err(failed(doing))
    }

    /// Every mark of `project`, by slug.
    pub fn upkeep_in(&self, project: &str) -> Result<Vec<KeptUpkeep>, WriteError> {
        let doing = "reading what the upkeep check found of a project's features";
        let mut statement = self.conn.prepare("SELECT slug, kind, evidence, checked_at, kept_at FROM feature_upkeep WHERE project = ?1 ORDER BY slug").map_err(failed(doing))?;
        let rows = statement
            .query_map([project], |row| Ok(KeptUpkeep { slug: row.get(0)?, kind: row.get(1)?, evidence: row.get(2)?, checked_at: row.get(3)?, kept_at: row.get(4)? }))
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The owner kept `slug` on the evidence it is proposed on. Whether a proposal to retire it was held.
    pub fn keep_retire_proposal(&mut self, project: &str, slug: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE feature_upkeep SET kept_at = ?3 WHERE project = ?1 AND slug = ?2 AND kind IN (?4, ?5)",
                params![project, slug, at, upkeep_kind::RETIRE_GONE, upkeep_kind::RETIRE_UNUSED],
            )
            .map(|changed| changed > 0)
            .map_err(failed("keeping a feature the librarian proposed to retire"))
    }

    /// Take the feature `slug` out of `project`'s catalog: the row, its prefixes and its mark. **Its links, sections and the
    /// files Jobs filed under it stay where they are**, under the id it had. Whether it was there. No alias is kept: a trailer
    /// that names it again is a feature a Job named, which waits on the owner as any does.
    pub fn retire_feature(&mut self, project: &str, slug: &str) -> Result<bool, WriteError> {
        let doing = "retiring a feature";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        let id: Option<String> = tx
            .query_row("SELECT id FROM features WHERE project = ?1 AND slug = ?2", params![project, slug], |row| row.get(0))
            .optional()
            .map_err(failed(doing))?;
        let Some(id) = id else { return Ok(false) };
        tx.execute("DELETE FROM feature_paths WHERE feature_id = ?1", [&id]).map_err(failed(doing))?;
        tx.execute("DELETE FROM features WHERE id = ?1", [&id]).map_err(failed(doing))?;
        tx.execute("DELETE FROM feature_upkeep WHERE project = ?1 AND slug = ?2", params![project, slug]).map_err(failed(doing))?;
        tx.commit().map_err(failed(doing))?;
        Ok(true)
    }

    /// Keep which of `project`'s linked docs a check found gone, in place of what was kept.
    pub fn keep_docs_gone(&mut self, project: &str, at: &str, gone: &[String]) -> Result<(), WriteError> {
        let doing = "keeping the linked docs the upkeep check found gone";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        tx.execute("DELETE FROM feature_doc_gone WHERE project = ?1", [project]).map_err(failed(doing))?;
        for target in gone {
            tx.execute("INSERT OR REPLACE INTO feature_doc_gone (project, target, checked_at) VALUES (?1, ?2, ?3)", params![project, target, at]).map_err(failed(doing))?;
        }
        tx.commit().map_err(failed(doing))
    }

    /// The linked docs of `project` the last check found gone.
    pub fn docs_gone_in(&self, project: &str) -> Result<BTreeSet<String>, WriteError> {
        let doing = "reading the linked docs found gone";
        let mut statement = self.conn.prepare("SELECT target FROM feature_doc_gone WHERE project = ?1").map_err(failed(doing))?;
        let rows = statement.query_map([project], |row| row.get::<_, String>(0)).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
