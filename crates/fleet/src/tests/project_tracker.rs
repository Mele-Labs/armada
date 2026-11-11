//! A project's tracker replaces its members' trackers for the Jobs in it and for the project's issues.
//! Providers here are made-up words: what a provider's word means is `adapters`' to assert, and under
//! test is that Fleet asks the project's choice where a Job is in a project that names one.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::Backlog;
use api::{HostedSessions, Projects, Sessions};
use ipc::{ClaimIssue, IssueState, ManifestId, NewProject, ProjectMember, ProjectRow, SaveKitValues, SessionRecord, StartSession, TaskIssue, ValueLayer, ValuesSet};
use testkit::{FakeBeads, FakeBeadsRemote, FakeHarness, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

use super::daemon::fittings;
use super::main_ci::MANIFEST;
use super::projects::{code_of, link, new_issue, new_task};
use super::session_host::{Shared, StandIn};
use super::tmp::TempDir;
use crate::daemon::Fleet;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The repository's own tracker, and the one a project names.
const REPOSITORY_BOARD: &str = "repoboard";
const PROJECT_BOARD: &str = "projboard";

struct Rig {
    fleet: Arc<Hosted>,
    _home: TempDir,
}

/// A Fleet serving one repository whose `armada.yml` names `repository_tracker` (or none) over `trackers`.
fn a_rig(trackers: FakeTrackers, repository_tracker: Option<&str>) -> Rig {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.trackers = Arc::new(trackers);
    let block = repository_tracker.map(|provider| format!("tracker:\n  provider: {provider}\n")).unwrap_or_default();
    fittings.starting().manifest = config::Manifest::parse(Path::new("armada.yml"), &format!("{MANIFEST}{block}")).expect("parses");
    let fleet = Fleet::assembled(fittings).hosting_sessions_on(Arc::new(Shared(Arc::new(StandIn::default()))), Duration::from_secs(600));
    Rig { fleet: Arc::new(fleet), _home: home }
}

/// Two backlogs, one for each board, and a Fleet over them.
fn over_two_backlogs() -> (Rig, Arc<FakeBeads>, FakeBeadsRemote, Arc<FakeBeads>, FakeBeadsRemote) {
    let (repository_remote, project_remote) = (FakeBeadsRemote::new(), FakeBeadsRemote::new());
    let (repository_copy, project_copy) = (Arc::new(FakeBeads::on(&repository_remote)), Arc::new(FakeBeads::on(&project_remote)));
    let trackers = FakeTrackers::empty()
        .beads_named(REPOSITORY_BOARD, Arc::clone(&repository_copy))
        .beads_named(PROJECT_BOARD, Arc::clone(&project_copy));
    (a_rig(trackers, Some(REPOSITORY_BOARD)), repository_copy, repository_remote, project_copy, project_remote)
}

fn project_board() -> ValuesSet {
    let block = BTreeMap::from([("provider".to_string(), PROJECT_BOARD.to_string()), ("site".to_string(), "acme".to_string())]);
    ValuesSet { tracker: Some(block), ..ValuesSet::default() }
}

impl Rig {
    fn manifest(&self) -> ManifestId {
        ManifestId::carried(self.fleet.repositories().first().expect("served").manifest().id().as_str())
    }

    async fn project(&self, values: Option<ValuesSet>) -> String {
        let members = Some(vec![ProjectMember { manifest_id: self.manifest(), folder: None }]);
        let new = NewProject { name: "Shop".to_string(), members, place: None, values };
        self.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
    }

    async fn job(&self, project: Option<&str>) -> SessionRecord {
        let start = StartSession {
            project: project.map(str::to_string),
            manifest_id: self.manifest(),
            title: Some("Ship it".to_string()),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
        };
        self.fleet.start_session(start).await.expect("started")
    }

    async fn claims(&self, job: &SessionRecord, issue: &str) {
        let claim = ClaimIssue { issue: issue.into(), session_id: Some(job.id.clone()), pull_requests: Vec::new() };
        self.fleet.claim_issue(None, claim).await.expect("claimed");
    }
}

fn held(remote: &FakeBeadsRemote, issue: &str) -> bool {
    remote.holder(issue).is_some_and(|holder| !holder.is_empty())
}

#[tokio::test]
async fn a_job_in_a_project_that_names_a_tracker_claims_through_it_and_not_through_its_repositorys() {
    let (rig, repository_copy, repository_remote, project_copy, project_remote) = over_two_backlogs();
    let issue = project_copy.create("On the project's board");
    project_copy.sync("/").expect("pushed");
    let beside = repository_copy.create("On the repository's board");
    repository_copy.sync("/").expect("pushed");
    let project = rig.project(Some(project_board())).await;
    let job = rig.job(Some(&project)).await;

    rig.claims(&job, &issue).await;

    assert!(held(&project_remote, &issue), "the claim is on the project's board");
    assert!(!held(&repository_remote, &beside), "and nothing was claimed on the repository's");
    assert_eq!(repository_copy.syncs(), 1, "Fleet never synced the repository's copy: the one sync is the test's own push");
}

#[tokio::test]
async fn a_job_in_no_project_or_in_a_project_that_names_none_claims_through_its_repositorys() {
    let (rig, repository_copy, repository_remote, _project_copy, project_remote) = over_two_backlogs();
    let first = repository_copy.create("Filed on the repository's board");
    let second = repository_copy.create("Filed on it too");
    repository_copy.sync("/").expect("pushed");
    let bare = rig.project(None).await;

    let in_none = rig.job(None).await;
    rig.claims(&in_none, &first).await;
    let in_a_bare_project = rig.job(Some(&bare)).await;
    rig.claims(&in_a_bare_project, &second).await;

    assert!(held(&repository_remote, &first), "a Job in no project claims on its repository's board");
    assert!(held(&repository_remote, &second), "so does one in a project that names no tracker");
    assert_eq!(project_remote.holder(&first), None);
}

#[tokio::test]
async fn get_values_names_the_project_for_a_tracker_that_replaces_the_repositorys() {
    let (rig, ..) = over_two_backlogs();
    let project = rig.project(Some(project_board())).await;
    let bare = rig.project(None).await;
    let asked = |project: Option<&str>, repository: Option<ManifestId>| rig.fleet.get_values(project.map(str::to_string), repository);

    let neither = asked(None, None).await.expect("values").tracker.expect("a tracker answer");
    assert_eq!((neither.value, neither.from), (None, ValueLayer::Default), "no repository and no project name none");

    let repository = asked(None, Some(rig.manifest())).await.expect("values").tracker.expect("a tracker answer");
    assert_eq!(repository.from, ValueLayer::Repository);
    assert_eq!(repository.value.and_then(|block| block.get("provider").cloned()).as_deref(), Some(REPOSITORY_BOARD));

    let both = asked(Some(&project), Some(rig.manifest())).await.expect("values").tracker.expect("a tracker answer");
    assert_eq!(both.from, ValueLayer::Project, "the project's replaces the repository's");
    let block = both.value.expect("the block");
    assert_eq!((block.get("provider").map(String::as_str), block.get("site").map(String::as_str)), (Some(PROJECT_BOARD), Some("acme")), "carried keys are kept");

    let without = asked(Some(&bare), Some(rig.manifest())).await.expect("values").tracker.expect("a tracker answer");
    assert_eq!(without.from, ValueLayer::Repository, "a project that names none leaves the repository's");
}

#[tokio::test]
async fn a_tracker_in_kits_values_is_refused() {
    let (rig, ..) = over_two_backlogs();
    let refused = rig.fleet.save_kit_values(SaveKitValues { values: project_board() }).await.expect_err("refused");
    assert_eq!(code_of(refused), (422, "fleet.project_values_refused".to_string()));
}

#[tokio::test]
async fn the_project_issues_read_lists_the_project_trackers_issues_and_not_the_repositorys() {
    let (on_repository, on_project) = (Arc::new(FakeTracker::listing(vec![FakeTracker::issue(1, "Repository's")])), Arc::new(FakeTracker::listing(vec![FakeTracker::issue(2, "Project's")])));
    let trackers = FakeTrackers::empty().tracker_named(REPOSITORY_BOARD, on_repository.clone()).tracker_named(PROJECT_BOARD, on_project.clone());
    let rig = a_rig(trackers, Some(REPOSITORY_BOARD));
    let project = rig.project(Some(project_board())).await;

    let listed = rig.fleet.list_project_issues(project.clone(), false).await.expect("listed");

    assert_eq!(listed.project, project);
    assert_eq!(listed.issues.iter().map(|one| one.title.as_str()).collect::<Vec<_>>(), ["Project's"]);
    assert_eq!((on_repository.listings(), on_project.listings()), (0, 1));
    rig.fleet.list_project_issues(project.clone(), false).await.expect("listed again");
    assert_eq!(on_project.listings(), 1, "held for five minutes");
    rig.fleet.list_project_issues(project, true).await.expect("refreshed");
    assert_eq!(on_project.listings(), 2, "`refresh` reads again");
}

#[tokio::test]
async fn the_project_issues_read_is_refused_for_a_project_that_names_no_tracker_and_one_there_is_not() {
    let rig = a_rig(FakeTrackers::empty(), Some(REPOSITORY_BOARD));
    let bare = rig.project(None).await;

    let refused = rig.fleet.list_project_issues(bare, false).await.expect_err("refused");
    assert_eq!(code_of(refused), (422, "fleet.project_no_tracker".to_string()));
    let missing = rig.fleet.list_project_issues("nope".to_string(), false).await.expect_err("refused");
    assert_eq!(code_of(missing), (404, "fleet.no_such_project".to_string()));
}

/// Two boards: the repository's lists #1, the project's lists #2 and knows #3 closed.
fn two_boards() -> Rig {
    let on_repository = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(1, "On the repository's board")]));
    let on_project = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(2, "On the project's board")]));
    on_project.knows_closed(3, "Closed on the project's board");
    a_rig(FakeTrackers::empty().tracker_named(REPOSITORY_BOARD, on_repository).tracker_named(PROJECT_BOARD, on_project), Some(REPOSITORY_BOARD))
}

fn issue_rows(projects: &ipc::Projects, at: usize) -> Vec<(String, Option<IssueState>, Option<String>)> {
    projects.projects[at]
        .rows
        .iter()
        .filter_map(|row| match row {
            ProjectRow::Issue { issue, state, title, .. } => Some((issue.clone(), *state, title.clone())),
            ProjectRow::Task { .. } => None,
        })
        .collect()
}

fn linked(projects: &ipc::Projects, at: usize) -> Vec<(String, Option<String>)> {
    let tasks = projects.projects[at].rows.iter().chain(&projects.loose);
    tasks
        .filter_map(|row| match row {
            ProjectRow::Task { issues, .. } => Some(issues.iter().map(|one: &TaskIssue| (one.issue.clone(), one.title.clone())).collect::<Vec<_>>()),
            ProjectRow::Issue { .. } => None,
        })
        .flatten()
        .collect()
}

#[tokio::test]
async fn a_projects_issue_rows_and_linked_issues_are_read_through_its_tracker_and_others_through_the_repositorys() {
    let rig = two_boards();
    let (named, bare) = (rig.project(Some(project_board())).await, rig.project(None).await);
    let manifest = rig.manifest();
    for issue in ["2", "3"] {
        rig.fleet.link_project_issue(new_issue(manifest.clone(), Some(named.clone()), issue)).await.expect("linked");
    }
    rig.fleet.link_project_issue(new_issue(manifest.clone(), Some(bare.clone()), "1")).await.expect("linked");
    let task = rig.fleet.add_project_task(new_task(manifest.clone(), Some(named.clone()), "Ship")).await.expect("added");
    let row = |projects: &ipc::Projects| match projects.projects.iter().find(|one| one.id == named).expect("the project").rows.iter().find(|row| matches!(row, ProjectRow::Task { .. })) {
        Some(ProjectRow::Task { id, .. }) => id.clone(),
        _ => panic!("the task"),
    };
    rig.fleet.link_task_issue(row(&task), link("2")).await.expect("linked");
    let loose = rig.fleet.add_project_task(new_task(manifest.clone(), None, "Loose")).await.expect("added");
    let loose_row = loose.loose.iter().find_map(|row| match row {
        ProjectRow::Task { id, .. } => Some(id.clone()),
        ProjectRow::Issue { .. } => None,
    });
    rig.fleet.link_task_issue(loose_row.expect("the loose task"), link("1")).await.expect("linked");

    let read = rig.fleet.list_projects().await.expect("listed");
    let at = |id: &str| read.projects.iter().position(|one| one.id == id).expect("the project");

    assert_eq!(
        issue_rows(&read, at(&named)),
        [
            ("2".to_string(), Some(IssueState::Open), Some("On the project's board".to_string())),
            ("3".to_string(), Some(IssueState::Closed), Some("Closed on the project's board".to_string())),
        ],
        "the project's rows are read through the project's tracker, listed and by reference"
    );
    assert_eq!(linked(&read, at(&named)).iter().filter(|(issue, _)| issue == "2").map(|(_, title)| title.clone()).collect::<Vec<_>>(), [Some("On the project's board".to_string())], "so is an issue linked to its task");
    assert_eq!(issue_rows(&read, at(&bare)), [("1".to_string(), Some(IssueState::Open), Some("On the repository's board".to_string()))], "a project that names none keeps its repository's");
    assert!(linked(&read, at(&bare)).contains(&("1".to_string(), Some("On the repository's board".to_string()))), "and so does a loose task");
}

#[tokio::test]
async fn a_link_in_a_project_that_names_a_tracker_is_checked_against_it_and_elsewhere_is_not() {
    let rig = two_boards();
    let (named, bare) = (rig.project(Some(project_board())).await, rig.project(None).await);
    let manifest = rig.manifest();

    let refused = rig.fleet.link_project_issue(new_issue(manifest.clone(), Some(named.clone()), "9")).await.expect_err("the tracker has none");
    assert_eq!(code_of(refused), (404, "fleet.no_such_issue".to_string()));
    let task = rig.fleet.add_project_task(new_task(manifest.clone(), Some(named.clone()), "Ship")).await.expect("added");
    let row = task.projects.iter().find(|one| one.id == named).and_then(|one| one.rows.first()).map(|row| match row {
        ProjectRow::Task { id, .. } | ProjectRow::Issue { id, .. } => id.clone(),
    });
    let refused = rig.fleet.link_task_issue(row.expect("the task"), link("9")).await.expect_err("the tracker has none");
    assert_eq!(code_of(refused).1, "fleet.no_such_issue");

    rig.fleet.link_project_issue(new_issue(manifest.clone(), Some(bare), "9")).await.expect("a project that names none does not ask");
    rig.fleet.link_project_issue(new_issue(manifest, None, "9")).await.expect("nor does a loose row");
}
