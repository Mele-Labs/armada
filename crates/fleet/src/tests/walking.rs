//! A step asking to be walked, served for the person it stopped for —
//! `crate::walking`. The workflow asks and the repository answers.

use std::sync::Arc;

use core_model::StepState;
use ipc::{Event, ServerPhase};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, built_then_walked, diff_evidence, note_evidence, one, worktree_directory,
};
use crate::tests::servers::{
    a_fleet_holding, a_running_job, fittings_holding, next_event, serving, storybook_port, Fixture,
    MANIFEST,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

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

/// A Fleet whose only workflow is `built_then_walked`, over the walking
/// Manifest — what a Prototype on this repository is, minus the real files.
fn a_fleet_walking(home: &TempDir, events: &api::Broadcaster) -> Arc<Fixture> {
    let manifest = walking_on_storybook();
    let mut fittings = fittings_holding(home, events, &manifest, &manifest);
    fittings.starting().workflows = one(built_then_walked());
    Arc::new(Fleet::assembled(fittings))
}

/// **The whole of it, the way it ran for the owner on 5 Oct 2026 and did
/// nothing.** A Job is made, its workflow frozen into the store and read back,
/// its Drone hands in, its Build step stops for a person — and the turn loop
/// then starts the repository's `walk` server in the Job's worktree, marked for
/// review. The two tests above hand `walked_at_stop` a step built in the test,
/// which is exactly what hid `walked` being dropped on the way to the store.
#[tokio::test]
async fn a_job_stopping_at_a_walked_step_gets_its_server_started_for_review() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_walking(&home, &events);
    let job = fleet
        .propose(a_proposal("try a stacked run beside the canvas"))
        .await
        .expect("proposed");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.expect("it dispatches");
    let mut watching = events.subscribe();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.expect("the first gate runs");
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("the Build gate runs");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "Build is a person's: {:?}",
        turned.ruled()
    );

    // What the loop does after a turn, with the Fleet it holds as an `Arc`.
    fleet.walked(&turned);

    let up = match next_event(&mut watching, |event| {
        matches!(
            event,
            Event::ServerServing(state) | Event::ServerExited(state)
                if state.job_id.as_ref().map(|id| id.as_str()) == Some(job_id.as_str())
        )
    })
    .await
    {
        Event::ServerServing(state) => state,
        Event::ServerExited(state) => panic!("it ended before serving: {state:?}"),
        _ => unreachable!("the filter admits only this Job's server"),
    };
    assert_eq!(up.name, "storybook", "the Manifest's `walk` names it");
    assert!(up.for_review, "Fleet started it for the person it stopped for");
    assert_eq!(up.phase, ServerPhase::Serving);

    // And the step is still the person's to answer: starting a server gated nothing.
    let held = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(
        held.step(&core_model::StepId::new("build".to_string()))
            .map(|row| row.state()),
        Some(StepState::AwaitingHuman)
    );
    fleet.stopped_every_server().await;
}

/// **A Job's frozen workflow keeps `walked`** — the half of the above a
/// store column can drop on its own, said once more where it is cheap to read.
#[tokio::test]
async fn a_loaded_job_still_says_its_step_asks_to_be_walked() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_walking(&home, &events);
    let job = fleet
        .propose(a_proposal("try a stacked run beside the canvas"))
        .await
        .expect("proposed");

    let loaded = fleet.load(job.id()).await.expect("the Job is there");
    let build = loaded
        .workflow()
        .step(&core_model::StepId::new("build".to_string()))
        .expect("the step");
    assert!(build.walked(), "a Job read back from the store");
}
