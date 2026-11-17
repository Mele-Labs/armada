//! Two projects of one name made one, from either side: this Mac's own kept and the copy given up, or the copy kept
//! and this Mac's own given up. Each Mac changes only what it holds, so the other Mac's part is sent as an act on its
//! project, queued while it sleeps, and the pressing Mac lists the one project from the press.

use std::sync::Arc;
use std::time::Duration;

use api::{OtherMacs, Projects};
use ipc::{
    EditProject, ManifestId, MergeProjects, NewProject, NewProjectTask, Project, ProjectAct, ProjectHolder, ProjectMember, ProjectPlace, ProjectRow,
    ProjectTaskState, RunPlace, ValuesSet,
};

use super::other_mac_projects::{act, hold, listed, state_in, task_row, theirs, titles_of, Link, WORK};
use super::session_host::{rig_placing, Rig};
use super::share::{job_in, project, shared};
use crate::phone::PhoneSwitch;
use crate::tests::tmp::TempDir;

const THEIRS: &str = "p-shop-work";

fn merge(kept: &str, gone: &str, from_gone: &[&str]) -> MergeProjects {
    MergeProjects { kept: kept.to_string(), gone: gone.to_string(), from_gone: from_gone.iter().map(|one| one.to_string()).collect() }
}

async fn task_in(rig: &Rig, project: &str, title: &str) {
    let task = NewProjectTask { manifest_id: rig.manifest.clone(), project: Some(project.to_string()), title: title.into(), notes: String::new() };
    rig.fleet.add_project_task(task).await.expect("added");
}

fn row_ids(projects: &ipc::Projects, project: &str) -> Vec<String> {
    let one = projects.projects.iter().find(|one| one.id == project).expect("the project");
    one.rows
        .iter()
        .map(|row| match row {
            ProjectRow::Task { id, .. } | ProjectRow::Issue { id, .. } => id.clone(),
        })
        .collect()
}

fn values(auto_merge: &str, delivery: Option<&str>) -> ValuesSet {
    ValuesSet { auto_merge: Some(auto_merge.to_string()), delivery: delivery.map(str::to_string), ..ValuesSet::default() }
}

/// What the holder says once its project has gone: no project of its own that matters here.
fn nothing_left() -> ipc::Projects {
    ipc::Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new() }
}

/// A copy of Work's "Shop", with a task of each state.
fn work_shop(rig: &Rig) -> Project {
    let mut held = task_row("t-2", "Held there", &rig.manifest);
    if let ProjectRow::Task { state, held_by, .. } = &mut held {
        *state = ProjectTaskState::Held;
        *held_by = Some(ProjectHolder { session_id: "01WORKJOB".into(), title: Some("A Job there".into()) });
    }
    let mut done = task_row("t-3", "Done there", &rig.manifest);
    let mut asked = task_row("t-4", "Asked there", &rig.manifest);
    if let ProjectRow::Task { state, .. } = &mut done {
        *state = ProjectTaskState::Done;
    }
    if let ProjectRow::Task { state, .. } = &mut asked {
        *state = ProjectTaskState::Asked;
    }
    let mut copy = theirs(THEIRS, "Shop", vec![task_row("t-1", "Open there", &rig.manifest), held, done, asked]);
    copy.members = vec![ProjectMember { manifest_id: ManifestId::carried("other-repo"), folder: None }];
    copy.values = Some(values("always", Some("ask")));
    copy
}

/// This Mac's "Shop", shared, with one task and the values it sets.
async fn own_shop(rig: &Rig) -> String {
    let shop = project(rig, "Shop").await;
    task_in(rig, &shop, "Mine").await;
    let edit = EditProject { values: Some(values("never", None)), shared_with_macs: Some(true), ..Default::default() };
    rig.fleet.edit_project(shop.clone(), edit).await.expect("edited");
    shop
}

fn linked(rig: &Rig) -> Arc<Link> {
    let link = Link::new();
    rig.fleet.switching_phone_with(Arc::clone(&link) as Arc<dyn PhoneSwitch>);
    link
}

async fn queue_empty(rig: &Rig) -> bool {
    for _ in 0..100 {
        if rig.fleet.store().lock().await.queued_project_acts(None).expect("reads").is_empty() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test]
async fn keeping_this_macs_project_takes_the_copys_tasks_members_and_values_and_tells_the_other_mac_to_give_its_up() {
    let rig = rig_placing(None);
    let shop = own_shop(&rig).await;
    let link = linked(&rig);
    let theirs_here = work_shop(&rig);
    hold(&rig, vec![theirs_here.clone()]).await;
    // A Job of this Mac that was started in the copy.
    let job = job_in(&rig, None).await;
    {
        let mut store = rig.fleet.store().lock().await;
        let mut hosting = store.hosting(job.as_str()).expect("reads").expect("hosted");
        hosting.project = Some(THEIRS.to_string());
        store.keep_hosting(&hosting).expect("kept");
    }
    link.answers(200, &nothing_left());

    let merged = rig.fleet.merge_projects(merge(&shop, THEIRS, &["auto_merge"])).await.expect("merged");

    assert_eq!(merged.projects.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [shop.as_str()], "one project listed");
    assert_eq!(titles_of(&merged, &shop), ["Mine", "Open there", "Held there", "Done there", "Asked there"]);
    let kept = merged.projects.iter().find(|one| one.id == shop).expect("kept");
    let state = |row: &str| state_in(&merged, row);
    assert_eq!((state("t-1"), state("t-2"), state("t-3"), state("t-4")), (ProjectTaskState::Open, ProjectTaskState::Open, ProjectTaskState::Done, ProjectTaskState::Asked), "a held task comes over open");
    assert!(kept.rows.iter().all(|row| !matches!(row, ProjectRow::Task { held_by: Some(_), .. })), "no Job on the other Mac holds one here");
    assert!(kept.members.iter().any(|one| one.manifest_id.as_str() == "other-repo"), "members are joined: {:?}", kept.members);
    assert!(kept.members.iter().any(|one| one.manifest_id == rig.manifest), "and the kept ones stay");
    let set = kept.values.clone().expect("values");
    assert_eq!((set.auto_merge.as_deref(), set.delivery.as_deref()), (Some("always"), Some("ask")), "the key named, and the key only the gone one set");
    assert_eq!(rig.fleet.project_of_job(job.as_str()).await.as_deref(), Some(shop.as_str()), "a Job in the gone project moves");

    let calls = link.calls();
    assert_eq!(calls.len(), 1);
    let ProjectAct::GiveUp { gone, into, absorbed } = &calls[0].act else { panic!("{:?}", calls[0]) };
    assert_eq!((gone.as_str(), into.as_str()), (THEIRS, shop.as_str()));
    assert_eq!(absorbed, &["t-1", "t-2", "t-3", "t-4"], "what it was given, so only the rest is sent back");
}

#[tokio::test]
async fn a_merge_with_a_mac_that_sleeps_lists_one_project_at_once_and_the_mac_is_told_when_it_wakes() {
    let rig = rig_placing(None);
    let shop = own_shop(&rig).await;
    let link = linked(&rig);
    hold(&rig, vec![work_shop(&rig)]).await;
    link.sleeps();

    let merged = rig.fleet.merge_projects(merge(&shop, THEIRS, &[])).await.expect("merged");

    assert_eq!(merged.projects.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [shop.as_str()], "never the duplicate");
    assert_eq!(titles_of(&merged, &shop).len(), 5, "both sets of tasks");
    assert_eq!(titles_of(&listed(&rig).await, &shop).len(), 5, "and so does a list of them afterwards");
    let queued = rig.fleet.store().lock().await.queued_project_acts(Some(WORK)).expect("reads");
    assert_eq!(queued.len(), 1, "one act for the Mac that sleeps");

    // It wakes, and says it still has its project: the queue is sent behind that, and the copy stays hidden until then.
    link.answers(200, &nothing_left());
    hold(&rig, vec![work_shop(&rig)]).await;
    assert!(queue_empty(&rig).await, "sent when the Mac spoke");
    assert!(matches!(link.calls().last().expect("sent").act, ProjectAct::GiveUp { .. }));
    for _ in 0..100 {
        if listed(&rig).await.projects.iter().filter(|one| one.name == "Shop").count() == 1 && rig.fleet.copies_held().await.expect("held").is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(listed(&rig).await.projects.iter().filter(|one| one.name == "Shop").count(), 1);
}

#[tokio::test]
async fn keeping_the_copy_sends_this_macs_tasks_and_values_and_then_moves_its_jobs_and_deletes_its_project_and_synced_file() {
    let rig = rig_placing(None);
    let sync = TempDir::new();
    let place = ProjectPlace { place: RunPlace::Synced, synced_folder: Some(sync.path().to_string_lossy().into_owned()), cloud: None };
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let made = rig.fleet.add_project(NewProject { name: "Shop".into(), members, place: Some(place), values: None }).await.expect("added");
    let shop = made.projects.into_iter().find(|one| one.name == "Shop").expect("shop").id;
    task_in(&rig, &shop, "Mine").await;
    shared(&rig, &shop, true).await;
    let file = sync.path().join("projects").join(format!("{shop}.yml"));
    assert!(file.exists(), "its file is in the synced folder");
    let job = job_in(&rig, Some(&shop)).await;
    let link = linked(&rig);
    let copy = work_shop(&rig);
    hold(&rig, vec![copy.clone()]).await;
    let mut said = copy.clone();
    said.rows.push(task_row("t-mine", "Mine", &rig.manifest));
    link.answers(200, &ipc::Projects { projects: vec![said], loose: Vec::new(), left_out: Vec::new() });

    let merged = rig.fleet.merge_projects(merge(THEIRS, &shop, &["delivery"])).await.expect("merged");

    let calls = link.calls();
    assert_eq!(calls.len(), 1, "one act, the snapshot");
    let ProjectAct::Absorb { kept, gone, from_gone } = &calls[0].act else { panic!("{:?}", calls[0]) };
    assert_eq!((kept.as_str(), gone.id.as_str(), gone.name.as_str(), from_gone.as_slice()), (THEIRS, shop.as_str(), "Shop", ["delivery".to_string()].as_slice()));
    assert_eq!(gone.rows.len(), 1, "its tasks, under their own ids");
    assert!(gone.members.iter().any(|one| one.manifest_id == rig.manifest));

    assert_eq!(merged.projects.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [THEIRS], "one project listed");
    assert_eq!(titles_of(&merged, THEIRS).len(), 5);
    assert!(rig.fleet.store().lock().await.projects().expect("reads").projects.is_empty(), "this Mac's own project is deleted with its tasks");
    assert!(!file.exists(), "and its file in the synced folder");
    assert_eq!(rig.fleet.project_of_job(job.as_str()).await.as_deref(), Some(THEIRS), "its Jobs are in the kept project");
}

#[tokio::test]
async fn keeping_the_copy_while_its_mac_sleeps_lists_one_project_and_finishes_here_when_the_mac_answers() {
    let rig = rig_placing(None);
    let shop = own_shop(&rig).await;
    let job = job_in(&rig, Some(&shop)).await;
    let link = linked(&rig);
    hold(&rig, vec![theirs(THEIRS, "Shop", vec![task_row("t-1", "Theirs", &rig.manifest)])]).await;
    link.sleeps();

    let merged = rig.fleet.merge_projects(merge(THEIRS, &shop, &[])).await.expect("merged");

    assert_eq!(merged.projects.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [THEIRS], "this Mac's own is not listed");
    assert_eq!(titles_of(&merged, THEIRS), ["Theirs", "Mine"], "the copy shows both sets of tasks");
    assert_eq!(rig.fleet.store().lock().await.projects().expect("reads").projects.len(), 1, "still kept here until the Mac has them");
    assert_eq!(rig.fleet.project_of_job(job.as_str()).await.as_deref(), Some(shop.as_str()));

    let mut said = theirs(THEIRS, "Shop", vec![task_row("t-1", "Theirs", &rig.manifest), task_row("t-mine", "Mine", &rig.manifest)]);
    said.shared_with_macs = true;
    link.answers(200, &ipc::Projects { projects: vec![said.clone()], loose: Vec::new(), left_out: Vec::new() });
    hold(&rig, vec![said]).await;
    assert!(queue_empty(&rig).await);
    for _ in 0..100 {
        if rig.fleet.store().lock().await.projects().expect("reads").projects.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(rig.fleet.store().lock().await.projects().expect("reads").projects.is_empty(), "given up once the Mac answered");
    assert_eq!(rig.fleet.project_of_job(job.as_str()).await.as_deref(), Some(THEIRS));
    assert_eq!(listed(&rig).await.projects.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [THEIRS]);
}

#[tokio::test]
async fn an_absorb_replayed_adds_nothing_twice_and_only_a_shared_project_takes_one() {
    let rig = rig_placing(None);
    let shop = own_shop(&rig).await;
    let gone = work_shop(&rig);
    let absorb = |gone: &Project| act(ProjectAct::Absorb { kept: shop.clone(), gone: gone.clone(), from_gone: vec!["auto_merge".into()] }, Some(7));

    let once = rig.fleet.project_act_from_mac(absorb(&gone)).await.expect("absorbed");
    let titles = titles_of(&once, &shop);
    assert_eq!(titles, ["Mine", "Open there", "Held there", "Done there", "Asked there"]);
    let before = rig.fleet.store().lock().await.projects().expect("reads");
    let twice = rig.fleet.project_act_from_mac(absorb(&gone)).await.expect("absorbed again");
    assert_eq!(titles_of(&twice, &shop), titles, "a replay adds no row");
    assert_eq!(rig.fleet.store().lock().await.projects().expect("reads"), before, "and changes nothing else kept");
    assert_eq!(twice.projects[0].members.len(), once.projects[0].members.len(), "nor a member");
    assert_eq!(twice.projects[0].values, once.projects[0].values, "nor a value");
    assert_eq!(row_ids(&twice, &shop).len(), 5);

    // Turned off, it is refused as a project that does not exist.
    shared(&rig, &shop, false).await;
    let refused = rig.fleet.project_act_from_mac(absorb(&gone)).await.expect_err("unshared");
    assert_eq!((refused.status(), refused.error().code.as_str()), (404, "fleet.no_such_project"));
}

#[tokio::test]
async fn a_task_added_to_the_gone_project_after_the_snapshot_is_sent_to_the_kept_mac_by_the_give_up() {
    let rig = rig_placing(None);
    let shop = own_shop(&rig).await;
    task_in(&rig, &shop, "Added after").await;
    let job = job_in(&rig, Some(&shop)).await;
    let link = linked(&rig);
    // This Mac holds a copy of Work's project, which is the one kept.
    hold(&rig, vec![theirs(THEIRS, "Shop", vec![task_row("t-1", "Theirs", &rig.manifest)])]).await;
    link.answers(200, &theirs_after());
    let rows = row_ids(&listed(&rig).await, &shop);
    assert_eq!(rows.len(), 2);

    // Work sends its GiveUp: it was given the first task only.
    let gave = act(ProjectAct::GiveUp { gone: shop.clone(), into: THEIRS.to_string(), absorbed: vec![rows[0].clone()] }, Some(9));
    let answered = rig.fleet.project_act_from_mac(gave).await.expect("given up");

    assert!(answered.projects.is_empty(), "nothing of it stays: {answered:?}");
    let calls = link.calls();
    assert_eq!(calls.len(), 1, "the task it was not given goes to the Mac that keeps the project");
    let ProjectAct::AddProjectTask { task, id } = &calls[0].act else { panic!("{:?}", calls[0]) };
    assert_eq!((task.title.as_str(), task.project.as_deref(), id.as_deref()), ("Added after", Some(THEIRS), Some(rows[1].as_str())));
    assert!(rig.fleet.store().lock().await.projects().expect("reads").projects.is_empty(), "its project and tasks are gone");
    assert_eq!(rig.fleet.project_of_job(job.as_str()).await.as_deref(), Some(THEIRS));
    // A give up for a project this Mac does not share is refused as one it does not have.
    let again = act(ProjectAct::GiveUp { gone: shop, into: THEIRS.to_string(), absorbed: Vec::new() }, None);
    assert_eq!(rig.fleet.project_act_from_mac(again).await.expect_err("gone").status(), 404);
}

fn theirs_after() -> ipc::Projects {
    ipc::Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new() }
}

#[tokio::test]
async fn a_merge_is_refused_for_a_kept_project_not_shared_names_that_differ_and_two_that_are_not_one_and_a_copy() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    let yard = project(&rig, "Yard").await;
    let mut yard_copy = theirs("p-yard-work", "Yard", Vec::new());
    yard_copy.shared_with_macs = true;
    let mut also = theirs("p-shop-two", "Shop", Vec::new());
    also.shared_with_macs = true;
    hold(&rig, vec![theirs(THEIRS, "Shop", vec![task_row("t-1", "Theirs", &rig.manifest)]), yard_copy, also]).await;
    let before = (listed(&rig).await, rig.fleet.store().lock().await.projects().expect("reads"));
    let refused = |said: Result<ipc::Projects, api::Refusal>| {
        let refused = said.expect_err("refused");
        (refused.status(), refused.error().code.clone(), refused.error().message.clone())
    };

    let unshared = refused(rig.fleet.merge_projects(merge(&shop, THEIRS, &[])).await);
    assert_eq!((unshared.0, unshared.1.as_str()), (422, "fleet.merge_kept_not_shared"));
    assert!(!unshared.2.contains('\u{2014}') && !unshared.2.contains('-'), "plain words, no dashes: {}", unshared.2);

    shared(&rig, &shop, true).await;
    let differ = refused(rig.fleet.merge_projects(merge(&shop, "p-yard-work", &[])).await);
    assert_eq!((differ.0, differ.1.as_str()), (422, "fleet.merge_names_differ"));
    let two_copies = refused(rig.fleet.merge_projects(merge(THEIRS, "p-shop-two", &[])).await);
    assert_eq!((two_copies.0, two_copies.1.as_str()), (422, "fleet.merge_not_a_pair"));
    let two_own = refused(rig.fleet.merge_projects(merge(&shop, &yard, &[])).await);
    assert_eq!((two_own.0, two_own.1.as_str()), (422, "fleet.merge_not_a_pair"));
    let one = refused(rig.fleet.merge_projects(merge(&shop, &shop, &[])).await);
    assert_eq!(one.1, "fleet.merge_not_a_pair");
    let missing = refused(rig.fleet.merge_projects(merge(&shop, "p-nope", &[])).await);
    assert_eq!((missing.0, missing.1.as_str()), (404, "fleet.no_such_project"));

    shared(&rig, &shop, false).await;
    assert_eq!((listed(&rig).await.projects.len(), rig.fleet.store().lock().await.projects().expect("reads").projects.len()), (before.0.projects.len(), before.1.projects.len()));
    assert_eq!(titles_of(&listed(&rig).await, THEIRS), ["Theirs"], "nothing refused changed anything");
}
