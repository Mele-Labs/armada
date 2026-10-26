//! A repository's projects, kept in Fleet's store on this Mac. `docs/practices/protocol/projects.md`.
//!
//! **An issue row keeps no state.** Whether it is open, and its title, are read from the tracker
//! when the rows are: the open list Fleet holds for the Issues tab first, and an issue not on it read
//! by its reference and held [`HELD_FOR`], as that list is. A tracker that cannot answer leaves both
//! absent and refuses nothing, so a project is always listed.
//!
//! **Every write answers the repository's projects whole** and publishes them as `projects.changed`.

use std::collections::BTreeMap;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, NotTracked, Vcs, WorkProduct};
use api::{Projects, Refusal};
use core_model::Timestamp;
use ipc::{
    IssueReference, IssueState, LinkProjectIssue, ManifestId, MoveProjectRow, NameProject, Project, ProjectRow,
    ProjectHolder, ProjectTask, ProjectTaskState, TaskIssue, TickProjectTask, WireError,
};
use store::{KeptHolder, KeptProject, KeptProjectRow, KeptRowItem, KeptTaskState, Store};

use crate::clock::elapsed;
use crate::daemon::Fleet;
use crate::tracking::HELD_FOR;

/// The repository has no project by that id. A 404.
const NO_SUCH_PROJECT: &str = "fleet.no_such_project";
/// The project has no row by that id. A 404.
const NO_SUCH_ROW: &str = "fleet.no_such_project_row";
/// A project's name or a task's title is blank. A 422.
const BLANK: &str = "fleet.project_blank";
/// What was linked is not an issue's reference. A 422.
const NOT_AN_ISSUE: &str = "fleet.project_issue_unreadable";
/// A task's act on an issue row, whose state and title are its tracker's. A 422.
const NOT_A_TASK: &str = "fleet.project_row_not_a_task";
/// A subtask of a subtask, of a task with subtasks, or of itself. A 422.
const TOO_DEEP: &str = "fleet.project_subtask_too_deep";

/// What the tracker said of one issue: open or closed, and its title.
type Said = (IssueState, String);

/// Issues read one by one because they were not on the open list, by repository and reference.
/// **In memory**, for [`crate::tracking::Tracking`]'s reason. `None` is an issue the tracker has none of.
#[derive(Default)]
pub(crate) struct IssueReads(Mutex<BTreeMap<(String, String), (Timestamp, Option<Said>)>>);

impl IssueReads {
    fn held(&self, manifest: &str, reference: &str, now: &Timestamp) -> Option<Option<Said>> {
        let held = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        held.get(&(manifest.to_string(), reference.to_string()))
            .filter(|(at, _)| elapsed(at, now) < HELD_FOR)
            .map(|(_, said)| said.clone())
    }

    fn keep(&self, manifest: &str, reference: &str, at: Timestamp, said: Option<Said>) {
        let mut held = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        held.insert((manifest.to_string(), reference.to_string()), (at, said));
    }
}

/// A task's state on the wire, and the Job holding it where one does.
fn task_state(state: KeptTaskState) -> (ProjectTaskState, Option<ProjectHolder>) {
    let holder = |holder: KeptHolder| ProjectHolder { session_id: holder.session, title: holder.title };
    match state {
        KeptTaskState::Open => (ProjectTaskState::Open, None),
        KeptTaskState::Done => (ProjectTaskState::Done, None),
        KeptTaskState::Held(by) => (ProjectTaskState::Held, Some(holder(by))),
        KeptTaskState::Asked(by) => (ProjectTaskState::Asked, Some(holder(by))),
    }
}

fn row_on_the_wire(row: KeptProjectRow, said: &BTreeMap<String, Said>) -> ProjectRow {
    match row.item {
        KeptRowItem::Task { title, notes, state, parent, issues } => {
            let (state, held_by) = task_state(state);
            let issues = issues
                .into_iter()
                .map(|issue| {
                    let read = said.get(&issue);
                    TaskIssue { title: read.map(|(_, title)| title.clone()), state: read.map(|(state, _)| *state), issue }
                })
                .collect();
            ProjectRow::Task { id: row.id, title, notes, state, held_by, parent, issues }
        }
        KeptRowItem::Issue { reference } => {
            let read = said.get(&reference);
            ProjectRow::Issue {
                id: row.id,
                title: read.map(|(_, title)| title.clone()),
                state: read.map(|(state, _)| *state),
                issue: reference,
            }
        }
    }
}

fn issue_references(kept: &[KeptProject]) -> Vec<String> {
    let mut references: Vec<String> = kept
        .iter()
        .flat_map(|project| &project.rows)
        .flat_map(|row| match &row.item {
            KeptRowItem::Issue { reference } => vec![reference.clone()],
            KeptRowItem::Task { issues, .. } => issues.clone(),
        })
        .collect();
    references.sort_unstable();
    references.dedup();
    references
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    pub(crate) fn project_refusal(&self, code: &'static str, said: impl Into<String>) -> WireError {
        WireError::raised(code, said, self.run_id())
    }

    fn blank(&self, said: &str) -> Refusal {
        Refusal::Unacceptable(self.project_refusal(BLANK, said))
    }

    pub(crate) fn project_in<'k>(&self, kept: &'k [KeptProject], project: &str) -> Result<&'k KeptProject, Refusal> {
        kept.iter().find(|one| one.id == project).ok_or_else(|| {
            Refusal::NotFound(self.project_refusal(NO_SUCH_PROJECT, format!("the repository has no project {project}")))
        })
    }

    pub(crate) fn row_in<'k>(&self, kept: &'k [KeptProject], project: &str, row: &str) -> Result<&'k KeptProjectRow, Refusal> {
        self.project_in(kept, project)?.rows.iter().find(|one| one.id == row).ok_or_else(|| {
            Refusal::NotFound(self.project_refusal(NO_SUCH_ROW, format!("the project has no row {row}")))
        })
    }

    /// `row` of `project` where it is a task.
    fn task_in(&self, kept: &[KeptProject], project: &str, row: &str) -> Result<(), Refusal> {
        match self.row_in(kept, project, row)?.item {
            KeptRowItem::Task { .. } => Ok(()),
            KeptRowItem::Issue { .. } => Err(self.not_a_task(row)),
        }
    }

    /// `row` of `project` as a task that may take subtasks: a top-level one.
    fn top_level_task_in(&self, kept: &[KeptProject], project: &str, row: &str) -> Result<(), Refusal> {
        match &self.row_in(kept, project, row)?.item {
            KeptRowItem::Issue { .. } => Err(self.not_a_task(row)),
            KeptRowItem::Task { parent: Some(_), .. } => {
                Err(Refusal::Unacceptable(self.project_refusal(TOO_DEEP, format!("row {row} is a subtask, and a subtask has none"))))
            }
            KeptRowItem::Task { .. } => Ok(()),
        }
    }

    fn not_a_task(&self, row: &str) -> Refusal {
        Refusal::Unacceptable(
            self.project_refusal(NOT_A_TASK, format!("row {row} is a linked issue, whose state and title are its tracker's")),
        )
    }

    /// Moving `row` under the task `parent`: both written tasks, the parent top-level, and the row a
    /// task with no subtasks of its own, which is not the parent itself.
    fn moved_under(&self, kept: &[KeptProject], project: &str, row: &str, parent: &str) -> Result<(), Refusal> {
        let moving = self.row_in(kept, project, row)?;
        if matches!(moving.item, KeptRowItem::Issue { .. }) {
            return Err(self.not_a_task(row));
        }
        self.top_level_task_in(kept, project, parent)?;
        let has_subtasks = self
            .project_in(kept, project)?
            .rows
            .iter()
            .any(|one| matches!(&one.item, KeptRowItem::Task { parent: Some(of), .. } if of == row));
        if parent == row || has_subtasks {
            return Err(Refusal::Unacceptable(self.project_refusal(
                TOO_DEEP,
                format!("row {row} would be a subtask of {parent}, and a subtask has none"),
            )));
        }
        Ok(())
    }

    fn unreadable_issue(&self, written: &str) -> Refusal {
        Refusal::Unacceptable(self.project_refusal(
            NOT_AN_ISSUE,
            format!("`{}` is not an issue: a number such as 7, a key such as ABC-12, or an id such as armada-3fz", written.trim()),
        ))
    }

    fn minted(&self) -> String {
        self.mint().ulid().as_str().to_lowercase()
    }

    /// What the tracker says of each issue linked in `kept`, by reference. **Never refuses**: one
    /// it could not say is left out, and the row draws without a state.
    async fn issues_said(&self, manifest_id: &ManifestId, kept: &[KeptProject]) -> BTreeMap<String, Said> {
        let references = issue_references(kept);
        let mut said = BTreeMap::new();
        if references.is_empty() {
            return said;
        }
        let Ok(open) = self.open_issues(manifest_id).await else {
            return said;
        };
        for issue in &open {
            if let Some(reference) = IssueReference::read(&issue.reference) {
                let state = if issue.closed { IssueState::Closed } else { IssueState::Open };
                said.insert(reference.as_str().to_string(), (state, issue.title.as_written().to_string()));
            }
        }
        let Ok(served) = self.served_named(Some(manifest_id)) else {
            return said;
        };
        let now = self.now();
        let unlisted: Vec<String> = references.into_iter().filter(|one| !said.contains_key(one)).collect();
        for reference in unlisted {
            if let Some(held) = self.tracking().reads.held(manifest_id.as_str(), &reference, &now) {
                said.extend(held.map(|one| (reference.clone(), one)));
                continue;
            }
            let (tracker, root, asked) = (self.tracker_of(&served), served.root().to_string(), reference.clone());
            let read = tokio::task::spawn_blocking(move || tracker.read_issue(&root, &asked))
                .await
                .unwrap_or_else(|panicked| Err(NotTracked::of("reading the issue", panicked.to_string())));
            let Ok(read) = read else {
                continue;
            };
            let read = read.map(|issue| {
                let state = if issue.closed { IssueState::Closed } else { IssueState::Open };
                (state, issue.title.as_written().to_string())
            });
            self.tracking().reads.keep(manifest_id.as_str(), &reference, now.clone(), read.clone());
            said.extend(read.map(|one| (reference, one)));
        }
        said
    }

    pub(crate) async fn projects_on_the_wire(&self, manifest_id: ManifestId, kept: Vec<KeptProject>) -> ipc::Projects {
        let said = self.issues_said(&manifest_id, &kept).await;
        ipc::Projects {
            manifest_id,
            projects: kept
                .into_iter()
                .map(|project| Project {
                    id: project.id,
                    name: project.name,
                    rows: project.rows.into_iter().map(|row| row_on_the_wire(row, &said)).collect(),
                })
                .collect(),
        }
    }

    /// `write` against the store, given the repository's projects as they were; then the projects
    /// as they are, answered and published.
    async fn projects_written<F>(&self, manifest_id: ManifestId, write: F) -> Result<ipc::Projects, Refusal>
    where
        F: FnOnce(&mut Store, &[KeptProject]) -> Result<(), Refusal> + Send,
    {
        self.served_named(Some(&manifest_id))?;
        let kept = {
            let mut store = self.store().lock().await;
            let before = store.projects(manifest_id.as_str()).map_err(|why| self.ledger_fault(why))?;
            write(&mut store, &before)?;
            store.projects(manifest_id.as_str()).map_err(|why| self.ledger_fault(why))?
        };
        let projects = self.projects_on_the_wire(manifest_id, kept).await;
        self.publish(ipc::Event::ProjectsChanged(projects.clone()));
        Ok(projects)
    }
}

impl<H, V, W> Projects for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn list_projects(&self, manifest_id: ManifestId) -> Result<ipc::Projects, Refusal> {
        self.served_named(Some(&manifest_id))?;
        let kept = self.store().lock().await.projects(manifest_id.as_str()).map_err(|why| self.ledger_fault(why))?;
        Ok(self.projects_on_the_wire(manifest_id, kept).await)
    }

    async fn add_project(&self, manifest_id: ManifestId, name: NameProject) -> Result<ipc::Projects, Refusal> {
        let name = name.name.trim().to_string();
        if name.is_empty() {
            return Err(self.blank("a project needs a name"));
        }
        let (id, at, repository) = (self.minted(), self.now(), manifest_id.as_str().to_string());
        self.projects_written(manifest_id, |store, _| {
            store.add_project(&repository, &id, &name, at.as_str()).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn rename_project(
        &self,
        manifest_id: ManifestId,
        project: String,
        name: NameProject,
    ) -> Result<ipc::Projects, Refusal> {
        let name = name.name.trim().to_string();
        let repository = manifest_id.as_str().to_string();
        self.projects_written(manifest_id, |store, kept| {
            self.project_in(kept, &project)?;
            if name.is_empty() {
                return Err(self.blank("a project needs a name"));
            }
            store.rename_project(&repository, &project, &name).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn delete_project(&self, manifest_id: ManifestId, project: String) -> Result<ipc::Projects, Refusal> {
        let repository = manifest_id.as_str().to_string();
        self.projects_written(manifest_id, |store, kept| {
            self.project_in(kept, &project)?;
            store.delete_project(&repository, &project).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn add_project_task(
        &self,
        manifest_id: ManifestId,
        project: String,
        task: ProjectTask,
    ) -> Result<ipc::Projects, Refusal> {
        let id = self.minted();
        self.projects_written(manifest_id, |store, kept| {
            self.project_in(kept, &project)?;
            let title = task.title.trim();
            if title.is_empty() {
                return Err(self.blank("a task needs a title"));
            }
            let item = KeptRowItem::Task {
                title: title.to_string(),
                notes: task.notes.trim().to_string(),
                state: KeptTaskState::Open,
                parent: None,
                issues: Vec::new(),
            };
            store.add_project_row(&project, &KeptProjectRow { id, item }).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn add_project_subtask(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        task: ProjectTask,
    ) -> Result<ipc::Projects, Refusal> {
        let id = self.minted();
        self.projects_written(manifest_id, |store, kept| {
            self.top_level_task_in(kept, &project, &row)?;
            let title = task.title.trim();
            if title.is_empty() {
                return Err(self.blank("a task needs a title"));
            }
            let item = KeptRowItem::Task {
                title: title.to_string(),
                notes: task.notes.trim().to_string(),
                state: KeptTaskState::Open,
                parent: Some(row.clone()),
                issues: Vec::new(),
            };
            store.add_project_row(&project, &KeptProjectRow { id, item }).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn link_task_issue(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        link: LinkProjectIssue,
    ) -> Result<ipc::Projects, Refusal> {
        self.projects_written(manifest_id, |store, kept| {
            self.task_in(kept, &project, &row)?;
            let issue = IssueReference::read(&link.issue).ok_or_else(|| self.unreadable_issue(&link.issue))?;
            store.link_task_issue(&project, &row, issue.as_str()).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn unlink_task_issue(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        link: LinkProjectIssue,
    ) -> Result<ipc::Projects, Refusal> {
        self.projects_written(manifest_id, |store, kept| {
            self.task_in(kept, &project, &row)?;
            let issue = IssueReference::read(&link.issue).ok_or_else(|| self.unreadable_issue(&link.issue))?;
            store.unlink_task_issue(&project, &row, issue.as_str()).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn edit_project_task(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        task: ProjectTask,
    ) -> Result<ipc::Projects, Refusal> {
        self.projects_written(manifest_id, |store, kept| {
            self.task_in(kept, &project, &row)?;
            let title = task.title.trim();
            if title.is_empty() {
                return Err(self.blank("a task needs a title"));
            }
            store
                .edit_project_task(&project, &row, title, task.notes.trim())
                .map(drop)
                .map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn link_project_issue(
        &self,
        manifest_id: ManifestId,
        project: String,
        link: LinkProjectIssue,
    ) -> Result<ipc::Projects, Refusal> {
        let id = self.minted();
        self.projects_written(manifest_id, |store, kept| {
            self.project_in(kept, &project)?;
            let issue = IssueReference::read(&link.issue).ok_or_else(|| self.unreadable_issue(&link.issue))?;
            let item = KeptRowItem::Issue { reference: issue.as_str().to_string() };
            store.add_project_row(&project, &KeptProjectRow { id, item }).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn remove_project_row(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
    ) -> Result<ipc::Projects, Refusal> {
        self.projects_written(manifest_id, |store, kept| {
            self.row_in(kept, &project, &row)?;
            store.remove_project_row(&project, &row).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn move_project_row(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        to: MoveProjectRow,
    ) -> Result<ipc::Projects, Refusal> {
        self.projects_written(manifest_id, |store, kept| {
            self.row_in(kept, &project, &row)?;
            if let Some(parent) = to.parent.as_deref().filter(|parent| !parent.is_empty()) {
                self.moved_under(kept, &project, &row, parent)?;
            }
            store
                .move_project_row(&project, &row, to.position as usize, to.parent.as_deref())
                .map(drop)
                .map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn tick_project_task(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        tick: TickProjectTask,
    ) -> Result<ipc::Projects, Refusal> {
        let state = if tick.done { KeptTaskState::Done } else { KeptTaskState::Open };
        self.projects_written(manifest_id, |store, kept| {
            self.task_in(kept, &project, &row)?;
            store.set_project_task_state(&project, &row, state).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }
}
