//! His projects, kept in Fleet's store on this Mac, across repositories, and the tasks in none. `docs/practices/protocol/projects.md`.
//!
//! **An issue row keeps no state.** Whether it is open, and its title, are read from the tracker
//! when the rows are: the open list Fleet holds for the Issues tab first, and an issue not on it read
//! by its reference and held [`HELD_FOR`], as that list is. A tracker that cannot answer leaves both
//! absent and refuses nothing, so a project is always listed. **A pull request linked to a task is read the
//! same way**, from the forge by its number and held [`HELD_FOR`], and a forge that is silent leaves both absent.
//!
//! **Every write answers his projects and the loose tasks whole** and publishes them as `projects.changed`.

use std::collections::BTreeMap;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, NotTracked, PullRequestStanding, Vcs, WorkProduct};
use api::{Projects, Refusal};
use core_model::Timestamp;
use ipc::{
    IssueReference, IssueState, LinkProjectIssue, LinkTaskJob, LinkTaskPull, ManifestId, MoveProjectRow, MoveProjectTaskRepository,
    NameProject, NewProjectIssue, NewProjectTask, Project, ProjectRow, ProjectHolder, ProjectTask, ProjectTaskState, TaskIssue,
    TaskPull, TaskPullState, TickProjectTask, WireError,
};
use store::{KeptHolder, KeptProjectRow, KeptProjects, KeptRowItem, KeptTaskState, Store, LOOSE_PROJECT};

use crate::clock::elapsed;
use crate::daemon::Fleet;
use crate::project_holds::subtasks_of;
use crate::tracking::HELD_FOR;

/// He has no project by that id. A 404.
const NO_SUCH_PROJECT: &str = "fleet.no_such_project";
/// There is no row by that id. A 404.
const NO_SUCH_ROW: &str = "fleet.no_such_project_row";
/// A project's name or a task's title is blank. A 422.
const BLANK: &str = "fleet.project_blank";
/// What was linked is not an issue's reference. A 422.
const NOT_AN_ISSUE: &str = "fleet.project_issue_unreadable";
/// A task's act on an issue row, whose state and title are its tracker's. A 422.
const NOT_A_TASK: &str = "fleet.project_row_not_a_task";
/// A subtask of a subtask, of a task with subtasks, or of itself. A 422.
const TOO_DEEP: &str = "fleet.project_subtask_too_deep";
/// What was linked is not a pull request's number. A 422.
const NOT_A_PULL: &str = "fleet.project_pull_unreadable";

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

/// What the forge said of one pull request: where it stands, and its title.
type PullSaid = (TaskPullState, String);

/// Pull requests linked in a project and read one by one, by repository and number. **In memory**, as
/// [`IssueReads`] is. A forge that said nothing is not kept: asking again is reasonable.
#[derive(Default)]
pub(crate) struct PullReads(Mutex<BTreeMap<(String, u64), (Timestamp, PullSaid)>>);

impl PullReads {
    fn held(&self, manifest: &str, number: u64, now: &Timestamp) -> Option<PullSaid> {
        let held = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        held.get(&(manifest.to_string(), number)).filter(|(at, _)| elapsed(at, now) < HELD_FOR).map(|(_, said)| said.clone())
    }

    pub(crate) fn keep(&self, manifest: &str, number: u64, at: Timestamp, said: PullSaid) {
        let mut held = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        held.insert((manifest.to_string(), number), (at, said));
    }
}

/// A task's state on the wire, and the Job holding it where one does.
fn task_state(state: KeptTaskState) -> (ProjectTaskState, Option<ProjectHolder>) {
    let holder = |holder: KeptHolder| ProjectHolder { session_id: holder.session, title: holder.title };
    match state {
        KeptTaskState::Open => (ProjectTaskState::Open, None),
        KeptTaskState::Done => (ProjectTaskState::Done, None),
        KeptTaskState::Held(by) => (ProjectTaskState::Held, Some(holder(by))),
        KeptTaskState::Asked(by) => (ProjectTaskState::Asked, by.map(holder)),
    }
}

/// What the trackers said of each issue, and the forges of each pull request, by repository.
type SaidBy = BTreeMap<String, BTreeMap<String, Said>>;
type PulledBy = BTreeMap<String, BTreeMap<u64, PullSaid>>;

/// A row on the wire. A task's links are read from the repository each was linked in, which is not
/// its task's once the task has moved.
fn row_on_the_wire(row: KeptProjectRow, said_by: &SaidBy, pulled_by: &PulledBy) -> ProjectRow {
    let manifest_id = ManifestId::carried(row.manifest_id.clone());
    match row.item {
        KeptRowItem::Task { title, notes, state, parent, issues, pulls } => {
            let (state, held_by) = task_state(state);
            // A link kept before links named their repository reads as its task's.
            let linked_in = |linked: &str| Some(ManifestId::carried(if linked.is_empty() { row.manifest_id.clone() } else { linked.to_string() }));
            let issues = issues
                .into_iter()
                .map(|link| {
                    let read = said_by.get(&link.manifest_id).and_then(|said| said.get(&link.reference));
                    TaskIssue {
                        manifest_id: linked_in(&link.manifest_id),
                        title: read.map(|(_, title)| title.clone()),
                        state: read.map(|(state, _)| *state),
                        issue: link.reference,
                    }
                })
                .collect();
            let pulls = pulls
                .into_iter()
                .map(|link| {
                    let read = pulled_by.get(&link.manifest_id).and_then(|pulled| pulled.get(&link.number));
                    let number = link.number;
                    TaskPull {
                        number,
                        manifest_id: linked_in(&link.manifest_id),
                        title: read.map(|(_, title)| title.clone()),
                        state: read.map(|(state, _)| *state),
                    }
                })
                .collect();
            ProjectRow::Task { id: row.id, manifest_id, title, notes, state, held_by, parent, issues, pulls }
        }
        KeptRowItem::Issue { reference } => {
            let read = said_by.get(&row.manifest_id).and_then(|said| said.get(&reference));
            ProjectRow::Issue {
                id: row.id,
                manifest_id,
                title: read.map(|(_, title)| title.clone()),
                state: read.map(|(state, _)| *state),
                issue: reference,
            }
        }
    }
}

fn rows_of(kept: &KeptProjects) -> impl Iterator<Item = &KeptProjectRow> {
    kept.projects.iter().flat_map(|project| &project.rows).chain(&kept.loose)
}

/// The references of the issues in `kept`, by the repository each issue row is in or each link was made in.
fn issue_references(kept: &KeptProjects) -> BTreeMap<String, Vec<String>> {
    let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows_of(kept) {
        match &row.item {
            KeptRowItem::Issue { reference } => by.entry(row.manifest_id.clone()).or_default().push(reference.clone()),
            KeptRowItem::Task { issues, .. } => {
                for link in issues {
                    by.entry(link.manifest_id.clone()).or_default().push(link.reference.clone());
                }
            }
        }
    }
    for references in by.values_mut() {
        references.sort_unstable();
        references.dedup();
    }
    by
}

/// The numbers of the pull requests linked in `kept`, by the repository each was linked in.
fn pull_numbers(kept: &KeptProjects) -> BTreeMap<String, Vec<u64>> {
    let mut by: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for row in rows_of(kept) {
        if let KeptRowItem::Task { pulls, .. } = &row.item {
            for link in pulls {
                by.entry(link.manifest_id.clone()).or_default().push(link.number);
            }
        }
    }
    for numbers in by.values_mut() {
        numbers.sort_unstable();
        numbers.dedup();
    }
    by
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

    pub(crate) fn project_in<'k>(&self, kept: &'k KeptProjects, project: &str) -> Result<&'k store::KeptProject, Refusal> {
        kept.projects.iter().find(|one| one.id == project).ok_or_else(|| {
            Refusal::NotFound(self.project_refusal(NO_SUCH_PROJECT, format!("there is no project {project}")))
        })
    }

    /// The project a write puts a row in: the one named, [`LOOSE_PROJECT`] where none is.
    fn project_named(&self, kept: &KeptProjects, project: Option<&str>) -> Result<String, Refusal> {
        match project.filter(|one| !one.is_empty()) {
            None => Ok(LOOSE_PROJECT.to_string()),
            Some(one) => self.project_in(kept, one).map(|found| found.id.clone()),
        }
    }

    /// Row `row` wherever it stands, and the project it is in ([`LOOSE_PROJECT`] for none).
    pub(crate) fn row_in<'k>(&self, kept: &'k KeptProjects, row: &str) -> Result<(&'k str, &'k KeptProjectRow), Refusal> {
        let found = kept
            .projects
            .iter()
            .flat_map(|project| project.rows.iter().map(move |one| (project.id.as_str(), one)))
            .chain(kept.loose.iter().map(|one| (LOOSE_PROJECT, one)))
            .find(|(_, one)| one.id == row);
        found.ok_or_else(|| Refusal::NotFound(self.project_refusal(NO_SUCH_ROW, format!("there is no row {row}"))))
    }

    /// `row` where it is a task.
    fn task_in(&self, kept: &KeptProjects, row: &str) -> Result<(), Refusal> {
        match self.row_in(kept, row)?.1.item {
            KeptRowItem::Task { .. } => Ok(()),
            KeptRowItem::Issue { .. } => Err(self.not_a_task(row)),
        }
    }

    /// `row` as a task that may take subtasks: a top-level one. Where it stands.
    fn top_level_task_in<'k>(&self, kept: &'k KeptProjects, row: &str) -> Result<(&'k str, &'k KeptProjectRow), Refusal> {
        let found = self.row_in(kept, row)?;
        match &found.1.item {
            KeptRowItem::Issue { .. } => Err(self.not_a_task(row)),
            KeptRowItem::Task { parent: Some(_), .. } => {
                Err(Refusal::Unacceptable(self.project_refusal(TOO_DEEP, format!("row {row} is a subtask, and a subtask has none"))))
            }
            KeptRowItem::Task { .. } => Ok(found),
        }
    }

    fn not_a_task(&self, row: &str) -> Refusal {
        Refusal::Unacceptable(
            self.project_refusal(NOT_A_TASK, format!("row {row} is a linked issue, whose state and title are its tracker's")),
        )
    }

    /// Moving `row` under the task `parent` of the project `into`: both written tasks, the parent
    /// top-level and in that project, and the row a task with no subtasks of its own, which is not the parent itself.
    fn moved_under(&self, kept: &KeptProjects, row: &str, parent: &str, into: &str) -> Result<(), Refusal> {
        let (_, moving) = self.row_in(kept, row)?;
        if matches!(moving.item, KeptRowItem::Issue { .. }) {
            return Err(self.not_a_task(row));
        }
        let (home, _) = self.top_level_task_in(kept, parent)?;
        if home != into {
            return Err(Refusal::NotFound(self.project_refusal(NO_SUCH_ROW, format!("there is no row {parent} in that project"))));
        }
        let has_subtasks = subtasks_of(kept, row).next().is_some();
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

    fn unreadable_pull(&self, number: u64) -> Refusal {
        Refusal::Unacceptable(self.project_refusal(NOT_A_PULL, format!("`{number}` is not a pull request's number")))
    }

    fn minted(&self) -> String {
        self.mint().ulid().as_str().to_lowercase()
    }

    /// What the tracker of `manifest_id` says of each of `references`, by reference. **Never refuses**: one
    /// it could not say is left out, and the row draws without a state.
    async fn issues_said(&self, manifest_id: &ManifestId, references: Vec<String>) -> BTreeMap<String, Said> {
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

    /// What the forge of `manifest_id` says of each of `numbers`, by number. **Never refuses**: one it
    /// could not say is left out, and the task draws it without a title or a state.
    async fn pulls_said(&self, manifest_id: &ManifestId, numbers: Vec<u64>) -> BTreeMap<u64, PullSaid> {
        let mut said = BTreeMap::new();
        if numbers.is_empty() {
            return said;
        }
        let Ok(served) = self.served_named(Some(manifest_id)) else {
            return said;
        };
        let now = self.now();
        for number in numbers {
            if let Some(held) = self.tracking().pulls.held(manifest_id.as_str(), number, &now) {
                said.insert(number, held);
                continue;
            }
            let (vcs, root) = (std::sync::Arc::clone(self.vcs()), served.root().to_string());
            let facts = tokio::task::spawn_blocking(move || vcs.pull_request_facts(&root, &number.to_string())).await.ok().flatten();
            let Some(facts) = facts else {
                continue;
            };
            let state = match facts.standing {
                PullRequestStanding::Draft | PullRequestStanding::Open => TaskPullState::Open,
                PullRequestStanding::Merged => TaskPullState::Merged,
                PullRequestStanding::Closed => TaskPullState::Closed,
            };
            self.tracking().pulls.keep(manifest_id.as_str(), number, now.clone(), (state, facts.title.clone()));
            said.insert(number, (state, facts.title));
        }
        said
    }

    pub(crate) async fn projects_on_the_wire(&self, kept: KeptProjects) -> ipc::Projects {
        let (mut said, mut pulled): (SaidBy, PulledBy) = (BTreeMap::new(), BTreeMap::new());
        for (manifest, references) in issue_references(&kept) {
            let read = self.issues_said(&ManifestId::carried(manifest.clone()), references).await;
            said.insert(manifest, read);
        }
        for (manifest, numbers) in pull_numbers(&kept) {
            let read = self.pulls_said(&ManifestId::carried(manifest.clone()), numbers).await;
            pulled.insert(manifest, read);
        }
        let wire = |rows: Vec<KeptProjectRow>| rows.into_iter().map(|row| row_on_the_wire(row, &said, &pulled)).collect();
        ipc::Projects {
            projects: kept.projects.into_iter().map(|project| Project { id: project.id, name: project.name, rows: wire(project.rows) }).collect(),
            loose: wire(kept.loose),
        }
    }

    /// `write` against the store, given his projects as they were; then the projects
    /// as they are, answered and published.
    async fn projects_written<F>(&self, write: F) -> Result<ipc::Projects, Refusal>
    where
        F: FnOnce(&mut Store, &KeptProjects) -> Result<(), Refusal> + Send,
    {
        let kept = {
            let mut store = self.store().lock().await;
            let before = store.projects().map_err(|why| self.ledger_fault(why))?;
            write(&mut store, &before)?;
            store.projects().map_err(|why| self.ledger_fault(why))?
        };
        let projects = self.projects_on_the_wire(kept).await;
        self.publish(ipc::Event::ProjectsChanged(projects.clone()));
        Ok(projects)
    }

    /// Move a top-level task and its subtasks to the repository `manifest_id`, which Fleet serves. **The
    /// issues and pull requests linked to them stay the repository's they were linked in**, and a Job
    /// holding the task keeps it. Moving it to the repository it is in changes nothing.
    pub async fn move_project_task_repository(&self, row: String, manifest_id: ManifestId) -> Result<ipc::Projects, Refusal> {
        self.served_named(Some(&manifest_id))?;
        self.projects_written(|store, kept| {
            self.top_level_task_in(kept, &row)?;
            store.move_project_task_repository(&row, manifest_id.as_str()).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
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
    async fn list_projects(&self) -> Result<ipc::Projects, Refusal> {
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        Ok(self.projects_on_the_wire(kept).await)
    }

    async fn add_project(&self, name: NameProject) -> Result<ipc::Projects, Refusal> {
        let name = name.name.trim().to_string();
        if name.is_empty() {
            return Err(self.blank("a project needs a name"));
        }
        let (id, at) = (self.minted(), self.now());
        self.projects_written(|store, _| store.add_project(&id, &name, at.as_str()).map_err(|why| self.ledger_fault(why))).await
    }

    async fn rename_project(&self, project: String, name: NameProject) -> Result<ipc::Projects, Refusal> {
        let name = name.name.trim().to_string();
        self.projects_written(|store, kept| {
            self.project_in(kept, &project)?;
            if name.is_empty() {
                return Err(self.blank("a project needs a name"));
            }
            store.rename_project(&project, &name).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn delete_project(&self, project: String) -> Result<ipc::Projects, Refusal> {
        self.projects_written(|store, kept| {
            self.project_in(kept, &project)?;
            store.delete_project(&project).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn add_project_task(&self, task: NewProjectTask) -> Result<ipc::Projects, Refusal> {
        self.served_named(Some(&task.manifest_id))?;
        let id = self.minted();
        self.projects_written(|store, kept| {
            let project = self.project_named(kept, task.project.as_deref())?;
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
                pulls: Vec::new(),
            };
            let row = KeptProjectRow { id, manifest_id: task.manifest_id.as_str().to_string(), item };
            store.add_project_row(&project, &row).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn add_project_subtask(&self, row: String, task: ProjectTask) -> Result<ipc::Projects, Refusal> {
        let id = self.minted();
        self.projects_written(|store, kept| {
            let (project, parent) = self.top_level_task_in(kept, &row)?;
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
                pulls: Vec::new(),
            };
            let subtask = KeptProjectRow { id, manifest_id: parent.manifest_id.clone(), item };
            store.add_project_row(project, &subtask).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn link_task_issue(&self, row: String, link: LinkProjectIssue) -> Result<ipc::Projects, Refusal> {
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            let issue = IssueReference::read(&link.issue).ok_or_else(|| self.unreadable_issue(&link.issue))?;
            store.link_task_issue(&row, issue.as_str()).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn unlink_task_issue(&self, row: String, link: LinkProjectIssue) -> Result<ipc::Projects, Refusal> {
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            let issue = IssueReference::read(&link.issue).ok_or_else(|| self.unreadable_issue(&link.issue))?;
            store.unlink_task_issue(&row, issue.as_str()).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn link_task_job(&self, row: String, link: LinkTaskJob) -> Result<ipc::Projects, Refusal> {
        let session = link.session_id.as_str().to_string();
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            self.job_may_hold(store, &session)?;
            let mut tasks = vec![row.clone()];
            tasks.extend(subtasks_of(kept, &row));
            store.hold_project_tasks(&session, &tasks).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn unlink_task_job(&self, row: String) -> Result<ipc::Projects, Refusal> {
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            store.free_held_project_task(&row).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn link_task_pull(&self, row: String, link: LinkTaskPull) -> Result<ipc::Projects, Refusal> {
        let projects = self
            .projects_written(|store, kept| {
                self.task_in(kept, &row)?;
                if link.number == 0 {
                    return Err(self.unreadable_pull(link.number));
                }
                store.link_task_pull(&row, link.number).map(drop).map_err(|why| self.ledger_fault(why))
            })
            .await?;
        // One that merged before it was linked asks now, once, as the sweep would have.
        let merged = projects.projects.iter().flat_map(|one| &one.rows).chain(&projects.loose).find_map(|one| match one {
            ProjectRow::Task { id, manifest_id, pulls, .. } if *id == row => pulls
                .iter()
                .find(|pull| pull.number == link.number && pull.state == Some(TaskPullState::Merged))
                .map(|pull| (manifest_id.as_str().to_string(), pull)),
            _ => None,
        });
        match merged {
            Some((repository, pull))
                if self.tasks_asked_by_linked_pull(&repository, link.number, pull.title.as_deref().unwrap_or_default()).await =>
            {
                self.list_projects().await
            }
            _ => Ok(projects),
        }
    }

    async fn unlink_task_pull(&self, row: String, link: LinkTaskPull) -> Result<ipc::Projects, Refusal> {
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            store.unlink_task_pull(&row, link.number).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn edit_project_task(&self, row: String, task: ProjectTask) -> Result<ipc::Projects, Refusal> {
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            let title = task.title.trim();
            if title.is_empty() {
                return Err(self.blank("a task needs a title"));
            }
            store.edit_project_task(&row, title, task.notes.trim()).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn link_project_issue(&self, link: NewProjectIssue) -> Result<ipc::Projects, Refusal> {
        self.served_named(Some(&link.manifest_id))?;
        let id = self.minted();
        self.projects_written(|store, kept| {
            let project = self.project_named(kept, link.project.as_deref())?;
            let issue = IssueReference::read(&link.issue).ok_or_else(|| self.unreadable_issue(&link.issue))?;
            let item = KeptRowItem::Issue { reference: issue.as_str().to_string() };
            let row = KeptProjectRow { id, manifest_id: link.manifest_id.as_str().to_string(), item };
            store.add_project_row(&project, &row).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn remove_project_row(&self, row: String) -> Result<ipc::Projects, Refusal> {
        self.projects_written(|store, kept| {
            self.row_in(kept, &row)?;
            store.remove_project_row(&row).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn move_project_row(&self, row: String, to: MoveProjectRow) -> Result<ipc::Projects, Refusal> {
        self.projects_written(|store, kept| {
            let (was, _) = self.row_in(kept, &row)?;
            let into = match to.project.as_deref() {
                Some(project) => self.project_named(kept, Some(project))?,
                None => was.to_string(),
            };
            if let Some(parent) = to.parent.as_deref().filter(|parent| !parent.is_empty()) {
                self.moved_under(kept, &row, parent, &into)?;
            }
            store
                .move_project_row(&row, to.project.as_ref().map(|_| into.as_str()), to.position as usize, to.parent.as_deref())
                .map(drop)
                .map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn move_project_task_repository(&self, row: String, to: MoveProjectTaskRepository) -> Result<ipc::Projects, Refusal> {
        Fleet::move_project_task_repository(self, row, to.manifest_id).await
    }

    async fn tick_project_task(&self, row: String, tick: TickProjectTask) -> Result<ipc::Projects, Refusal> {
        let state = if tick.done { KeptTaskState::Done } else { KeptTaskState::Open };
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            store.set_project_task_state(&row, state).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }
}
