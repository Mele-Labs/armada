//! A step asking to be walked, served for the person it stopped for —
//! `crate::walking`. The workflow asks and the repository answers.

use std::sync::Arc;

use core_model::StepState;
use ipc::{Event, ServerPhase};

use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, diff_evidence, note_evidence, one, worktree_directory};
use crate::tests::servers::{
    a_fleet_holding, a_running_job, fittings_holding, next_event, serving, storybook_port, MANIFEST,
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

/// `implement`, then `build`, which holds for a person and asks to be walked —
/// through the parser, so `walked` travels the way a real file's does.
fn implement_then_a_walked_build() -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-walked.yml"),
        "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\n\
         steps:\n  - id: implement\n    label: \"Implement\"\n    \
         evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      \
         - type: diff_nonempty\n    delivers: false\n    advance_gate: auto\n  - \
         id: build\n    label: \"Build\"\n    \
         evidence: {submitted: {type: facts_note}, walked: true}\n    \
         delivers: false\n    advance_gate: human_always\n",
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &crate::tests::daemon::manifest())
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

/// **Through the store, not around it.** The cases above hand-build a step with
/// `.walking(true)`, so a store that dropped the flag passed them. This one
/// proposes a Job and reads it back, as `walked` does.
#[tokio::test]
async fn a_step_loaded_from_the_store_is_still_walked() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let manifest = walking_on_storybook();
    let mut fittings = fittings_holding(&home, &events, &manifest, &manifest);
    fittings.starting().workflows = one(implement_then_a_walked_build());
    let fleet = Arc::new(crate::daemon::Fleet::assembled(fittings));
    let job = fleet
        .propose(a_proposal("walk the build"))
        .await
        .expect("a Job proposed");

    let loaded = fleet.load(job.id()).await.expect("the Job is there");

    let build = core_model::StepId::new("build".to_string());
    assert!(loaded.workflow().step(&build).expect("build").walked());
}

/// **End to end**: a Job's step holds for a person after a real store round
/// trip, and `walked` starts the repository's `walk` server for review.
#[tokio::test]
async fn a_walked_step_held_for_review_serves_after_a_store_round_trip() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let manifest = walking_on_storybook();
    let mut fittings = fittings_holding(&home, &events, &manifest, &manifest);
    fittings.starting().workflows = one(implement_then_a_walked_build());
    let fleet = Arc::new(crate::daemon::Fleet::assembled(fittings));
    let job = fleet
        .propose(a_proposal("walk the build"))
        .await
        .expect("a Job proposed");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.expect("it dispatches");
    let mut watching = events.subscribe();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.expect("the first gate runs");
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("the walked step's gate runs");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "the walked step is a person's: {:?}",
        turned.ruled()
    );

    fleet.walked(&turned);
    let up = match next_event(&mut watching, |event| {
        matches!(event, Event::ServerServing(state) if state.job_id.as_ref().map(|id| id.as_str()) == Some(job_id.as_str()))
    })
    .await
    {
        Event::ServerServing(state) => state,
        other => panic!("not a server serving: {other:?}"),
    };

    assert_eq!(up.name, "storybook", "the Manifest's `walk`");
    assert!(up.for_review);
    let held = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(
        held.current_step().expect("a step").state(),
        StepState::AwaitingHuman,
        "serving does not decide the step"
    );
    fleet.stopped_every_server().await;
}
