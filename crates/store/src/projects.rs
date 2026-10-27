//! The projects a person keeps on this Mac, each an ordered list of rows, and the tasks in no project.
//!
//! **A project is his and spans repositories**: it has no repository, and every row names its own
//! (`manifest_id`). A task in no project is loose, with a place among the loose rows, and its
//! project id is the empty string. A row id is unique across all of them, so a row is found by it.
//! **A task keeps its own state and an issue row keeps none**: an issue row is the tracker's
//! reference only, and whether it is open, and its title, are read from the tracker when the rows
//! are. Positions are renumbered from 0 on every move and removal, so they stay a plain order.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The project id a loose task is kept under.
pub const LOOSE: &str = "";

/// Every project in his order, each with its rows in order, and the loose rows in theirs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptProjects {
    pub projects: Vec<KeptProject>,
    pub loose: Vec<KeptProjectRow>,
}

/// One project, its rows in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptProject {
    pub id: String,
    pub name: String,
    pub rows: Vec<KeptProjectRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptProjectRow {
    pub id: String,
    /// The repository the row is in, which moving it between projects keeps.
    pub manifest_id: String,
    pub item: KeptRowItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptRowItem {
    /// A task he wrote. `notes` is empty for none. `parent` is the id of the task it is a subtask of,
    /// one level deep, `issues` the references of the issues linked to it, and `pulls` the numbers of the pull
    /// requests linked to it, each in the order linked.
    Task {
        title: String,
        notes: String,
        state: KeptTaskState,
        parent: Option<String>,
        issues: Vec<String>,
        pulls: Vec<u64>,
    },
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
/// as open. A held task always names its Job, which is why the Job rides on the variant; an asked one
/// names the Job that finished, and none where a linked pull request's merge asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptTaskState {
    Open,
    Done,
    /// A Job started with it and has not finished.
    Held(KeptHolder),
    /// The Job finished, and whether the task is done is yet to be said. **Outlives the Job.**
    Asked(Option<KeptHolder>),
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
            KeptTaskState::Held(holder) | KeptTaskState::Asked(Some(holder)) => &holder.session,
            KeptTaskState::Open | KeptTaskState::Done | KeptTaskState::Asked(None) => "",
        }
    }

    /// A state a newer build wrote reads as open here: this build has no word for it. So does a
    /// held one that names no Job.
    fn read(written: &str, held_by: String, title: Option<String>) -> KeptTaskState {
        let holder = KeptHolder { session: held_by, title };
        match written {
            "done" => KeptTaskState::Done,
            "held" if !holder.session.is_empty() => KeptTaskState::Held(holder),
            "asked" => KeptTaskState::Asked(Some(holder).filter(|holder| !holder.session.is_empty())),
            _ => KeptTaskState::Open,
        }
    }
}

/// The tasks one Job asked about in one project, for the ask on that Job's list. `project_id` is
/// [`LOOSE`] and `project_name` empty for the tasks in no project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAsked {
    pub project_id: String,
    pub project_name: String,
    /// The earliest any of them was asked.
    pub since: String,
    /// Each task's id and title, in the project's order.
    pub tasks: Vec<(String, String)>,
}

/// Rows in their project's order, each top-level row followed by its subtasks. A subtask whose parent
/// is not among them is left at the end, so nothing read is lost.
fn nested(rows: Vec<KeptProjectRow>) -> Vec<KeptProjectRow> {
    let parent_of = |row: &KeptProjectRow| match &row.item {
        KeptRowItem::Task { parent, .. } => parent.clone(),
        KeptRowItem::Issue { .. } => None,
    };
    let (subtasks, tops): (Vec<_>, Vec<_>) = rows.into_iter().partition(|row| parent_of(row).is_some());
    let mut ordered = Vec::with_capacity(tops.len() + subtasks.len());
    let mut remaining = subtasks;
    for top in tops {
        let id = top.id.clone();
        ordered.push(top);
        let (own, rest): (Vec<_>, Vec<_>) = remaining.into_iter().partition(|row| parent_of(row).as_deref() == Some(id.as_str()));
        ordered.extend(own);
        remaining = rest;
    }
    ordered.extend(remaining);
    ordered
}

const TASK: &str = "task";
const ISSUE: &str = "issue";

fn database(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

/// A row with its project's id, or `None` for a kind this build has no word for, which is skipped.
fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, Option<KeptProjectRow>)> {
    let manifest_id: String = row.get(10)?;
    let (project_id, id, kind): (String, String, String) = (row.get(0)?, row.get(1)?, row.get(2)?);
    let item = match kind.as_str() {
        TASK => Some(KeptRowItem::Task {
            title: row.get(3)?,
            notes: row.get(4)?,
            state: KeptTaskState::read(&row.get::<_, String>(6)?, row.get(7)?, row.get(8)?),
            parent: Some(row.get::<_, String>(9)?).filter(|parent| !parent.is_empty()),
            issues: Vec::new(),
            pulls: Vec::new(),
        }),
        ISSUE => Some(KeptRowItem::Issue { reference: row.get(5)? }),
        _ => None,
    };
    Ok((project_id, item.map(|item| KeptProjectRow { id, manifest_id, item })))
}

impl Store {
    /// Every project in order, each with its rows in order, and the loose rows.
    pub fn projects(&self) -> Result<KeptProjects, WriteError> {
        let doing = "reading the projects";
        let mut statement = self.conn.prepare("SELECT id, name FROM projects ORDER BY position, created, id").map_err(database(doing))?;
        let mut projects = statement
            .query_map([], |row| Ok(KeptProject { id: row.get(0)?, name: row.get(1)?, rows: Vec::new() }))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(database(doing))?;
        let mut statement = self
            .conn
            .prepare(
                "SELECT r.project_id, r.id, r.kind, r.title, r.notes, r.issue, r.state, r.held_by, s.title, r.parent, r.manifest_id
                 FROM project_rows r
                 LEFT JOIN sessions s ON s.id = r.held_by AND r.held_by != ''
                 ORDER BY r.position, r.id",
            )
            .map_err(database(doing))?;
        let mut rows: Vec<(String, KeptProjectRow)> = statement
            .query_map([], read_row)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(database(doing))?
            .into_iter()
            .filter_map(|(project, row)| row.map(|row| (project, row)))
            .collect();
        let at: std::collections::HashMap<String, usize> = rows.iter().enumerate().map(|(at, (_, row))| (row.id.clone(), at)).collect();
        let mut statement = self.conn.prepare("SELECT task_id, issue FROM project_task_issues ORDER BY position").map_err(database(doing))?;
        let linked = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(database(doing))?;
        for (task, reference) in linked {
            if let Some((_, KeptProjectRow { item: KeptRowItem::Task { issues, .. }, .. })) = at.get(&task).map(|at| &mut rows[*at]) {
                issues.push(reference);
            }
        }
        let mut statement = self.conn.prepare("SELECT task_id, pull FROM project_task_pulls ORDER BY position").map_err(database(doing))?;
        let linked = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(database(doing))?;
        for (task, number) in linked {
            if let Some((_, KeptProjectRow { item: KeptRowItem::Task { pulls, .. }, .. })) = at.get(&task).map(|at| &mut rows[*at]) {
                pulls.push(number as u64);
            }
        }
        let mut loose = Vec::new();
        for (project_id, row) in rows {
            match projects.iter_mut().find(|one| one.id == project_id) {
                Some(project) => project.rows.push(row),
                None if project_id == LOOSE => loose.push(row),
                None => {}
            }
        }
        for project in &mut projects {
            project.rows = nested(std::mem::take(&mut project.rows));
        }
        Ok(KeptProjects { projects, loose: nested(loose) })
    }

    /// A new project at the end of his list. `at` is when.
    pub fn add_project(&mut self, id: &str, name: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO projects (id, name, position, created)
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM projects), ?3)",
                params![id, name, at],
            )
            .map(|_| ())
            .map_err(database("keeping a project"))
    }

    /// Whether he had a project by that id.
    pub fn rename_project(&mut self, id: &str, name: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE projects SET name = ?2 WHERE id = ?1", params![id, name])
            .map(|changed| changed > 0)
            .map_err(database("renaming a project"))
    }

    /// Drop a project and its rows. Whether he had one by that id.
    pub fn delete_project(&mut self, id: &str) -> Result<bool, WriteError> {
        let doing = "deleting a project";
        let tx = self.conn.transaction().map_err(database(doing))?;
        let gone = tx.execute("DELETE FROM projects WHERE id = ?1", params![id]).map_err(database(doing))?;
        if gone > 0 {
            tx.execute(
                "DELETE FROM project_task_issues WHERE task_id IN (SELECT id FROM project_rows WHERE project_id = ?1)",
                params![id],
            )
            .map_err(database(doing))?;
            tx.execute(
                "DELETE FROM project_task_pulls WHERE task_id IN (SELECT id FROM project_rows WHERE project_id = ?1)",
                params![id],
            )
            .map_err(database(doing))?;
            tx.execute("DELETE FROM project_rows WHERE project_id = ?1", params![id]).map_err(database(doing))?;
        }
        tx.commit().map_err(database(doing))?;
        Ok(gone > 0)
    }

    /// A row at the end of `project_id`'s list ([`LOOSE`] for none), or of its parent's subtasks.
    pub fn add_project_row(&mut self, project_id: &str, row: &KeptProjectRow) -> Result<(), WriteError> {
        let (kind, title, notes, issue, state, held_by, parent) = match &row.item {
            KeptRowItem::Task { title, notes, state, parent, .. } => (
                TASK,
                title.as_str(),
                notes.as_str(),
                "",
                state.as_str(),
                state.holder(),
                parent.as_deref().unwrap_or(""),
            ),
            KeptRowItem::Issue { reference } => (ISSUE, "", "", reference.as_str(), "", "", ""),
        };
        self.conn
            .execute(
                "INSERT INTO project_rows (id, project_id, position, kind, title, notes, issue, state, held_by, parent, manifest_id)
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM project_rows WHERE project_id = ?2 AND parent = ?9),
                         ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![row.id, project_id, kind, title, notes, issue, state, held_by, parent, row.manifest_id],
            )
            .map(|_| ())
            .map_err(database("keeping a project row"))
    }

    /// A task's title and notes. Whether there was a task by that id.
    pub fn edit_project_task(&mut self, id: &str, title: &str, notes: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE project_rows SET title = ?2, notes = ?3 WHERE id = ?1 AND kind = 'task'", params![id, title, notes])
            .map(|changed| changed > 0)
            .map_err(database("editing a project task"))
    }

    /// A task's state. Whether there was a task by that id. **A person's tick overrides a
    /// hold**: the Job it named is dropped whatever the state it is moved to.
    pub fn set_project_task_state(&mut self, id: &str, state: KeptTaskState) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE project_rows SET state = ?2, held_by = ?3, asked_at = '' WHERE id = ?1 AND kind = 'task'",
                params![id, state.as_str(), state.holder()],
            )
            .map(|changed| changed > 0)
            .map_err(database("ticking a project task"))
    }

    /// Put `session` on each task named, by row id: it holds them from now. A task it
    /// was already holding, or another Job was, is held by this one.
    pub fn hold_project_tasks(&mut self, session: &str, tasks: &[String]) -> Result<(), WriteError> {
        let doing = "holding project tasks";
        let tx = self.conn.transaction().map_err(database(doing))?;
        for row in tasks {
            tx.execute(
                "UPDATE project_rows SET state = 'held', held_by = ?2, asked_at = '' WHERE id = ?1 AND kind = 'task'",
                params![row, session],
            )
            .map_err(database(doing))?;
        }
        tx.commit().map_err(database(doing))
    }

    /// What `session` holds, `held` and now finished: each task is `asked` from `at`. How many.
    pub fn ask_held_project_tasks(&mut self, session: &str, at: &str) -> Result<usize, WriteError> {
        self.conn
            .execute(
                "UPDATE project_rows SET state = 'asked', asked_at = ?2 WHERE held_by = ?1 AND state = 'held'",
                params![session, at],
            )
            .map_err(database("asking about project tasks"))
    }

    /// What `session` holds and has not finished, let go: each task is `open` again. How many.
    pub fn free_held_project_tasks(&mut self, session: &str) -> Result<usize, WriteError> {
        self.conn
            .execute(
                "UPDATE project_rows SET state = 'open', held_by = '', asked_at = '' WHERE held_by = ?1 AND state = 'held'",
                params![session],
            )
            .map_err(database("letting go of project tasks"))
    }

    /// The tasks `session` asked about and nobody has settled, a group per project in the order the
    /// projects are kept, the tasks in no project last.
    pub fn asked_project_tasks(&self, session: &str) -> Result<Vec<KeptAsked>, WriteError> {
        let doing = "reading the project tasks asked about";
        let mut statement = self
            .conn
            .prepare(
                "SELECT r.project_id, COALESCE(p.name, ''), r.id, r.title, r.asked_at
                 FROM project_rows r LEFT JOIN projects p ON p.id = r.project_id
                 WHERE r.held_by = ?1 AND r.state = 'asked'
                 ORDER BY p.position IS NULL, p.position, p.created, r.project_id, r.position, r.id",
            )
            .map_err(database(doing))?;
        let rows = statement
            .query_map(params![session], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(database(doing))?;
        let mut asked: Vec<KeptAsked> = Vec::new();
        for (project_id, project_name, id, title, at) in rows {
            match asked.last_mut().filter(|last| last.project_id == project_id) {
                Some(last) => {
                    last.since = last.since.clone().min(at);
                    last.tasks.push((id, title));
                }
                None => asked.push(KeptAsked { project_id, project_name, since: at, tasks: vec![(id, title)] }),
            }
        }
        Ok(asked)
    }

    /// The ask `session` raised on `project_id` ([`LOOSE`] for the tasks in none), answered: its tasks are `done`, or `open` again. How many.
    pub fn settle_asked_project_tasks(&mut self, session: &str, project_id: &str, done: bool) -> Result<usize, WriteError> {
        let state = if done { KeptTaskState::Done } else { KeptTaskState::Open };
        self.conn
            .execute(
                "UPDATE project_rows SET state = ?3, held_by = '', asked_at = ''
                 WHERE held_by = ?1 AND project_id = ?2 AND state = 'asked'",
                params![session, project_id, state.as_str()],
            )
            .map_err(database("settling project tasks"))
    }

    /// The project and parent of row `id` (`""` for none of either); `None` where there is no such row.
    fn place_of(tx: &rusqlite::Transaction<'_>, id: &str, doing: &'static str) -> Result<Option<(String, String)>, WriteError> {
        tx.query_row("SELECT project_id, parent FROM project_rows WHERE id = ?1", params![id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map(Some)
            .or_else(|cause| match cause {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(database(doing))
    }

    /// The ids of `parent`'s rows in `project_id` ('' for the top level), in order.
    fn siblings(tx: &rusqlite::Transaction<'_>, project_id: &str, parent: &str, doing: &'static str) -> Result<Vec<String>, WriteError> {
        let mut statement = tx
            .prepare("SELECT id FROM project_rows WHERE project_id = ?1 AND parent = ?2 ORDER BY position, id")
            .map_err(database(doing))?;
        let ids = statement
            .query_map(params![project_id, parent], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<String>, _>>())
            .map_err(database(doing))?;
        Ok(ids)
    }

    fn renumbered(tx: &rusqlite::Transaction<'_>, order: &[String], doing: &'static str) -> Result<(), WriteError> {
        for (position, one) in order.iter().enumerate() {
            tx.execute("UPDATE project_rows SET position = ?2 WHERE id = ?1", params![one, position as i64]).map_err(database(doing))?;
        }
        Ok(())
    }

    /// Drop a row, and its subtasks and the issues linked to either. Whether there was one by that id.
    pub fn remove_project_row(&mut self, id: &str) -> Result<bool, WriteError> {
        let doing = "removing a project row";
        let tx = self.conn.transaction().map_err(database(doing))?;
        let Some((project, parent)) = Self::place_of(&tx, id, doing)? else {
            return Ok(false);
        };
        let mut gone = Self::siblings(&tx, &project, id, doing)?;
        gone.push(id.to_string());
        for one in &gone {
            tx.execute("DELETE FROM project_task_issues WHERE task_id = ?1", params![one]).map_err(database(doing))?;
            tx.execute("DELETE FROM project_task_pulls WHERE task_id = ?1", params![one]).map_err(database(doing))?;
            tx.execute("DELETE FROM project_rows WHERE id = ?1", params![one]).map_err(database(doing))?;
        }
        let order = Self::siblings(&tx, &project, &parent, doing)?;
        Self::renumbered(&tx, &order, doing)?;
        tx.commit().map_err(database(doing))?;
        Ok(true)
    }

    /// Move a row to `to`, counted from 0 among the rows of the parent it ends up under; past the end
    /// is the end. `project` is the project it goes to ([`LOOSE`] for none) and `None` leaves it in
    /// the one it is in; `parent` is the task it goes under, `""` for the top level, and `None` leaves
    /// it under the one it has, or at the top level where the project changes. **A row keeps its repository**,
    /// and its subtasks go with it. Whether there was one by that id.
    pub fn move_project_row(&mut self, id: &str, project: Option<&str>, to: usize, parent: Option<&str>) -> Result<bool, WriteError> {
        let doing = "moving a project row";
        let tx = self.conn.transaction().map_err(database(doing))?;
        let Some((was_project, was_parent)) = Self::place_of(&tx, id, doing)? else {
            return Ok(false);
        };
        let into = project.unwrap_or(&was_project);
        let under = parent.unwrap_or(if into == was_project { &was_parent } else { "" });
        let mut order = Self::siblings(&tx, into, under, doing)?;
        order.retain(|one| one != id);
        order.insert(to.min(order.len()), id.to_string());
        tx.execute("UPDATE project_rows SET project_id = ?2, parent = ?3 WHERE id = ?1", params![id, into, under]).map_err(database(doing))?;
        tx.execute("UPDATE project_rows SET project_id = ?2 WHERE parent = ?1", params![id, into]).map_err(database(doing))?;
        Self::renumbered(&tx, &order, doing)?;
        if (into, under) != (was_project.as_str(), was_parent.as_str()) {
            let left = Self::siblings(&tx, &was_project, &was_parent, doing)?;
            Self::renumbered(&tx, &left, doing)?;
        }
        tx.commit().map_err(database(doing))?;
        Ok(true)
    }

    /// Link an issue, by reference, to a task; linking it again changes nothing. Whether there was a task by that id.
    pub fn link_task_issue(&mut self, id: &str, reference: &str) -> Result<bool, WriteError> {
        self.task_issue(id, "linking an issue to a task", |tx| {
            tx.execute(
                "INSERT OR IGNORE INTO project_task_issues (task_id, issue, position)
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM project_task_issues WHERE task_id = ?1))",
                params![id, reference],
            )
        })
    }

    /// Take an issue off a task; one it did not have changes nothing. Whether there was a task by that id.
    pub fn unlink_task_issue(&mut self, id: &str, reference: &str) -> Result<bool, WriteError> {
        self.task_issue(id, "unlinking an issue from a task", |tx| {
            tx.execute("DELETE FROM project_task_issues WHERE task_id = ?1 AND issue = ?2", params![id, reference])
        })
    }

    /// Link a pull request, by number, to a task; linking it again changes nothing. Whether there was a task by that id.
    pub fn link_task_pull(&mut self, id: &str, number: u64) -> Result<bool, WriteError> {
        self.task_issue(id, "linking a pull request to a task", |tx| {
            tx.execute(
                "INSERT OR IGNORE INTO project_task_pulls (task_id, pull, position)
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM project_task_pulls WHERE task_id = ?1))",
                params![id, number as i64],
            )
        })
    }

    /// Take a pull request off a task; one it did not have changes nothing. Whether there was a task by that id.
    pub fn unlink_task_pull(&mut self, id: &str, number: u64) -> Result<bool, WriteError> {
        self.task_issue(id, "unlinking a pull request from a task", |tx| {
            tx.execute("DELETE FROM project_task_pulls WHERE task_id = ?1 AND pull = ?2", params![id, number as i64])
        })
    }

    /// Pull request `number` of `manifest_id` merged: each task it is linked to that has not been asked
    /// about it, and is open or held, is `asked` from `at`. A Job holding one keeps holding it. **A link
    /// asks once**, so a task answered `open` is not asked again by the same merge. Who held each task
    /// that moved, `""` for nobody.
    pub fn ask_pulled_project_tasks(&mut self, manifest_id: &str, number: u64, at: &str) -> Result<Vec<String>, WriteError> {
        let doing = "asking about project tasks a pull request merged for";
        let tx = self.conn.transaction().map_err(database(doing))?;
        let linked: Vec<(String, String)> = {
            let mut statement = tx
                .prepare(
                    "SELECT t.task_id, r.state FROM project_task_pulls t
                     JOIN project_rows r ON r.id = t.task_id
                     WHERE r.manifest_id = ?1 AND t.pull = ?2 AND t.asked = 0 ORDER BY t.task_id",
                )
                .map_err(database(doing))?;
            let linked = statement
                .query_map(params![manifest_id, number as i64], |row| Ok((row.get(0)?, row.get(1)?)))
                .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
                .map_err(database(doing))?;
            linked
        };
        let mut moved = Vec::new();
        for (task, state) in linked {
            tx.execute("UPDATE project_task_pulls SET asked = 1 WHERE task_id = ?1 AND pull = ?2", params![task, number as i64])
                .map_err(database(doing))?;
            if state != "open" && state != "held" {
                continue;
            }
            let holder: String = tx
                .query_row("SELECT held_by FROM project_rows WHERE id = ?1", params![task], |row| row.get(0))
                .map_err(database(doing))?;
            tx.execute("UPDATE project_rows SET state = 'asked', asked_at = ?2 WHERE id = ?1", params![task, at])
                .map_err(database(doing))?;
            moved.push(holder);
        }
        tx.commit().map_err(database(doing))?;
        Ok(moved)
    }

    /// A task `held` by a Job, and its subtasks held by the same one, let go: each is `open` again. How many.
    pub fn free_held_project_task(&mut self, id: &str) -> Result<usize, WriteError> {
        self.conn
            .execute(
                "UPDATE project_rows SET state = 'open', held_by = '', asked_at = ''
                 WHERE state = 'held' AND (id = ?1 OR parent = ?1)
                   AND held_by = (SELECT held_by FROM project_rows WHERE id = ?1 AND state = 'held')",
                params![id],
            )
            .map_err(database("letting go of a project task"))
    }

    fn task_issue(
        &mut self,
        id: &str,
        doing: &'static str,
        write: impl FnOnce(&rusqlite::Transaction<'_>) -> rusqlite::Result<usize>,
    ) -> Result<bool, WriteError> {
        let tx = self.conn.transaction().map_err(database(doing))?;
        let task: i64 = tx
            .query_row("SELECT COUNT(*) FROM project_rows WHERE id = ?1 AND kind = 'task'", params![id], |row| row.get(0))
            .map_err(database(doing))?;
        if task == 0 {
            return Ok(false);
        }
        write(&tx).map_err(database(doing))?;
        tx.commit().map_err(database(doing))?;
        Ok(true)
    }
}
