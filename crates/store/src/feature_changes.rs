//! A feature renamed or merged into another. `docs/concepts/feature-catalog.md`, *Confirming and merging*.
//!
//! **Each is one transaction.** The old slug stays as an alias of the feature it became, so a trailer already in git, or an act
//! that still names it, files under the right feature. An ask filed under the old slug is filed under the new.

use rusqlite::{params, OptionalExtension, Transaction};

use crate::error::WriteError;
use crate::open::Store;

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(crate::error::fault(doing)(why))
}

/// The asks of `project` filed under `from` are filed under `to`, and under `to` once where both held it.
fn asks_moved(tx: &Transaction<'_>, project: &str, from: &str, to: &str) -> rusqlite::Result<()> {
    const OF_PROJECT: &str = "ask_id IN (SELECT ask_id FROM story_asks WHERE project = ?1)";
    tx.execute(&format!("UPDATE OR IGNORE story_ask_features SET slug = ?3 WHERE slug = ?2 AND {OF_PROJECT}"), params![project, from, to])?;
    tx.execute(&format!("DELETE FROM story_ask_features WHERE slug = ?2 AND {OF_PROJECT}"), params![project, from])?;
    Ok(())
}

/// `from` is known as `to` from now on.
fn aliased(tx: &Transaction<'_>, project: &str, from: &str, to: &str) -> rusqlite::Result<()> {
    tx.execute("UPDATE feature_aliases SET slug = ?3 WHERE project = ?1 AND slug = ?2", params![project, from, to])?;
    tx.execute("DELETE FROM feature_aliases WHERE project = ?1 AND alias = ?2", params![project, to])?;
    tx.execute("INSERT OR REPLACE INTO feature_aliases (project, alias, slug) VALUES (?1, ?2, ?3)", params![project, from, to])?;
    Ok(())
}

impl Store {
    /// Give the feature `from` of `project` the slug `to`, and a new title where one is given. **The caller checks `to` is
    /// free.** A `to` equal to `from` changes the title only.
    pub fn rename_feature(&mut self, project: &str, from: &str, to: &str, title: Option<&str>) -> Result<(), WriteError> {
        let doing = "renaming a feature";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        tx.execute(
            "UPDATE features SET slug = ?3, title = COALESCE(?4, title) WHERE project = ?1 AND slug = ?2",
            params![project, from, to, title],
        )
        .map_err(failed(doing))?;
        if from != to {
            aliased(&tx, project, from, to).map_err(failed(doing))?;
            asks_moved(&tx, project, from, to).map_err(failed(doing))?;
        }
        tx.commit().map_err(failed(doing))
    }

    /// Fold the feature `from` of `project` into `into`. **Everything it held moves**: its prefixes and entry points join the
    /// target's, its links and files are the target's, and a section the target never wrote is copied. The target keeps its
    /// id, its state and its `added_at`. The caller checks both exist and differ.
    pub fn merge_feature(&mut self, project: &str, from: &str, into: &str) -> Result<(), WriteError> {
        let doing = "merging a feature";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        let id_of = |slug: &str| -> rusqlite::Result<Option<String>> {
            tx.query_row("SELECT id FROM features WHERE project = ?1 AND slug = ?2", params![project, slug], |row| row.get(0)).optional()
        };
        let (Some(gone), Some(kept)) = (id_of(from).map_err(failed(doing))?, id_of(into).map_err(failed(doing))?) else {
            return Ok(());
        };
        let points = |id: &str| -> rusqlite::Result<Vec<String>> {
            let raw: String = tx.query_row("SELECT entry_points FROM features WHERE id = ?1", [id], |row| row.get(0))?;
            Ok(serde_json::from_str(&raw).unwrap_or_default())
        };
        let mut entry_points = points(&kept).map_err(failed(doing))?;
        for point in points(&gone).map_err(failed(doing))? {
            if !entry_points.contains(&point) {
                entry_points.push(point);
            }
        }
        let entry_points = serde_json::to_string(&entry_points).unwrap_or_else(|_| String::from("[]"));
        let step = |sql: &str, args: &[&dyn rusqlite::ToSql]| tx.execute(sql, args).map(drop).map_err(failed(doing));
        step("UPDATE features SET entry_points = ?2 WHERE id = ?1", &[&kept, &entry_points])?;
        step("INSERT OR IGNORE INTO feature_paths (feature_id, prefix) SELECT ?1, prefix FROM feature_paths WHERE feature_id = ?2", &[&kept, &gone])?;
        step(
            "INSERT OR IGNORE INTO feature_sections (feature_id, section, text, job, written_at, at_commit)
             SELECT ?1, section, text, job, written_at, at_commit FROM feature_sections WHERE feature_id = ?2",
            &[&kept, &gone],
        )?;
        step("UPDATE OR IGNORE feature_links SET feature_id = ?1 WHERE feature_id = ?2", &[&kept, &gone])?;
        step("UPDATE OR IGNORE feature_job_files SET feature_id = ?1 WHERE feature_id = ?2", &[&kept, &gone])?;
        for table in ["feature_links", "feature_job_files", "feature_sections", "feature_paths"] {
            step(&format!("DELETE FROM {table} WHERE feature_id = ?1"), &[&gone])?;
        }
        step("DELETE FROM features WHERE id = ?1", &[&gone])?;
        aliased(&tx, project, from, into).map_err(failed(doing))?;
        asks_moved(&tx, project, from, into).map_err(failed(doing))?;
        tx.commit().map_err(failed(doing))
    }
}
