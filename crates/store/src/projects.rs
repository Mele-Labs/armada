//! The projects a person keeps, per repository on this Mac, each an ordered list of rows.
//!
//! **A task keeps its own state and an issue row keeps none**: an issue row is the tracker's
//! reference only, and whether it is open, and its title, are read from the tracker when the rows
//! are. Positions are renumbered from 0 on every move and removal, so they stay a plain order.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One project, its rows in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptProject {
    pub id: String,
    pub manifest_id: String,
    pub name: String,
    pub rows: Vec<KeptProjectRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptProjectRow {
    pub id: String,
    pub item: KeptRowItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptRowItem {
    /// A task he wrote. `notes` is empty for none.
    Task { title: String, notes: String, state: KeptTaskState },
    /// A linked issue: the tracker's reference, as `IssueReference` reads it, and nothing else.
    Issue { reference: String },
}

/// A written task's own state. **More are to come** (a Job holding it, the done? ask), which is why
/// the column is unchecked text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeptTaskState {
    Open,
    Done,
}

impl KeptTaskState {
    fn as_str(self) -> &'static str {
        match self {
            KeptTaskState::Open => "open",
            KeptTaskState::Done => "done",
        }
    }

    /// A state a newer build wrote reads as open here: this build has no word for it.
    fn read(written: &str) -> KeptTaskState {
        match written {
            "done" => KeptTaskState::Done,
            _ => KeptTaskState::Open,
        }
    }
}

const TASK: &str = "task";
const ISSUE: &str = "issue";

fn database(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

/// A row with its project's id, or `None` for a kind this build has no word for, which is skipped.
fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, Option<KeptProjectRow>)> {
    let (project_id, id, kind): (String, String, String) = (row.get(0)?, row.get(1)?, row.get(2)?);
    let item = match kind.as_str() {
        TASK => Some(KeptRowItem::Task {
            title: row.get(3)?,
            notes: row.get(4)?,
            state: KeptTaskState::read(&row.get::<_, String>(6)?),
        }),
        ISSUE => Some(KeptRowItem::Issue { reference: row.get(5)? }),
        _ => None,
    };
    Ok((project_id, item.map(|item| KeptProjectRow { id, item })))
}

impl Store {
    /// One repository's projects in order, each with its rows in order.
    pub fn projects(&self, manifest_id: &str) -> Result<Vec<KeptProject>, WriteError> {
        let doing = "reading the projects";
        let mut statement = self
            .conn
            .prepare("SELECT id, manifest_id, name FROM projects WHERE manifest_id = ?1 ORDER BY position, created, id")
            .map_err(database(doing))?;
        let mut projects = statement
            .query_map(params![manifest_id], |row| {
                Ok(KeptProject { id: row.get(0)?, manifest_id: row.get(1)?, name: row.get(2)?, rows: Vec::new() })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(database(doing))?;
        let mut statement = self
            .conn
            .prepare(
                "SELECT r.project_id, r.id, r.kind, r.title, r.notes, r.issue, r.state
                 FROM project_rows r JOIN projects p ON p.id = r.project_id
                 WHERE p.manifest_id = ?1 ORDER BY r.position, r.id",
            )
            .map_err(database(doing))?;
        let rows = statement
            .query_map(params![manifest_id], read_row)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(database(doing))?;
        for (project_id, row) in rows {
            let (Some(row), Some(project)) = (row, projects.iter_mut().find(|one| one.id == project_id)) else {
                continue;
            };
            project.rows.push(row);
        }
        Ok(projects)
    }

    /// A new project at the end of its repository's list. `at` is when.
    pub fn add_project(&mut self, manifest_id: &str, id: &str, name: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO projects (id, manifest_id, name, position, created)
                 VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(position) + 1, 0) FROM projects WHERE manifest_id = ?2), ?4)",
                params![id, manifest_id, name, at],
            )
            .map(|_| ())
            .map_err(database("keeping a project"))
    }

    /// Whether the repository had a project by that id.
    pub fn rename_project(&mut self, manifest_id: &str, id: &str, name: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE projects SET name = ?3 WHERE manifest_id = ?1 AND id = ?2", params![manifest_id, id, name])
            .map(|changed| changed > 0)
            .map_err(database("renaming a project"))
    }

    /// Drop a project and its rows. Whether the repository had one by that id.
    pub fn delete_project(&mut self, manifest_id: &str, id: &str) -> Result<bool, WriteError> {
        let doing = "deleting a project";
        let tx = self.conn.transaction().map_err(database(doing))?;
        let gone = tx
            .execute("DELETE FROM projects WHERE manifest_id = ?1 AND id = ?2", params![manifest_id, id])
            .map_err(database(doing))?;
        if gone > 0 {
            tx.execute("DELETE FROM project_rows WHERE project_id = ?1", params![id]).map_err(database(doing))?;
        }
        tx.commit().map_err(database(doing))?;
        Ok(gone > 0)
    }

    /// A row at the end of `project_id`'s list.
    pub fn add_project_row(&mut self, project_id: &str, row: &KeptProjectRow) -> Result<(), WriteError> {
        let (kind, title, notes, issue, state) = match &row.item {
            KeptRowItem::Task { title, notes, state } => (TASK, title.as_str(), notes.as_str(), "", state.as_str()),
            KeptRowItem::Issue { reference } => (ISSUE, "", "", reference.as_str(), ""),
        };
        self.conn
            .execute(
                "INSERT INTO project_rows (id, project_id, position, kind, title, notes, issue, state)
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM project_rows WHERE project_id = ?2),
                         ?3, ?4, ?5, ?6, ?7)",
                params![row.id, project_id, kind, title, notes, issue, state],
            )
            .map(|_| ())
            .map_err(database("keeping a project row"))
    }

    /// A task's title and notes. Whether the project had a task by that id.
    pub fn edit_project_task(&mut self, project_id: &str, id: &str, title: &str, notes: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE project_rows SET title = ?3, notes = ?4 WHERE project_id = ?1 AND id = ?2 AND kind = 'task'",
                params![project_id, id, title, notes],
            )
            .map(|changed| changed > 0)
            .map_err(database("editing a project task"))
    }

    /// A task's state. Whether the project had a task by that id.
    pub fn set_project_task_state(&mut self, project_id: &str, id: &str, state: KeptTaskState) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE project_rows SET state = ?3 WHERE project_id = ?1 AND id = ?2 AND kind = 'task'",
                params![project_id, id, state.as_str()],
            )
            .map(|changed| changed > 0)
            .map_err(database("ticking a project task"))
    }

    /// Drop a row. Whether the project had one by that id.
    pub fn remove_project_row(&mut self, project_id: &str, id: &str) -> Result<bool, WriteError> {
        self.reorder_project_rows(project_id, id, None, "removing a project row")
    }

    /// Move a row to `to`, counted from 0 among the project's rows; past the end is the end.
    /// Whether the project had one by that id.
    pub fn move_project_row(&mut self, project_id: &str, id: &str, to: usize) -> Result<bool, WriteError> {
        self.reorder_project_rows(project_id, id, Some(to), "moving a project row")
    }

    /// Take `id` out of the order and, with `to`, put it back there; every position is written again.
    fn reorder_project_rows(
        &mut self,
        project_id: &str,
        id: &str,
        to: Option<usize>,
        doing: &'static str,
    ) -> Result<bool, WriteError> {
        let tx = self.conn.transaction().map_err(database(doing))?;
        let mut order: Vec<String> = {
            let mut statement = tx
                .prepare("SELECT id FROM project_rows WHERE project_id = ?1 ORDER BY position, id")
                .map_err(database(doing))?;
            let ids = statement
                .query_map(params![project_id], |row| row.get(0))
                .and_then(|rows| rows.collect::<Result<Vec<String>, _>>())
                .map_err(database(doing))?;
            ids
        };
        let Some(at) = order.iter().position(|one| one == id) else {
            return Ok(false);
        };
        let taken = order.remove(at);
        match to {
            Some(to) => order.insert(to.min(order.len()), taken),
            None => {
                tx.execute("DELETE FROM project_rows WHERE project_id = ?1 AND id = ?2", params![project_id, id])
                    .map_err(database(doing))?;
            }
        }
        for (position, one) in order.iter().enumerate() {
            tx.execute(
                "UPDATE project_rows SET position = ?3 WHERE project_id = ?1 AND id = ?2",
                params![project_id, one, position as i64],
            )
            .map_err(database(doing))?;
        }
        tx.commit().map_err(database(doing))?;
        Ok(true)
    }
}
