//! A copy of another Mac's project changed here: its name, members, values, share switch and deletion are sent to the
//! Mac that keeps it, shown on the copy at once while that Mac sleeps; its place and its store stay that Mac's. And the
//! reads of a copy answer here, from what it carries and this Mac's own layers.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, OtherMacs, Projects};
use ipc::{EditProject, OtherMacProjects, ProjectAct, ProjectActFromMac, ProjectMember, ProjectPlace, SetProjectStore, SetProjectWorkflow, ValueLayer, ValuesSet};

use super::other_mac_projects::{hold, listed, task_row, theirs, Link, WORK};
use super::session_host::rig_placing;
use super::share::{job_in, project, shared};
use crate::phone::PhoneSwitch;

fn draft() -> ValuesSet {
    ValuesSet { pr_mode: Some(core_model::PrMode::Draft.into()), ..ValuesSet::default() }
}

#[tokio::test]
async fn an_edit_of_a_copy_shows_at_once_waits_for_its_mac_and_only_its_place_and_store_are_refused() {
    let rig = rig_placing(None);
    let link = Link::new();
    rig.fleet.switching_phone_with(Arc::clone(&link) as Arc<dyn PhoneSwitch>);
    hold(&rig, vec![theirs("p-notes", "Notes", vec![task_row("t-1", "Call the plumber", &rig.manifest)])]).await;
    link.sleeps();
    let copy = |projects: &ipc::Projects| projects.projects.iter().find(|one| one.id == "p-notes").cloned();

    // Name, members and values show on the copy at once, as the Mac that keeps it will write them.
    let member = ProjectMember { manifest_id: rig.manifest.clone(), folder: None };
    let edit = EditProject { name: Some("  Notes, home ".into()), members: Some(vec![member.clone()]), values: Some(draft()), ..EditProject::default() };
    let shown = copy(&rig.fleet.edit_project("p-notes".into(), edit).await.expect("queued")).expect("the copy");
    assert_eq!((shown.name.as_str(), shown.mac.as_deref()), ("Notes, home", Some(WORK)));
    assert_eq!((shown.members, shown.values), (vec![member], Some(draft())));

    // A workflow switched off works from the copy's values as shown, the edit still waiting included.
    let off = SetProjectWorkflow { workflow: "ship".into(), on: false };
    let shown = copy(&rig.fleet.set_project_workflow("p-notes".into(), off).await.expect("queued")).expect("the copy");
    assert_eq!(shown.values, Some(ValuesSet { workflows_off: vec!["ship".into()], ..draft() }));

    // Refused here, before anything waits: a blank name, any edit that moves the file, and the store.
    let blank = EditProject { name: Some("  ".into()), ..EditProject::default() };
    assert_eq!(rig.fleet.edit_project("p-notes".into(), blank).await.expect_err("blank").status(), 422);
    let moved = EditProject { name: Some("Moved".into()), place: Some(ProjectPlace::default()), ..EditProject::default() };
    let refused = rig.fleet.edit_project("p-notes".into(), moved).await.expect_err("the place is Work's");
    assert_eq!((refused.status(), refused.error().code.as_str()), (409, "fleet.other_mac_project_held"));
    assert_eq!(refused.error().message, format!("Only Mac {WORK} can choose where this project's file is kept, since that Mac writes it."));
    let refused = rig.fleet.set_project_store("p-notes".into(), SetProjectStore { store: None }).await.expect_err("the store is Work's");
    assert_eq!((refused.status(), refused.error().code.as_str()), (409, "fleet.other_mac_project_held"));
    assert_eq!(refused.error().message, format!("Only Mac {WORK} can set this project's store, since the join and the deploy run there with its sign-in."));
    assert_eq!(copy(&listed(&rig).await).expect("the copy").name, "Notes, home", "the refused edit changed nothing, its name included");

    // The share switch shows at once, and a delete takes the copy away at once.
    let off = EditProject { shared_with_macs: Some(false), ..EditProject::default() };
    assert!(!copy(&rig.fleet.edit_project("p-notes".into(), off).await.expect("queued")).expect("the copy").shared_with_macs);
    assert!(copy(&rig.fleet.delete_project("p-notes".into()).await.expect("queued")).is_none(), "gone at once");
    assert_eq!(rig.fleet.store().lock().await.queued_project_acts(Some(WORK)).expect("reads").len(), 4);

    // Work speaks: each is sent, oldest first, and the copy is what Work last said.
    link.answers(200, &ipc::Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new(), left_behind: Vec::new() });
    let sent_before = link.calls().len();
    Arc::clone(&rig.fleet).hold_other_mac_projects(OtherMacProjects { mac: WORK.into(), projects: Vec::new() }).await.expect("held");
    for _ in 0..100 {
        if rig.fleet.store().lock().await.queued_project_acts(None).expect("reads").is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let calls = link.calls();
    let kinds: Vec<&ProjectAct> = calls[sent_before..].iter().map(|one| &one.act).collect();
    assert_eq!(kinds.len(), 4, "{kinds:?}");
    let ProjectAct::EditProject { project, edit } = kinds[0] else { panic!("{:?}", kinds[0]) };
    assert_eq!((project.as_str(), edit.name.as_deref(), edit.place.as_ref()), ("p-notes", Some("  Notes, home "), None));
    assert!(matches!(kinds[1], ProjectAct::EditProject { edit, .. } if edit.values.as_ref().is_some_and(|one| one.workflows_off == ["ship"])));
    assert!(matches!(kinds[2], ProjectAct::EditProject { edit, .. } if edit.shared_with_macs == Some(false)));
    assert!(matches!(kinds[3], ProjectAct::DeleteProject { project } if project == "p-notes"));
    assert!(copy(&listed(&rig).await).is_none());
}

#[tokio::test]
async fn reads_of_a_copy_answer_from_what_it_carries_over_this_macs_own_layers() {
    let rig = rig_placing(None);
    let member = ProjectMember { manifest_id: rig.manifest.clone(), folder: None };
    let notes = ipc::Project { members: vec![member], values: Some(ValuesSet { workflows_off: vec!["ship".into()], ..draft() }), ..theirs("p-notes", "Notes", Vec::new()) };
    hold(&rig, vec![notes]).await;

    let values = rig.fleet.get_values(Some("p-notes".into()), None).await.expect("a copy's values answer");
    assert_eq!((values.pr_mode.value.as_deref(), values.pr_mode.from), (Some("draft"), ValueLayer::Project));
    let in_repository = rig.fleet.get_values(Some("p-notes".into()), Some(rig.manifest.clone())).await.expect("for a repository it lists");
    assert_eq!(in_repository.pr_mode.from, ValueLayer::Project);

    // Its issues are its tracker's, read here: it names none, so it has none of its own, and is not missing.
    let issues = rig.fleet.list_project_issues("p-notes".into(), false).await.expect_err("no tracker");
    assert_eq!((issues.status(), issues.error().code.as_str()), (422, "fleet.project_no_tracker"));
    let claims = rig.fleet.get_project_claims("p-notes".into()).await.expect("a copy's claims answer");
    assert!(!claims.store, "it names no store");
    assert!(rig.fleet.get_project_store("p-notes".into()).await.expect("its store setting answers").store.is_none());
    assert_eq!(rig.fleet.get_project_claims("nope".into()).await.expect_err("none").status(), 404, "a project that is nowhere is missing");
}

#[tokio::test]
async fn the_mac_that_keeps_a_shared_project_runs_an_edit_and_a_delete_sent_from_its_copy_but_never_a_place() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    let edit = |edit: EditProject| ProjectActFromMac { act: ProjectAct::EditProject { project: shop.clone(), edit }, at: Some(1) };
    let rename = EditProject { name: Some("Shop floor".into()), values: Some(draft()), ..EditProject::default() };
    assert_eq!(rig.fleet.project_act_from_mac(edit(rename.clone())).await.expect_err("unshared").status(), 404, "as a project that does not exist");

    shared(&rig, &shop, true).await;
    let answered = rig.fleet.project_act_from_mac(edit(rename)).await.expect("made here");
    let made = answered.projects.iter().find(|one| one.id == shop).expect("answered");
    assert_eq!((made.name.as_str(), made.values.clone()), ("Shop floor", Some(draft())));
    let moved = EditProject { name: Some("Moved".into()), place: Some(ProjectPlace::default()), ..EditProject::default() };
    let refused = rig.fleet.project_act_from_mac(edit(moved)).await.expect_err("the place is chosen here");
    assert_eq!((refused.status(), refused.error().code.as_str()), (422, "fleet.project_place_refused"));
    let kept = listed(&rig).await.projects.into_iter().find(|one| one.id == shop).expect("kept");
    assert_eq!(kept.name, "Shop floor", "refused whole");

    let deleted = rig.fleet.project_act_from_mac(ProjectActFromMac { act: ProjectAct::DeleteProject { project: shop.clone() }, at: None }).await.expect("deleted");
    assert!(deleted.projects.iter().all(|one| one.id != shop));
    assert!(listed(&rig).await.projects.iter().all(|one| one.id != shop));
}

#[tokio::test]
async fn a_job_starts_in_a_copy_that_lists_its_repository_and_is_refused_by_one_that_does_not() {
    let rig = rig_placing(None);
    let member = ProjectMember { manifest_id: rig.manifest.clone(), folder: None };
    hold(&rig, vec![ipc::Project { members: vec![member], ..theirs("p-notes", "Notes", Vec::new()) }, theirs("p-elsewhere", "Elsewhere", Vec::new())]).await;

    let job = job_in(&rig, Some("p-notes")).await;
    let hosted = rig.fleet.store().lock().await.hosting(job.as_str()).expect("read").expect("hosted").project;
    assert_eq!(hosted.as_deref(), Some("p-notes"));
    assert!(rig.fleet.job_shared(job.as_str()).await, "shared with the Mac that keeps the copy");

    let start = ipc::StartSession {
        project: Some("p-elsewhere".into()),
        manifest_id: rig.manifest.clone(),
        title: Some("Build the pane".to_string()),
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
        workflow: None,
    };
    let refused = rig.fleet.start_session(start).await.expect_err("the copy does not list the repository");
    assert_eq!((refused.status(), refused.error().code.as_str()), (422, "fleet.project_copy_not_member"));
}
