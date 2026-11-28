//! The owner's reasons for how a project's catalog is shaped, one row a reason. `docs/concepts/feature-catalog.md`, *His rules*.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What he did when he gave the reason, as `catalog_rules.act` holds it.
pub mod rule_act {
    pub const MAKE_AREA: &str = "make_area";
    pub const RENAME_AREA: &str = "rename_area";
    pub const SPLIT_AREA: &str = "split_area";
    pub const MERGE_AREAS: &str = "merge_areas";
    pub const MERGE_FEATURES: &str = "merge_features";
    pub const RENAME_FEATURE: &str = "rename_feature";
    pub const MOVE_FEATURE: &str = "move_feature";
    pub const NOT_THIS_FEATURE: &str = "not_this_feature";
    pub const NOT_TOGETHER: &str = "not_together";
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptCatalogRule {
    pub id: String,
    pub project: String,
    pub text: String,
    /// A [`rule_act`].
    pub act: String,
    pub features: Vec<String>,
    pub areas: Vec<String>,
    pub paths: Vec<String>,
    pub at: String,
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

fn json(lines: &[String]) -> String {
    serde_json::to_string(lines).unwrap_or_else(|_| String::from("[]"))
}

fn strings(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

impl Store {
    pub fn keep_catalog_rule(&mut self, rule: &KeptCatalogRule) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO catalog_rules (id, project, text, act, features, areas, paths, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![rule.id, rule.project, rule.text, rule.act, json(&rule.features), json(&rule.areas), json(&rule.paths), rule.at],
            )
            .map(drop)
            .map_err(failed("keeping a catalog rule"))
    }

    /// The rules of `project`, newest first.
    pub fn catalog_rules(&self, project: &str) -> Result<Vec<KeptCatalogRule>, WriteError> {
        let doing = "reading a project's catalog rules";
        let mut statement = self
            .conn
            .prepare("SELECT id, project, text, act, features, areas, paths, at FROM catalog_rules WHERE project = ?1 ORDER BY at DESC, id DESC")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([project], |row| {
                Ok(KeptCatalogRule {
                    id: row.get(0)?,
                    project: row.get(1)?,
                    text: row.get(2)?,
                    act: row.get(3)?,
                    features: strings(row.get(4)?),
                    areas: strings(row.get(5)?),
                    paths: strings(row.get(6)?),
                    at: row.get(7)?,
                })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Take the rule `id` out of `project`'s catalog. False where it held none by that id.
    pub fn drop_catalog_rule(&mut self, project: &str, id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM catalog_rules WHERE project = ?1 AND id = ?2", params![project, id])
            .map(|gone| gone > 0)
            .map_err(failed("dropping a catalog rule"))
    }
}
