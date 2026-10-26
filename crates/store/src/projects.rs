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

/// The Job holding a task: its id, and its title where it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptHolder {
    pub session: String,
    pub title: Option<String>,
}

impl KeptHolder {
    pub fn of(session: &str) -> KeptHolder {
        KeptHolder { session: session.to_string(), title: None }
    }
}

/// A written task's own state. The column is unchecked text, so a state a newer build writes reads
/// as open. A held or asked task always names its Job, which is why the Job rides on the variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptTaskState {
    Open,
    Done,
    /// A Job started with it and has not finished.
    Held(KeptHolder),
    /// The Job finished, and whether the task is done is yet to be said. **Outlives the Job.**
    Asked(KeptHolder),
}

impl KeptTaskState {
    fn as_str(&self) -> &'static str {
        match self {
            KeptTaskState::Open => "open",
            KeptTaskState::Done => "done",
            KeptTaskState::Held(_) => "held",
            KeptTaskState::Asked(_) => "asked",
        }
    }

    fn holder(&self) -> &str {
        match self {
            KeptTaskState::Held(holder) | KeptTaskState::Asked(holder) => &holder.session,
            KeptTaskState::Open | KeptTaskState::Done => "",
        }
    }

    /// A state a newer build wrote reads as open here: this build has no word for it. So does a
    /// held or asked one that names no Job.
    fn read(written: &str, held_by: String, title: Option<String>) -> KeptTaskState {
        let holder = KeptHolder { session: held_by, title };
        match written {
            "done" => KeptTaskState::Done,
            "held" if !holder.session.is_empty() => KeptTaskState::Held(holder),
            "asked" if !holder.session.is_empty() => KeptTaskState::Asked(holder),
            _ => KeptTaskState::Open,
        }
    }
}

/// The tasks one Job asked about in one project, for the ask on that Job's list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAsked {
    pub project_id: String,
    pub project_name: String,
    /// The earliest any of them was asked.
    pub since: String,
    /// Each task's id and title, in the project's order.
    pub tasks: Vec<(String, String)>,
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
            state: KeptTaskState::read(&row.get::<_, String>(6)?, row.get(7)?, row.get(8)?),
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
                "SELECT r.project_id, r.id, r.kind, r.title, r.notes, r.issue, r.state, r.held_by, s.title
                 FROM project_rows r JOIN projects p ON p.id = r.project_id
                 LEFT JOIN sessions s ON s.id = r.held_by AND r.held_by != ''
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
        let (kind, title, notes, issue, state, held_by) = match &row.item {
            KeptRowItem::Task { title, notes, state } => {
                (TASK, title.as_str(), notes.as_str(), "", state.as_str(), state.holder())
            }
            KeptRowItem::Issue { reference } => (ISSUE, "", "", reference.as_str(), "", ""),
        };
        self.conn
            .execute(
                "INSERT INTO project_rows (id, project_id, position, kind, title, notes, issue, state, held_by)
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM project_rows WHERE project_id = ?2),
                         ?3, ?4, ?5, ?6, ?7, ?8)",
                params![row.id, project_id, kind, title, notes, issue, state, held_by],
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

    /// A task's state. Whether the project had a task by that id. **A person's tick overrides a
    /// hold**: the Job it named is dropped whatever the state it is moved to.
    pub fn set_project_task_state(&mut self, project_id: &str, id: &str, state: KeptTaskState) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE project_rows SET state = ?3, held_by = ?4, asked_at = ''
                 WHERE project_id = ?1 AND id = ?2 AND kind = 'task'",
                params![project_id, id, state.as_str(), state.holder()],
            )
            .map(|changed| changed > 0)
            .map_err(database("ticking a project task"))
    }

    /// The repository a project belongs to, if it is a project of any.
    pub fn project_manifest(&self, project_id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT manifest_id FROM projects WHERE id = ?1", params![project_id], |row| row.get(0))
            .map(Some)
            .or_else(|cause| match cause {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(database("reading a project"))
    }

    /// Put `session` on each task named, `(project id, row id)`: it holds them from now. A task it
    /// was already holding, or another Job was, is held by this one.
    pub fn hold_project_tasks(&mut self, session: &str, tasks: &[(String, String)]) -> Result<(), WriteError> {
        let doing = "holding project tasks";
        let tx = self.conn.transaction().map_err(database(doing))?;
        for (project, row) in tasks {
            tx.execute(
                "UPDATE project_rows SET state = 'held', held_by = ?3, asked_at = ''
                 WHERE project_id = ?1 AND id = ?2 AND kind = 'task'",
                params![project, row, session],
            )
            .map_err(database(doing))?;
        }
        tx.commit().map_err(database(doing))
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
