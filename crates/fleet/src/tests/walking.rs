//! A step asking to be walked, served for the person it stopped for —
//! `crate::walking`. The workflow asks and the repository answers.

use std::sync::Arc;

use ipc::ServerPhase;

use crate::tests::servers::{a_fleet_holding, a_running_job, serving, storybook_port, MANIFEST};
use crate::tests::tmp::TempDir;

/// What `tests::servers` holds, naming `storybook` as the server to walk on.
fn walking_on_storybook() -> String {
    format!("{MANIFEST}walk: storybook\n")
}

/// **The whole claim**: a walked step starts the repository's `walk` server
/// in the Job's own worktree, marked as started for a person's review, and
/// it answers on the Job's own span.
#[tokio::test]
async fn a_walked_step_starts_the_repositorys_walk_server_for_the_job() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let manifest = walking_on_storybook();
    let fleet = a_fleet_holding(&home, &events, &manifest, &manifest);
    let job = a_running_job(&fleet, &home).await;
    let port = storybook_port(&fleet, &job).await;
    let step = job.workflow().steps()[0].clone().walking(true);
    let mut watching = events.subscribe();

    let started = Arc::clone(&fleet)
        .walked_at_stop(job.clone(), &step)
        .await
        .expect("the step asks and the repository names a server")
        .expect("it starts");

    assert_eq!(started.name, "storybook");
    assert!(
        started.for_review,
        "Bridge opens it because Fleet says why it is up"
    );
    assert_eq!(started.phase, ServerPhase::Starting);
    let up = serving(&mut watching, &started.id).await;
    assert_eq!(
        up.job_id.as_ref().map(|id| id.as_str()),
        Some(job.id().as_str())
    );
    assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_ok());
    fleet.stopped_every_server().await;
}

/// A step that does not ask starts nothing, whatever the repository names.
#[tokio::test]
async fn a_step_not_walked_starts_nothing() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let manifest = walking_on_storybook();
    let fleet = a_fleet_holding(&home, &events, &manifest, &manifest);
    let job = a_running_job(&fleet, &home).await;
    let step = job.workflow().steps()[0].clone();

    assert!(Arc::clone(&fleet)
        .walked_at_stop(job, &step)
        .await
        .is_none());
}

/// **Any repository, whatever its tooling**: one that names no `walk` stops
/// for its person exactly as before, with nothing started and nothing refused.
#[tokio::test]
async fn a_repository_naming_no_walk_server_starts_nothing() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, MANIFEST, MANIFEST);
    let job = a_running_job(&fleet, &home).await;
    let step = job.workflow().steps()[0].clone().walking(true);

    assert!(Arc::clone(&fleet)
        .walked_at_stop(job, &step)
        .await
        .is_none());
}
