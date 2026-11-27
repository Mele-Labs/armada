//! The sub projects the owner started from the Jobs list. `docs/concepts/feature-catalog.md`, *Sub projects*.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One sub project, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSubProject {
    pub id: String,
    pub project: String,
    pub name: String,
    /// Session ids, in his order.
    pub jobs: Vec<String>,
    /// Feature slugs.
    pub features: Vec<String>,
    pub started_at: String,
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

fn lines(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

fn json(lines: &[String]) -> String {
    serde_json::to_string(lines).unwrap_or_else(|_| String::from("[]"))
}

impl Store {
    /// Keep `sub_project`.
    pub fn put_sub_project(&mut self, sub_project: &KeptSubProject) -> Result<(), WriteError> {
        let doing = "keeping a sub project";
        self.conn
            .execute(
                "INSERT INTO sub_projects (id, project, name, jobs, features, started_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![sub_project.id, sub_project.project, sub_project.name, json(&sub_project.jobs), json(&sub_project.features), sub_project.started_at],
            )
            .map_err(failed(doing))?;
        Ok(())
    }

    /// The sub projects of `project`, or of every project where it is `None`, oldest first.
    pub fn sub_projects(&self, project: Option<&str>) -> Result<Vec<KeptSubProject>, WriteError> {
        let doing = "reading the sub projects";
        let mut statement = self
            .conn
            .prepare("SELECT id, project, name, jobs, features, started_at FROM sub_projects WHERE ?1 IS NULL OR project = ?1 ORDER BY started_at, rowid")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([project], |row| {
                Ok(KeptSubProject {
                    id: row.get(0)?,
                    project: row.get(1)?,
                    name: row.get(2)?,
                    jobs: lines(row.get(3)?),
                    features: lines(row.get(4)?),
                    started_at: row.get(5)?,
                })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Drop the sub project `id`. False where none is kept under it.
    pub fn drop_sub_project(&mut self, id: &str) -> Result<bool, WriteError> {
        let doing = "ungrouping a sub project";
        let dropped = self.conn.execute("DELETE FROM sub_projects WHERE id = ?1", [id]).map_err(failed(doing))?;
        Ok(dropped > 0)
    }
}
