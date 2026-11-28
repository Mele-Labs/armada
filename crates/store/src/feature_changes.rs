//! A feature renamed or merged into another, and an area made, renamed, split or merged. `docs/concepts/feature-catalog.md`,
//! *Confirming and merging* and *Areas*.
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

/// Keep the area `slug` of `project` right after `after`, or after every area where `after` is `None`, and number every
/// area's place again so none share one.
fn area_placed(tx: &Transaction<'_>, project: &str, slug: &str, title: &str, after: Option<&str>) -> rusqlite::Result<()> {
    let mut order: Vec<String> = {
        let mut statement = tx.prepare("SELECT slug FROM feature_areas WHERE project = ?1 ORDER BY position, slug")?;
        let rows = statement.query_map([project], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let at = after.and_then(|after| order.iter().position(|one| one == after)).map_or(order.len(), |found| found + 1);
    order.insert(at, slug.to_string());
    tx.execute("INSERT INTO feature_areas (project, slug, title, position) VALUES (?1, ?2, ?3, ?4)", params![project, slug, title, at as u32])?;
    for (place, one) in order.iter().enumerate() {
        tx.execute("UPDATE feature_areas SET position = ?3 WHERE project = ?1 AND slug = ?2", params![project, one, place as u32])?;
    }
    Ok(())
}

impl Store {
    /// Keep a new area of `project` after every area it keeps. **The caller checks the slug is free.**
    pub fn make_feature_area(&mut self, project: &str, slug: &str, title: &str) -> Result<(), WriteError> {
        let doing = "making a feature area";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        area_placed(&tx, project, slug, title, None).map_err(failed(doing))?;
        tx.commit().map_err(failed(doing))
    }

    /// Give the area `from` of `project` the slug `to`, and a new title where one is given; its features go with it. **The
    /// caller checks `to` is free.**
    pub fn rename_feature_area(&mut self, project: &str, from: &str, to: &str, title: Option<&str>) -> Result<(), WriteError> {
        let doing = "renaming a feature area";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        tx.execute("UPDATE feature_areas SET slug = ?3, title = COALESCE(?4, title) WHERE project = ?1 AND slug = ?2", params![project, from, to, title])
            .map_err(failed(doing))?;
        if from != to {
            tx.execute("UPDATE features SET area = ?3 WHERE project = ?1 AND area = ?2", params![project, from, to]).map_err(failed(doing))?;
        }
        tx.commit().map_err(failed(doing))
    }

    /// Keep the area `slug` right after `from`, and move the features `features` of `project` into it. **The caller checks
    /// the slug is free and each feature is in `from`.**
    pub fn split_feature_area(&mut self, project: &str, from: &str, slug: &str, title: &str, features: &[String]) -> Result<(), WriteError> {
        let doing = "splitting a feature area";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        area_placed(&tx, project, slug, title, Some(from)).map_err(failed(doing))?;
        for feature in features {
            tx.execute("UPDATE features SET area = ?3 WHERE project = ?1 AND slug = ?2", params![project, feature, slug]).map_err(failed(doing))?;
        }
        tx.commit().map_err(failed(doing))
    }

    /// Move every feature of the area `from` of `project` into `into`, after those `into` holds, and take `from` out. **The
    /// caller checks both are kept and differ.**
    pub fn merge_feature_area(&mut self, project: &str, from: &str, into: &str) -> Result<(), WriteError> {
        let doing = "merging a feature area";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        let after: i64 = tx
            .query_row("SELECT COALESCE(MAX(position) + 1, 0) FROM features WHERE project = ?1 AND area = ?2", params![project, into], |row| row.get(0))
            .map_err(failed(doing))?;
        tx.execute("UPDATE features SET area = ?3, position = position + ?4 WHERE project = ?1 AND area = ?2", params![project, from, into, after])
            .map_err(failed(doing))?;
        tx.execute("DELETE FROM feature_areas WHERE project = ?1 AND slug = ?2", params![project, from]).map_err(failed(doing))?;
        tx.commit().map_err(failed(doing))
    }

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
        step("UPDATE OR IGNORE feature_job_named SET feature_id = ?1 WHERE feature_id = ?2", &[&kept, &gone])?;
        step("UPDATE OR IGNORE feature_doc_checks SET feature_id = ?1 WHERE feature_id = ?2", &[&kept, &gone])?;
        for table in ["feature_links", "feature_job_files", "feature_job_named", "feature_doc_checks", "feature_sections", "feature_paths"] {
            step(&format!("DELETE FROM {table} WHERE feature_id = ?1"), &[&gone])?;
        }
        step("DELETE FROM features WHERE id = ?1", &[&gone])?;
        aliased(&tx, project, from, into).map_err(failed(doing))?;
        asks_moved(&tx, project, from, into).map_err(failed(doing))?;
        tx.commit().map_err(failed(doing))
    }
}
