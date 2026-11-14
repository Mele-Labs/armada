//! His projects, kept in Fleet's store on this Mac, across repositories, and the tasks in none. `docs/practices/protocol/projects.md`.
//!
//! **An issue row keeps no state.** Whether it is open, and its title, are read from the tracker
//! when the rows are: the open list Fleet holds for the Issues tab first, and an issue not on it read
//! by its reference and held [`HELD_FOR`], as that list is. A tracker that cannot answer leaves both
//! absent and refuses nothing, so a project is always listed. **A pull request linked to a task is read the
//! same way**, from the forge by its number and held [`HELD_FOR`], and a forge that is silent leaves both absent.
//!
//! **Every write answers his projects and the loose tasks whole** and publishes them as `projects.changed`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, NotTracked, PullRequestStanding, Vcs, WorkProduct};
use api::{Projects, Refusal};
use core_model::{Member, ScopeId, Timestamp, Ulid};
use ipc::{
    EditProject, EditedTaskFields, IssueReference, IssueState, LinkProjectIssue, LinkTaskJob, LinkTaskPull, ManifestId, MoveProjectRow,
    MoveProjectTaskRepository, NewProject, NewProjectIssue, NewProjectTask, Project, ProjectHolder, ProjectLeftOut, ProjectMember,
    ProjectAct, ProjectPlace, ProjectRow, ProjectTask, ProjectTaskState, RunPlace, TaskIssue, TaskPull, TaskPullState, TickProjectTask, WireError,
};
use store::{Edited, KeptHolder, KeptPlace, KeptProjectRow, KeptProjects, KeptRowItem, KeptTaskState, Store, LOOSE_PROJECT};

use crate::clock::elapsed;
use crate::daemon::Fleet;
use crate::project_holds::subtasks_of;
use crate::run_memory::{cloud, place_of};
use crate::tracking::{HELD_FOR, NO_SUCH_ISSUE};

/// He has no project by that id. A 404.
pub(crate) const NO_SUCH_PROJECT: &str = "fleet.no_such_project";
/// There is no row by that id. A 404.
pub(crate) const NO_SUCH_ROW: &str = "fleet.no_such_project_row";
/// A project's name or a task's title is blank. A 422.
pub(crate) const BLANK: &str = "fleet.project_blank";
/// What was linked is not an issue's reference. A 422.
const NOT_AN_ISSUE: &str = "fleet.project_issue_unreadable";
/// A task's act on an issue row, whose state and title are its tracker's. A 422.
pub(crate) const NOT_A_TASK: &str = "fleet.project_row_not_a_task";
/// A subtask of a subtask, of a task with subtasks, or of itself. A 422.
pub(crate) const TOO_DEEP: &str = "fleet.project_subtask_too_deep";
/// What was linked is not a pull request's number. A 422.
const NOT_A_PULL: &str = "fleet.project_pull_unreadable";

/// A row written into a project, a project of his and not the loose tasks: the project's id and the repositories of the
/// row and its subtasks, as Fleet names them.
type Put = (String, Vec<String>);

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

/// What the trackers said of each issue, and the forges of each pull request, by repository. **An issue read through
/// a project's own tracker is under that project's key**, since the repository's tracker would say something else.
type SaidBy = BTreeMap<String, BTreeMap<String, Said>>;

/// The key an issue read is kept under: the repository's, or the project's and then the repository's where the
/// project names a tracker.
fn said_key(project: Option<&str>, manifest: &str) -> String {
    match project {
        Some(project) => format!("{project}\u{1}{manifest}"),
        None => manifest.to_string(),
    }
}
type PulledBy = BTreeMap<String, BTreeMap<u64, PullSaid>>;

/// A row on the wire. A task's links are read from the repository each was linked in, which is not
/// its task's once the task has moved.
fn row_on_the_wire(row: KeptProjectRow, project: Option<&str>, said_by: &SaidBy, pulled_by: &PulledBy) -> ProjectRow {
    let manifest_id = ManifestId::carried(row.manifest_id.clone());
    match row.item {
        KeptRowItem::Task { title, notes, state, parent, issues, pulls } => {
            let (state, held_by) = task_state(state);
            // A link kept before links named their repository reads as its task's.
            let linked_in = |linked: &str| Some(ManifestId::carried(if linked.is_empty() { row.manifest_id.clone() } else { linked.to_string() }));
            let issues = issues
                .into_iter()
                .map(|link| {
                    let read = said_by.get(&said_key(project, &link.manifest_id)).and_then(|said| said.get(&link.reference));
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
            let read = said_by.get(&said_key(project, &row.manifest_id)).and_then(|said| said.get(&reference));
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

fn member_on_the_wire(member: &core_model::Member) -> ProjectMember {
    ProjectMember {
        manifest_id: ManifestId::from(member.repository()),
        folder: member.folder_of().map(|folder| folder.as_str().to_string()),
    }
}

/// Where a project's file is kept, as the person chose it: none chosen is this Mac.
fn place_on_the_wire(kept: Option<KeptPlace>) -> ProjectPlace {
    let Some(kept) = kept else {
        return ProjectPlace { place: RunPlace::ThisMac, synced_folder: None, cloud: None };
    };
    let place = match place_of(&kept.place) {
        RunPlace::Repository => RunPlace::ThisMac,
        place => place,
    };
    ProjectPlace {
        place,
        synced_folder: kept.synced_folder,
        cloud: kept.cloud.as_deref().and_then(|text| cloud::Target::kept(text).ok()).map(|target| target.wire()),
    }
}

fn left_out_on_the_wire(one: &config::ProjectLeftOut) -> ProjectLeftOut {
    let place = match one.place() {
        config::ProjectPlace::ThisMac => RunPlace::ThisMac,
        config::ProjectPlace::Synced => RunPlace::Synced,
        config::ProjectPlace::Cloud => RunPlace::Cloud,
    };
    ProjectLeftOut { place, path: one.path().to_string_lossy().into_owned(), why: one.said().to_string() }
}

/// Whether the file set aside is project `id`'s: named `<id>.yml`, or the id itself where it could not name one.
fn left_out_is(one: &config::ProjectLeftOut, id: &str) -> bool {
    one.path().file_stem().is_some_and(|stem| stem == id) || one.path() == std::path::Path::new(id)
}

/// What a write put in `project`, where it is one of his: none for the loose tasks.
fn put_in(project: &str, repositories: Vec<String>) -> Option<Put> {
    (project != LOOSE_PROJECT).then(|| (project.to_string(), repositories))
}

fn rows_of(kept: &KeptProjects) -> impl Iterator<Item = &KeptProjectRow> {
    kept.projects.iter().flat_map(|project| &project.rows).chain(&kept.loose)
}

/// The references of the issues in `kept`, by the repository each issue row is in or each link was made in, and the
/// project whose own tracker reads them: **only a project in `named`**, the rest are read by their repository's.
fn issue_references(kept: &KeptProjects, named: &BTreeSet<String>) -> BTreeMap<(Option<String>, String), Vec<String>> {
    let mut by: BTreeMap<(Option<String>, String), Vec<String>> = BTreeMap::new();
    let rows = kept
        .projects
        .iter()
        .flat_map(|project| project.rows.iter().map(move |row| (named.contains(&project.id).then(|| project.id.clone()), row)))
        .chain(kept.loose.iter().map(|row| (None, row)));
    for (project, row) in rows {
        match &row.item {
            KeptRowItem::Issue { reference } => by.entry((project, row.manifest_id.clone())).or_default().push(reference.clone()),
            KeptRowItem::Task { issues, .. } => {
                for link in issues {
                    by.entry((project.clone(), link.manifest_id.clone())).or_default().push(link.reference.clone());
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

    pub(crate) fn blank(&self, said: &str) -> Refusal {
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

    /// Where `project` names a tracker, the issue `reference` must be one it has: a 404 `fleet.no_such_issue` where it
    /// answers that it has none. **A tracker that cannot answer refuses nothing**, as no read of one does, and a project
    /// that names none, or none at all, leaves the reference unchecked as it always was.
    async fn issue_checked(&self, project: Option<&str>, manifest: &str, reference: &IssueReference) -> Result<(), Refusal> {
        let Some(project) = project.filter(|id| self.project_names_tracker(id)) else {
            return Ok(());
        };
        let Ok(served) = self.served_named(Some(&ManifestId::carried(manifest))) else {
            return Ok(());
        };
        let tracker = self.trackers().tracker(&self.choice_in(&served, Some(project)));
        let (root, asked) = (served.root().to_string(), reference.as_str().to_string());
        match tokio::task::spawn_blocking(move || tracker.read_issue(&root, &asked)).await {
            Ok(Ok(None)) => Err(Refusal::NotFound(
                self.project_refusal(NO_SUCH_ISSUE, format!("The tracker project {project} names has no issue {reference}")),
            )),
            _ => Ok(()),
        }
    }

    fn unreadable_pull(&self, number: u64) -> Refusal {
        Refusal::Unacceptable(self.project_refusal(NOT_A_PULL, format!("`{number}` is not a pull request's number")))
    }

    pub(crate) fn minted(&self) -> String {
        self.mint().ulid().as_str().to_lowercase()
    }

    /// What the tracker of `manifest_id` says of each of `references`, by reference. **Never refuses**: one
    /// it could not say is left out, and the row draws without a state.
    async fn issues_said(&self, manifest_id: &ManifestId, project: Option<&str>, references: Vec<String>) -> BTreeMap<String, Said> {
        let mut said = BTreeMap::new();
        if references.is_empty() {
            return said;
        }
        // A project that names a tracker reads its rows' issues through it, from its own open list.
        let open = match project {
            Some(project) => self.project_open_issues(project, false).await.map(|(held, ..)| held.issues),
            None => self.open_issues(manifest_id).await,
        };
        let Ok(open) = open else {
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
            let held_under = said_key(project, manifest_id.as_str());
            if let Some(held) = self.tracking().reads.held(&held_under, &reference, &now) {
                said.extend(held.map(|one| (reference.clone(), one)));
                continue;
            }
            let tracker = self.trackers().tracker(&self.choice_in(&served, project));
            let (root, asked) = (served.root().to_string(), reference.clone());
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
            self.tracking().reads.keep(&held_under, &reference, now.clone(), read.clone());
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

    /// This Mac's own projects on the wire; [`Fleet::projects_on_the_wire`] adds the copies of his other Macs'.
    pub(crate) async fn own_projects_on_the_wire(&self, kept: KeptProjects) -> ipc::Projects {
        let (mut said, mut pulled): (SaidBy, PulledBy) = (BTreeMap::new(), BTreeMap::new());
        let named: BTreeSet<String> = kept.projects.iter().filter(|project| self.project_names_tracker(&project.id)).map(|project| project.id.clone()).collect();
        for ((project, manifest), references) in issue_references(&kept, &named) {
            let read = self.issues_said(&ManifestId::carried(manifest.clone()), project.as_deref(), references).await;
            said.insert(said_key(project.as_deref(), &manifest), read);
        }
        for (manifest, numbers) in pull_numbers(&kept) {
            let read = self.pulls_said(&ManifestId::carried(manifest.clone()), numbers).await;
            pulled.insert(manifest, read);
        }
        let wire = |project: Option<&str>, rows: Vec<KeptProjectRow>| rows.into_iter().map(|row| row_on_the_wire(row, project, &said, &pulled)).collect();
        // **The file is the truth for a name and the members**, and the store's name is the last it saw,
        // shown where the file cannot be read, with no members.
        let held = self.projects_held();
        let places: Vec<Option<KeptPlace>> = {
            let store = self.store().lock().await;
            kept.projects.iter().map(|project| store.project_place(&project.id).ok().flatten()).collect()
        };
        let projects = kept
            .projects
            .into_iter()
            .zip(places)
            .map(|(project, place)| {
                let rows = wire(named.contains(&project.id).then_some(project.id.as_str()), project.rows);
                let read = ScopeId::new(&project.id).ok().and_then(|scope| held.get(&scope));
                let unread = match read {
                    Some(_) => None,
                    None => Some(
                        held.left_out()
                            .iter()
                            .find(|one| left_out_is(one, &project.id))
                            .map_or_else(|| String::from("its file has not been read"), |one| one.said().to_string()),
                    ),
                };
                Project {
                    name: read.map_or(project.name, |one| one.name().as_str().to_string()),
                    members: read.map(|one| one.members().iter().map(member_on_the_wire).collect()).unwrap_or_default(),
                    place: place_on_the_wire(place),
                    unread,
                    values: read.and_then(|one| crate::values::values_set(one.values())),
                    shared_with_macs: read.is_some_and(|one| one.shared_with_macs()),
                    store: read.and_then(|one| one.store()).map(crate::project_store::store_on_the_wire),
                    mac: None,
                    id: project.id,
                    rows,
                }
            })
            .collect();
        ipc::Projects { projects, loose: wire(None, kept.loose), left_out: held.left_out().iter().map(left_out_on_the_wire).collect() }
    }

    /// `write` against the store, given his projects as they were; then the projects
    /// as they are, answered and published.
    async fn projects_written<F>(&self, write: F) -> Result<ipc::Projects, Refusal>
    where
        F: FnOnce(&mut Store, &KeptProjects) -> Result<(), Refusal> + Send,
    {
        self.projects_written_reading(write, false).await
    }

    /// [`Fleet::projects_written`], reading every project's file again before the answer where `reading`:
    /// a write of a project itself, and not of a row in it.
    pub(crate) async fn projects_written_reading<F>(&self, write: F, reading: bool) -> Result<ipc::Projects, Refusal>
    where
        F: FnOnce(&mut Store, &KeptProjects) -> Result<(), Refusal> + Send,
    {
        self.projects_written_following(|store, kept| write(store, kept).map(|()| None), reading).await
    }

    /// [`Fleet::projects_written_reading`], where the write may say it put a row of `repositories` in the project it
    /// returns. **A project's members are the repositories its tasks name**, so any of them the project does not
    /// hold becomes a whole member, written to its file, before the answer.
    async fn projects_written_following<F>(&self, write: F, reading: bool) -> Result<ipc::Projects, Refusal>
    where
        F: FnOnce(&mut Store, &KeptProjects) -> Result<Option<Put>, Refusal> + Send,
    {
        let (kept, put) = {
            let mut store = self.store().lock().await;
            let before = store.projects().map_err(|why| self.ledger_fault(why))?;
            let put = write(&mut store, &before)?;
            (store.projects().map_err(|why| self.ledger_fault(why))?, put)
        };
        let followed = match put {
            Some(put) => self.members_followed(put).await?,
            None => false,
        };
        if reading || followed {
            self.projects_reload().await;
        }
        let projects = self.projects_on_the_wire(kept).await;
        self.publish(ipc::Event::ProjectsChanged(projects.clone()));
        Ok(projects)
    }

    /// `add_project_task`, the row kept under `id` where the Mac that made the act minted it.
    pub(crate) async fn add_project_task_as(&self, id: Option<String>, task: NewProjectTask) -> Result<ipc::Projects, Refusal> {
        self.served_named(Some(&task.manifest_id))?;
        let (id, held) = self.row_minted(id).await?;
        if held {
            return self.list_projects().await;
        }
        self.projects_written_following(
            |store, kept| {
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
                store.add_project_row(&project, &row).map_err(|why| self.ledger_fault(why))?;
                Ok(put_in(&project, vec![task.manifest_id.as_str().to_string()]))
            },
            false,
        )
        .await
    }

    /// `add_project_subtask`, the row kept under `id` where the Mac that made the act minted it.
    pub(crate) async fn add_project_subtask_as(&self, id: Option<String>, row: String, task: ProjectTask) -> Result<ipc::Projects, Refusal> {
        let (id, held) = self.row_minted(id).await?;
        if held {
            return self.list_projects().await;
        }
        self.projects_written_following(
            |store, kept| {
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
                store.add_project_row(project, &subtask).map_err(|why| self.ledger_fault(why))?;
                Ok(put_in(project, vec![parent.manifest_id.clone()]))
            },
            false,
        )
        .await
    }

    /// `edit_project_task`, each field stamped `edited`.
    /// `fields` names the ones the person changed, where it is known; none is both.
    pub(crate) async fn edit_project_task_edited(
        &self,
        row: String,
        task: ProjectTask,
        edited: Edited,
        fields: Option<EditedTaskFields>,
    ) -> Result<ipc::Projects, Refusal> {
        let fields = fields.unwrap_or(EditedTaskFields { title: true, notes: true });
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            let title = task.title.trim();
            if title.is_empty() {
                return Err(self.blank("a task needs a title"));
            }
            store.edit_project_task(&row, fields.title.then_some(title), fields.notes.then_some(task.notes.trim()), edited).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    /// `tick_project_task`, the state stamped `edited`.
    pub(crate) async fn tick_project_task_edited(&self, row: String, tick: TickProjectTask, edited: Edited) -> Result<ipc::Projects, Refusal> {
        let state = if tick.done { KeptTaskState::Done } else { KeptTaskState::Open };
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            store.set_project_task_state(&row, state, edited).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    /// `move_project_row`, the row's place stamped `edited`.
    pub(crate) async fn move_project_row_edited(&self, row: String, to: MoveProjectRow, edited: Edited) -> Result<ipc::Projects, Refusal> {
        self.projects_written_following(
            |store, kept| {
                let (was, moving) = self.row_in(kept, &row)?;
                let into = match to.project.as_deref() {
                    Some(project) => self.project_named(kept, Some(project))?,
                    None => was.to_string(),
                };
                if let Some(parent) = to.parent.as_deref().filter(|parent| !parent.is_empty()) {
                    self.moved_under(kept, &row, parent, &into)?;
                }
                // The row and its subtasks go with it, each in the repository it is in.
                let mut repositories = vec![moving.manifest_id.clone()];
                for sub in subtasks_of(kept, &row) {
                    repositories.extend(self.row_in(kept, &sub).ok().map(|(_, one)| one.manifest_id.clone()));
                }
                repositories.dedup();
                store
                    .move_project_row(&row, to.project.as_ref().map(|_| into.as_str()), to.position as usize, to.parent.as_deref(), edited)
                    .map_err(|why| self.ledger_fault(why))?;
                Ok(put_in(&into, repositories))
            },
            false,
        )
        .await
    }

    /// Move a top-level task and its subtasks to the repository `manifest_id`, which Fleet serves. **The
    /// issues and pull requests linked to them stay the repository's they were linked in**, and a Job
    /// holding the task keeps it. Moving it to the repository it is in changes nothing. The project it is
    /// in comes to hold the repository.
    pub async fn move_project_task_repository(&self, row: String, manifest_id: ManifestId) -> Result<ipc::Projects, Refusal> {
        self.served_named(Some(&manifest_id))?;
        self.projects_written_following(
            |store, kept| {
                let (project, _) = self.top_level_task_in(kept, &row)?;
                store.move_project_task_repository(&row, manifest_id.as_str()).map_err(|why| self.ledger_fault(why))?;
                Ok(put_in(project, vec![manifest_id.as_str().to_string()]))
            },
            false,
        )
        .await
    }

    /// [`Fleet::members_followed`] for `repositories` the Jobs of `project` are in, and where its file changed the projects read
    /// again and published as `projects.changed`. Whether the file changed.
    pub(crate) async fn members_followed_published(&self, project: &str, repositories: Vec<String>) -> Result<bool, Refusal> {
        if !self.members_followed((project.to_string(), repositories)).await? {
            return Ok(false);
        }
        self.projects_reload().await;
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        let projects = self.projects_on_the_wire(kept).await;
        self.publish(ipc::Event::ProjectsChanged(projects));
        Ok(true)
    }

    /// Make each repository of `put` a whole member of the project, where it holds none of it, and write its file.
    /// A project whose file did not read is left as it is, since a file that will not read is not written over.
    /// Whether the file changed.
    async fn members_followed(&self, (project, repositories): Put) -> Result<bool, Refusal> {
        let Some(held) = ScopeId::new(&project).ok().and_then(|scope| self.projects_held().get(&scope).cloned()) else {
            return Ok(false);
        };
        let mut members = held.members().to_vec();
        for repository in repositories {
            let repository = core_model::ManifestId::carried(Ulid::carried(&repository));
            if !held.holds(&repository) && !members.iter().any(|one| one.repository() == &repository) {
                members.push(Member::whole(repository));
            }
        }
        if members.len() == held.members().len() {
            return Ok(false);
        }
        let Ok(grown) = core_model::Project::declared(held.id().clone(), held.name().clone(), members) else {
            return Ok(false);
        };
        let grown = grown.with_values(held.values().clone()).with_shared_with_macs(held.shared_with_macs()).with_store(held.store().cloned());
        self.project_stored(&grown).await?;
        Ok(true)
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
    async fn get_project_store(&self, project: String) -> Result<ipc::ProjectStoreState, Refusal> {
        self.project_store_state(&project).await
    }

    async fn set_project_store(&self, project: String, set: ipc::SetProjectStore) -> Result<ipc::ProjectStoreState, Refusal> {
        self.project_store_set(&project, set).await
    }

    async fn deploy_project_store(&self, project: String, deploy: ipc::DeployProjectStore) -> Result<ipc::StoreDeployed, Refusal> {
        self.project_store_deployed(&project, deploy).await
    }

    async fn list_projects(&self) -> Result<ipc::Projects, Refusal> {
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        Ok(self.projects_on_the_wire(kept).await)
    }

    async fn list_holding_projects(&self, manifest_id: Option<ManifestId>) -> Result<ipc::Projects, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        // From the files Fleet holds, so a project is listed whether or not his order has a row for it yet.
        // No rows are read, so no tracker or forge is asked.
        let held = self.projects_held();
        let store = self.store().lock().await;
        let projects = held
            .holding(served.manifest().id())
            .into_iter()
            .map(|one| Project {
                id: one.id().as_str().to_string(),
                name: one.name().as_str().to_string(),
                rows: Vec::new(),
                members: one.members().iter().map(member_on_the_wire).collect(),
                place: place_on_the_wire(store.project_place(one.id().as_str()).ok().flatten()),
                unread: None,
                values: crate::values::values_set(one.values()),
                shared_with_macs: one.shared_with_macs(),
                store: one.store().map(crate::project_store::store_on_the_wire),
                mac: None,
            })
            .collect();
        Ok(ipc::Projects { projects, loose: Vec::new(), left_out: held.left_out().iter().map(left_out_on_the_wire).collect() })
    }

    async fn add_project(&self, new: NewProject) -> Result<ipc::Projects, Refusal> {
        let name = new.name.trim().to_string();
        if name.is_empty() {
            return Err(self.blank("a project needs a name"));
        }
        let (id, at) = (self.minted(), self.now());
        let members = self.members_read(new.members.as_deref().unwrap_or_default())?;
        let made = self.project_declared(&id, &name, members)?;
        let made = match &new.values {
            Some(set) => made.with_values(self.values_read(set)?),
            None => made,
        };
        let chosen = match new.place {
            Some(place) => Some(self.project_place_chosen(&id, place.place, place.synced_folder, place.cloud).await?),
            None => None,
        };
        // Its file first, so the row never stands without one.
        self.project_stored_at(&made, chosen.as_ref()).await?;
        self.projects_written_reading(
            |store, _| {
                store.add_project(&id, &name, at.as_str()).map_err(|why| self.ledger_fault(why))?;
                match &chosen {
                    Some(chosen) => store
                        .set_project_place(&id, &chosen.place, chosen.synced_folder.as_deref(), chosen.cloud.as_deref())
                        .map_err(|why| self.ledger_fault(why)),
                    None => Ok(()),
                }
            },
            true,
        )
        .await
    }

    async fn edit_project(&self, project: String, edit: EditProject) -> Result<ipc::Projects, Refusal> {
        self.project_not_held(&project).await?;
        {
            let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
            self.project_in(&kept, &project)?;
        }
        let name = edit.name.as_deref().map(str::trim);
        if name.is_some_and(str::is_empty) {
            return Err(self.blank("a project needs a name"));
        }
        let held = self.project_read(&project)?;
        let members = match &edit.members {
            Some(members) => self.members_read(members)?,
            None => held.members().to_vec(),
        };
        let edited = self.project_declared(&project, name.unwrap_or(held.name().as_str()), members)?;
        let edited = edited
            .with_values(match &edit.values {
                Some(set) => self.values_read(set)?,
                None => held.values().clone(),
            })
            .with_shared_with_macs(edit.shared_with_macs.unwrap_or(held.shared_with_macs()))
            .with_store(held.store().cloned());
        let chosen = match edit.place {
            Some(place) => Some(self.project_place_chosen(&project, place.place, place.synced_folder, place.cloud).await?),
            None => None,
        };
        match &chosen {
            Some(chosen) => self.project_relocated(&edited, chosen).await?,
            None if edited != held => self.project_stored(&edited).await?,
            None => {}
        }
        self.project_learned_standing(&project, edited.values()).await;
        let named = edited.name().as_str().to_string();
        self.projects_written_reading(|store, _| store.rename_project(&project, &named).map(drop).map_err(|why| self.ledger_fault(why)), true).await
    }

    async fn get_values(&self, project: Option<String>, manifest_id: Option<ManifestId>) -> Result<ipc::ResolvedValues, Refusal> {
        self.values_asked(project, manifest_id).await
    }

    async fn list_project_issues(&self, project: String, refresh: bool) -> Result<ipc::ProjectIssueList, Refusal> {
        self.project_issues_listed(project, refresh).await
    }

    async fn save_kit_values(&self, save: ipc::SaveKitValues) -> Result<ipc::KitValuesSaved, Refusal> {
        self.kit_values_saved(save).await
    }

    async fn set_project_workflow(&self, project: String, set: ipc::SetProjectWorkflow) -> Result<ipc::Projects, Refusal> {
        let workflow = set.workflow.trim();
        if workflow.is_empty() {
            return Err(self.blank("a workflow is switched by its id"));
        }
        self.project_not_held(&project).await?;
        {
            let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
            self.project_in(&kept, &project)?;
        }
        let mut values = self.project_read(&project)?.values().clone();
        values.workflows_off.retain(|one| one != workflow);
        if !set.on {
            values.workflows_off.push(workflow.to_string());
        }
        // Through `edit_project`, so the file is written, checked and published as every edit of a project's values is.
        let values = crate::values::values_set(&values).unwrap_or_default();
        self.edit_project(project, EditProject { values: Some(values), ..EditProject::default() }).await
    }

    async fn delete_project(&self, project: String) -> Result<ipc::Projects, Refusal> {
        self.project_not_held(&project).await?;
        {
            let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
            self.project_in(&kept, &project)?;
        }
        self.project_file_removed(&project).await?;
        self.projects_written_reading(|store, _| store.delete_project(&project).map(drop).map_err(|why| self.ledger_fault(why)), true).await
    }

    async fn add_project_task(&self, task: NewProjectTask) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.copy_of_project(task.project.as_deref()).await? {
            return self.act_on_copy(&mac, ProjectAct::AddProjectTask { task, id: None }).await;
        }
        self.add_project_task_as(None, task).await
    }

    async fn add_project_subtask(&self, row: String, task: ProjectTask) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.copy_of_row(&row).await? {
            return self.act_on_copy(&mac, ProjectAct::AddProjectSubtask { row, task, id: None }).await;
        }
        self.add_project_subtask_as(None, row, task).await
    }

    async fn link_task_issue(&self, row: String, link: LinkProjectIssue) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.copy_of_row(&row).await? {
            return self.act_on_copy(&mac, ProjectAct::LinkTaskIssue { row, link }).await;
        }
        if let Some(issue) = IssueReference::read(&link.issue) {
            let (project, manifest) = {
                let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
                let (project, found) = self.row_in(&kept, &row)?;
                (project.to_string(), found.manifest_id.clone())
            };
            self.issue_checked(Some(&project), &manifest, &issue).await?;
        }
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            let issue = IssueReference::read(&link.issue).ok_or_else(|| self.unreadable_issue(&link.issue))?;
            store.link_task_issue(&row, issue.as_str()).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn unlink_task_issue(&self, row: String, link: LinkProjectIssue) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.copy_of_row(&row).await? {
            return self.act_on_copy(&mac, ProjectAct::UnlinkTaskIssue { row, link }).await;
        }
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            let issue = IssueReference::read(&link.issue).ok_or_else(|| self.unreadable_issue(&link.issue))?;
            store.unlink_task_issue(&row, issue.as_str()).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn link_task_job(&self, row: String, link: LinkTaskJob) -> Result<ipc::Projects, Refusal> {
        self.row_not_held(&row).await?;
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
        self.row_not_held(&row).await?;
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            store.free_held_project_task(&row).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn link_task_pull(&self, row: String, link: LinkTaskPull) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.copy_of_row(&row).await? {
            return self.act_on_copy(&mac, ProjectAct::LinkTaskPull { row, link }).await;
        }
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
        if let Some(mac) = self.copy_of_row(&row).await? {
            return self.act_on_copy(&mac, ProjectAct::UnlinkTaskPull { row, link }).await;
        }
        self.projects_written(|store, kept| {
            self.task_in(kept, &row)?;
            store.unlink_task_pull(&row, link.number).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn edit_project_task(&self, row: String, task: ProjectTask) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.copy_of_row(&row).await? {
            return self.act_on_copy(&mac, ProjectAct::EditProjectTask { row, task, fields: None }).await;
        }
        self.edit_project_task_edited(row, task, self.edited_now(), None).await
    }

    async fn link_project_issue(&self, link: NewProjectIssue) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.copy_of_project(link.project.as_deref()).await? {
            return self.act_on_copy(&mac, ProjectAct::LinkProjectIssue { link }).await;
        }
        self.served_named(Some(&link.manifest_id))?;
        if let Some(issue) = IssueReference::read(&link.issue) {
            self.issue_checked(link.project.as_deref(), link.manifest_id.as_str(), &issue).await?;
        }
        let id = self.minted();
        self.projects_written_following(
            |store, kept| {
                let project = self.project_named(kept, link.project.as_deref())?;
                let issue = IssueReference::read(&link.issue).ok_or_else(|| self.unreadable_issue(&link.issue))?;
                let item = KeptRowItem::Issue { reference: issue.as_str().to_string() };
                let row = KeptProjectRow { id, manifest_id: link.manifest_id.as_str().to_string(), item };
                store.add_project_row(&project, &row).map_err(|why| self.ledger_fault(why))?;
                Ok(put_in(&project, vec![link.manifest_id.as_str().to_string()]))
            },
            false,
        )
        .await
    }

    async fn remove_project_row(&self, row: String) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.copy_of_row(&row).await? {
            return self.act_on_copy(&mac, ProjectAct::RemoveProjectRow { row }).await;
        }
        self.projects_written(|store, kept| {
            self.row_in(kept, &row)?;
            store.remove_project_row(&row).map(drop).map_err(|why| self.ledger_fault(why))
        })
        .await
    }

    async fn move_project_row(&self, row: String, to: MoveProjectRow) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.move_on_copy(&row, &to).await? {
            return self.act_on_copy(&mac, ProjectAct::MoveProjectRow { row, to }).await;
        }
        self.move_project_row_edited(row, to, self.edited_now()).await
    }

    async fn move_project_task_repository(&self, row: String, to: MoveProjectTaskRepository) -> Result<ipc::Projects, Refusal> {
        self.row_not_held(&row).await?;
        Fleet::move_project_task_repository(self, row, to.manifest_id).await
    }

    async fn tick_project_task(&self, row: String, tick: TickProjectTask) -> Result<ipc::Projects, Refusal> {
        if let Some(mac) = self.copy_of_row(&row).await? {
            return self.act_on_copy(&mac, ProjectAct::TickProjectTask { row, tick }).await;
        }
        self.tick_project_task_edited(row, tick, self.edited_now()).await
    }
}
