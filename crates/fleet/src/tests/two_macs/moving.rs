//! A Job moved out of a shared project on one Mac leaves the other Mac's list, and nothing of it is sent there to
//! say so: the other Mac reads its Jobs again when the projects it is shown change, and drops what is no longer
//! shared.

use std::sync::Arc;

use api::HostedSessions;
use ipc::SetSessionProject;

use super::{from, job_in, paired, project, shared, until, OneMac, Ticking};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_job_moved_out_of_a_shared_project_leaves_the_other_macs_list_and_nothing_of_it_crosses() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let (shop, taxes) = (project(&work.rig, "Shop").await, project(&work.rig, "Taxes").await);
    shared(&work.rig, &shop, true).await;
    let moving = job_in(&work.rig, Some(&shop)).await;
    paired(&home, &work).await;
    let on_home = moving.clone();
    until("Home lists Work's shared Job", || async { from(&home.sessions().await.1, &on_home).is_some() }).await;
    let carried = home.every_event().await;

    let moved = SetSessionProject { session_ids: vec![moving.clone()], project: Some(taxes.clone()) };
    work.rig.fleet.set_session_project(moved).await.expect("moved");

    let on_home = moving.clone();
    until("the Job leaves Home's list", || async { from(&home.sessions().await.1, &on_home).is_none() }).await;
    until("Home says the row went", || async {
        carried.lock().expect("not poisoned").iter().any(|one| one.contains("session.removed") && one.contains(moving.as_str()))
    })
    .await;
    // Let anything still on its way arrive before reading what Home was told.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Home itself made the removal. Nothing else it was handed names the Job or the private project it went to.
    let told = carried.lock().expect("not poisoned").clone();
    let about_it: Vec<&String> = told.iter().filter(|one| one.contains(moving.as_str())).collect();
    assert!(about_it.iter().all(|one| one.contains("session.removed")), "something of the Job crossed: {about_it:?}");
    assert!(told.iter().all(|one| !one.contains(taxes.as_str())), "the private project crossed");
    let (raw, _) = home.sessions().await;
    assert!(!raw.contains(moving.as_str()), "Home's list still names the Job");
}
