//! The sub projects the owner started from the Jobs list. `docs/concepts/feature-catalog.md`, *Sub projects*.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One sub project, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSubProject {
    pub id: String,
    pub project: String,
    pub name: String,
    /// His words. The name, on one kept before goals.
    pub goal: String,
    /// Session ids, in his order.
    pub jobs: Vec<String>,
    /// Feature slugs.
    pub features: Vec<String>,
    /// The session id of the Job that orchestrates it.
    pub orchestrator: Option<String>,
    pub started_at: String,
    /// His last Not yet on its Goal met? card.
    pub not_yet_at: Option<String>,
    /// His Agree, which folded it away.
    pub ended_at: Option<String>,
}

const COLUMNS: &str = "id, project, name, goal, jobs, features, orchestrator, started_at, not_yet_at, ended_at";

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

fn lines(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

fn json(lines: &[String]) -> String {
    serde_json::to_string(lines).unwrap_or_else(|_| String::from("[]"))
}

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptSubProject> {
    Ok(KeptSubProject {
        id: row.get(0)?,
        project: row.get(1)?,
        name: row.get(2)?,
        goal: row.get(3)?,
        jobs: lines(row.get(4)?),
        features: lines(row.get(5)?),
        orchestrator: row.get(6)?,
        started_at: row.get(7)?,
        not_yet_at: row.get(8)?,
        ended_at: row.get(9)?,
    })
}

impl Store {
    /// Keep `sub_project`.
    pub fn put_sub_project(&mut self, sub_project: &KeptSubProject) -> Result<(), WriteError> {
        let doing = "keeping a sub project";
        self.conn
            .execute(
                "INSERT INTO sub_projects (id, project, name, goal, jobs, features, orchestrator, started_at, not_yet_at, ended_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    sub_project.id,
                    sub_project.project,
                    sub_project.name,
                    sub_project.goal,
                    json(&sub_project.jobs),
                    json(&sub_project.features),
                    sub_project.orchestrator,
                    sub_project.started_at,
                    sub_project.not_yet_at,
                    sub_project.ended_at,
                ],
            )
            .map_err(failed(doing))?;
        Ok(())
    }

    /// The sub projects of `project`, or of every project where it is `None`, oldest first: the open ones, or the ones
    /// folded away where `ended`.
    pub fn sub_projects(&self, project: Option<&str>, ended: bool) -> Result<Vec<KeptSubProject>, WriteError> {
        let doing = "reading the sub projects";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM sub_projects WHERE (?1 IS NULL OR project = ?1) AND (ended_at IS NOT NULL) = ?2 ORDER BY started_at, rowid"
            ))
            .map_err(failed(doing))?;
        let rows = statement.query_map(params![project, ended], kept).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The sub project `id`.
    pub fn sub_project(&self, id: &str) -> Result<Option<KeptSubProject>, WriteError> {
        let doing = "reading a sub project";
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM sub_projects WHERE id = ?1"), [id], kept)
            .optional()
            .map_err(failed(doing))
    }

    /// The open sub project `session` orchestrates.
    pub fn sub_project_orchestrated_by(&self, session: &str) -> Result<Option<KeptSubProject>, WriteError> {
        let doing = "reading the sub project a Job orchestrates";
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM sub_projects WHERE orchestrator = ?1 AND ended_at IS NULL ORDER BY started_at, rowid LIMIT 1"),
                [session],
                kept,
            )
            .optional()
            .map_err(failed(doing))
    }

    /// Marks the open sub project `id`, kept without an orchestrator, as asked for one at `at`. False where it has one,
    /// was asked before or is folded away, so only one read starts it.
    pub fn ask_sub_project_orchestrator(&mut self, id: &str, at: &str) -> Result<bool, WriteError> {
        let doing = "asking for a sub project's orchestrator";
        let asked = self
            .conn
            .execute(
                "UPDATE sub_projects SET orchestrator_asked_at = ?2
                 WHERE id = ?1 AND orchestrator IS NULL AND orchestrator_asked_at IS NULL AND ended_at IS NULL",
                params![id, at],
            )
            .map_err(failed(doing))?;
        Ok(asked > 0)
    }

    /// The sub project `id` is orchestrated by `session`.
    pub fn set_sub_project_orchestrator(&mut self, id: &str, session: &str) -> Result<(), WriteError> {
        let doing = "keeping a sub project's orchestrator";
        self.conn.execute("UPDATE sub_projects SET orchestrator = ?2 WHERE id = ?1", params![id, session]).map_err(failed(doing))?;
        Ok(())
    }

    /// His Not yet on the open sub project `id`. False where it is folded away or not kept.
    pub fn sub_project_not_yet(&mut self, id: &str, at: &str) -> Result<bool, WriteError> {
        let doing = "keeping a Not yet on a sub project";
        let moved = self
            .conn
            .execute("UPDATE sub_projects SET not_yet_at = ?2 WHERE id = ?1 AND ended_at IS NULL", params![id, at])
            .map_err(failed(doing))?;
        Ok(moved > 0)
    }

    /// His Agree folds the open sub project `id` away. False where it is folded already or not kept.
    pub fn end_sub_project(&mut self, id: &str, at: &str) -> Result<bool, WriteError> {
        let doing = "folding a sub project away";
        let moved = self
            .conn
            .execute("UPDATE sub_projects SET ended_at = ?2 WHERE id = ?1 AND ended_at IS NULL", params![id, at])
            .map_err(failed(doing))?;
        Ok(moved > 0)
    }

    /// Drop the sub project `id`. False where none is kept under it.
    pub fn drop_sub_project(&mut self, id: &str) -> Result<bool, WriteError> {
        let doing = "ungrouping a sub project";
        let dropped = self.conn.execute("DELETE FROM sub_projects WHERE id = ?1", [id]).map_err(failed(doing))?;
        Ok(dropped > 0)
    }
}
