//! A shared project changed on its copy is changed on the Mac that keeps it, and the copy shows that Mac's answer:
//! a rename and its values at once where the Mac answers, queued and shown at once while it sleeps, and a delete
//! that takes the project away on both.

use std::sync::Arc;

use api::Projects as _;
use ipc::{EditProject, MacLink, Projects, ValuesSet};

use super::{paired, project, shared, until, OneMac, Ticking};

/// The name of project `id` as `mac` lists it, and the Mac it is held from. None where it is not listed.
async fn named(mac: &OneMac, id: &str) -> Option<(String, Option<String>)> {
    let (_, projects): (String, Projects) = mac.projects().await;
    projects.projects.into_iter().find(|one| one.id == id).map(|one| (one.name, one.mac))
}

async fn draft_on(mac: &OneMac, id: &str) -> bool {
    let (_, projects): (String, Projects) = mac.projects().await;
    let values = projects.projects.into_iter().find(|one| one.id == id).and_then(|one| one.values);
    values.and_then(|one| one.pr_mode) == Some(core_model::PrMode::Draft.into())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_change_to_a_shared_project_made_on_its_copy_reaches_the_mac_that_keeps_it_and_comes_back() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let open = project(&home.rig, "Shop").await;
    shared(&home.rig, &open, true).await;
    paired(&home, &work).await;
    let mut home = home;
    let home_id = home.id.clone();
    let on_home = |name: &str| Some((name.to_string(), None));
    let on_work = |name: &str| Some((name.to_string(), Some(home_id.clone())));
    until("Shop reaches Work", || async { named(&work, &open).await == on_work("Shop") }).await;

    // 1. A rename and a value set on Work's copy are made on Home, and the copy is what Home answers.
    let draft = ValuesSet { pr_mode: Some(core_model::PrMode::Draft.into()), ..ValuesSet::default() };
    let edit = EditProject { name: Some("Shop floor".into()), values: Some(draft), ..EditProject::default() };
    let answered = work.rig.fleet.edit_project(open.clone(), edit).await.expect("sent to Home");
    let shown = answered.projects.iter().find(|one| one.id == open).expect("the copy");
    assert_eq!((shown.name.as_str(), shown.mac.as_deref()), ("Shop floor", Some(home_id.as_str())), "Home's answer is the copy");
    assert_eq!(named(&home, &open).await, on_home("Shop floor"), "Home renamed its own project");
    assert!(draft_on(&home, &open).await, "and set the value in its own file");
    assert!(draft_on(&work, &open).await, "the copy shows the value");

    // 2. While Home sleeps the rename waits on Work, shows on its copy at once, and is made on Home when it speaks.
    home.stop().await;
    until("Home shows asleep on Work", || async { matches!(work.mac(&home_id).await.and_then(|mac| mac.link), Some(MacLink::Asleep { .. })) }).await;
    let edit = EditProject { name: Some("Shop, upstairs".into()), ..EditProject::default() };
    work.rig.fleet.edit_project(open.clone(), edit).await.expect("queued");
    assert_eq!(named(&work, &open).await, on_work("Shop, upstairs"), "shown on the copy at once");
    assert_eq!(named(&home, &open).await, on_home("Shop floor"), "Home has not heard it yet");
    assert!(!work.rig.fleet.store().lock().await.queued_project_acts(None).expect("reads").is_empty(), "queued for Home");
    home.restart();
    until("the rename is made on Home", || async { named(&home, &open).await == on_home("Shop, upstairs") }).await;
    until("Work's queue is sent", || async { work.rig.fleet.store().lock().await.queued_project_acts(None).expect("reads").is_empty() }).await;
    until("and the copy is Home's answer", || async { named(&work, &open).await == on_work("Shop, upstairs") }).await;

    // 3. Deleted on the copy, the project goes on Home, and the copy goes with it.
    work.rig.fleet.delete_project(open.clone()).await.expect("sent to Home");
    until("Home deleted its project", || async { named(&home, &open).await.is_none() }).await;
    until("the copy left Work", || async { named(&work, &open).await.is_none() }).await;
}
