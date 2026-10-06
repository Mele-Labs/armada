//! A Job paused gives its slot back and comes back to whichever is free.
//! `docs/concepts/job.md`, *Pausing*.
//!
//! The pool's git rules, and the work being at the branch's tip, are
//! `adapters`' tests. These are Fleet's half: what it refuses, what it records,
//! and that a paused Job's gate, steps and review rows stay as they were.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::SlotParkRefused;
use axum::http::StatusCode;
use axum::Router;
use config::{Manifest, Reloads};
use core_model::{Job, JobId, JobStatus, QueuedReason, StepState};
use ipc::{JobSummary, RunId, WireError};
use testkit::FakeWorkProduct;

use crate::adrift::Adrift;
use crate::tests::admitted::admit;
use crate::tests::daemon::{a_fleet, a_proposal, fittings, worktree_directory};
use crate::tests::http::call;
use crate::tests::reviewing::{a_fleet_reviewing_the_first_step, at_the_gate};
use crate::tests::tmp::TempDir;

type Fixture = crate::daemon::Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn root(home: &TempDir) -> String {
    home.path().to_string_lossy().to_string()
}

fn changed() -> FakeWorkProduct {
    FakeWorkProduct::changed(&["src/log.rs"])
}

/// Propose, approve and admit a Job.
async fn running(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("proposed");
    worktree_directory(home, &job);
    fleet.approve(job.id()).await.expect("approved");
    admit(fleet).await.expect("admission runs");
    job.id().clone()
}

/// A pool of one: every slot but the first is an agent's.
fn a_pool_of_one(home: &TempDir) -> Fixture {
    let fleet = a_fleet(home, changed());
    for n in 2..=8 {
        fleet.vcs().hold_slot(&root(home), n, "an agent's session");
    }
    fleet
}

/// What the Job's steps read, which a pause must leave exactly as it was.
fn steps_of(job: &Job) -> Vec<(String, StepState)> {
    job.steps()
        .iter()
        .map(|row| (row.step_id().as_str().to_string(), row.state()))
        .collect()
}

#[tokio::test]
async fn a_running_job_paused_frees_its_slot_and_a_resume_continues_the_same_step_in_it() {
    let home = TempDir::new();
    let fleet = a_pool_of_one(&home);
    let job = running(&fleet, &home, "fix the reader").await;
    let before = fleet.load(&job).await.unwrap();
    assert_eq!(before.worktree_slot(), Some(1));
    let step = before.current_step_id().cloned().expect("a step");

    let paused = fleet.pause_job(&job).await.expect("a running Job pauses");

    assert_eq!(paused.status(), JobStatus::Queued, "no status of its own");
    assert!(paused.pause().is_some());
    assert_eq!(paused.worktree_slot(), None);
    assert_eq!(
        paused.step(&step).map(|row| row.state()),
        Some(StepState::Stopped),
        "its Drone is gone, so its step stopped where it stood"
    );
    assert_eq!(
        fleet.queued_reason(&paused).await.unwrap().reason,
        Some(QueuedReason::Paused)
    );
    assert_eq!(fleet.vcs().slot_holders(&root(&home))[0], None);
    assert_eq!(
        fleet.vcs().parked_slots(),
        vec![(1, job.as_str().to_string())]
    );
    assert!(
        !fleet.working_on().await.contains(&job),
        "nothing is working it"
    );
    admit(&fleet).await.unwrap();
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Queued,
        "admission does not start a paused Job"
    );

    // Another Job takes the freed slot, and the resume finds the pool full.
    let other = running(&fleet, &home, "another change").await;
    assert_eq!(fleet.load(&other).await.unwrap().worktree_slot(), Some(1));
    let waiting = fleet.resume_job(&job).await.expect("a resume is taken");
    assert!(waiting.pause().is_some_and(|pause| pause.resuming));
    assert_eq!(
        fleet.queued_reason(&waiting).await.unwrap().reason,
        Some(QueuedReason::WaitingOnResources),
        "it reads waiting for a slot, not paused"
    );
    admit(&fleet).await.unwrap();
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Queued);

    fleet.kill_job(&other).await.expect("the other ends");
    // An agent takes the freed slot before the turn: the Drone bound has room
    // and only the pool is short, which is what the label has to read.
    fleet.vcs().hold_slot(&root(&home), 1, "an agent's session");
    let still = fleet.load(&job).await.unwrap();
    assert_eq!(
        fleet.queued_reason(&still).await.unwrap().reason,
        Some(QueuedReason::WaitingOnResources)
    );
    admit(&fleet).await.unwrap();
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Queued);
    fleet.vcs().free_slot(&root(&home), 1);
    admit(&fleet).await.unwrap();

    let back = fleet.load(&job).await.unwrap();
    assert_eq!(back.status(), JobStatus::Running);
    assert_eq!(back.worktree_slot(), Some(1));
    assert!(back.pause().is_none(), "the marker is lifted");
    assert_eq!(back.current_step_id(), Some(&step), "the same step");
    assert_eq!(
        back.step(&step).map(|row| row.state()),
        Some(StepState::Running)
    );
    assert_eq!(
        fleet.vcs().reseated_slots(),
        vec![(
            format!("armada/{}", back.handle()),
            1,
            job.as_str().to_string()
        )],
        "its own branch, leased at its tip"
    );
}

#[tokio::test]
async fn a_job_at_a_gate_keeps_it_through_a_pause_and_a_resume() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, changed());
    let job = at_the_gate(&fleet, &home).await;
    let before = fleet.load(&job).await.unwrap();
    assert_eq!(before.worktree_slot(), Some(1));

    let paused = fleet.pause_job(&job).await.expect("a gate Job pauses");

    assert_eq!(
        paused.status(),
        JobStatus::AwaitingReview,
        "it keeps its gate"
    );
    assert_eq!(steps_of(&paused), steps_of(&before));
    assert_eq!(paused.worktree_slot(), None);
    assert!(paused.pause().is_some());
    assert_eq!(fleet.vcs().slot_holders(&root(&home))[0], None);

    let back = fleet.resume_job(&job).await.expect("a gate Job resumes");

    assert_eq!(back.status(), JobStatus::AwaitingReview);
    assert_eq!(steps_of(&back), steps_of(&before));
    assert_eq!(back.worktree_slot(), Some(1));
    assert!(back.pause().is_none());
    let read = fleet.load(&job).await.unwrap();
    assert_eq!(read, back, "what was written is what reads back");
    fleet
        .approve_review(&job)
        .await
        .expect("and a person's act on it works again");
}

#[tokio::test]
async fn a_pause_the_record_refuses_changes_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, changed());
    let job = at_the_gate(&fleet, &home).await;
    let before = fleet.load(&job).await.unwrap();
    let held = fleet.vcs().slot_holders(&root(&home));

    // Checks in flight on the worktree.
    let rerun = fleet.rechecking().take(&job).expect("a re-run is out");
    let refused = fleet.pause_job(&job).await.expect_err("while Checks run");
    assert!(
        matches!(refused, Adrift::ChecksRunningAgain { .. }),
        "{refused:?}"
    );
    drop(rerun);

    // git refusing the park, for each reason the pool has.
    for why in [
        SlotParkRefused::OnNoBranch,
        SlotParkRefused::OnTheBase(String::from("main")),
        SlotParkRefused::OnAnotherBranch {
            leased: String::from("armada/a"),
            on: String::from("armada/b"),
        },
        SlotParkRefused::HeldByAnother(String::from("somebody")),
        SlotParkRefused::Busy,
        SlotParkRefused::Vcs(String::from("git said no")),
    ] {
        fleet.vcs().refuse_next_park(why.clone());
        match fleet.pause_job(&job).await {
            Err(Adrift::CannotPark { refused, .. }) => assert_eq!(refused, why),
            other => panic!("the pool refused with {why:?}: {other:?}"),
        }
        let kept = fleet.load(&job).await.unwrap();
        assert_eq!(kept, before, "{why:?} changed the record");
        assert_eq!(fleet.vcs().slot_holders(&root(&home)), held);
    }

    // A slot another holds is known before anything is touched.
    fleet.vcs().free_slot(&root(&home), 1);
    fleet.vcs().hold_slot(&root(&home), 1, "somebody");
    assert!(matches!(
        fleet.pause_job(&job).await,
        Err(Adrift::CannotPark {
            refused: SlotParkRefused::HeldByAnother(_),
            ..
        })
    ));
    fleet.vcs().free_slot(&root(&home), 1);
    fleet.vcs().hold_slot(&root(&home), 1, job.as_str());

    // Not paused, and a status that cannot hold one.
    assert!(matches!(
        fleet.resume_job(&job).await,
        Err(Adrift::NotPaused { .. })
    ));
    let proposed = fleet.propose(a_proposal("not yet approved")).await.unwrap();
    assert!(matches!(
        fleet.pause_job(proposed.id()).await,
        Err(Adrift::NotPausable {
            status: JobStatus::AwaitingApproval,
            ..
        })
    ));

    // Already paused.
    fleet.pause_job(&job).await.expect("it pauses");
    assert!(matches!(
        fleet.pause_job(&job).await,
        Err(Adrift::AlreadyPaused { .. })
    ));
}

#[tokio::test]
async fn a_running_jobs_drone_is_gone_when_the_pool_refuses_and_the_job_is_escalated() {
    let home = TempDir::new();
    let fleet = a_pool_of_one(&home);
    let job = running(&fleet, &home, "fix the reader").await;
    fleet.vcs().refuse_next_park(SlotParkRefused::OnNoBranch);

    let refused = fleet.pause_job(&job).await.expect_err("the pool refuses");

    assert!(matches!(refused, Adrift::CannotPark { .. }));
    let left = fleet.load(&job).await.unwrap();
    assert_eq!(
        left.status(),
        JobStatus::Escalated,
        "a Drone cannot be handed back"
    );
    assert_eq!(
        fleet
            .last_reason(&job)
            .await
            .unwrap()
            .and_then(|why| why.as_wire()),
        Some("would_not_start")
    );
    assert!(left.pause().is_none());
    assert_eq!(left.worktree_slot(), Some(1), "it still holds its slot");
}

#[tokio::test]
async fn a_persons_act_on_a_paused_job_is_refused_as_paused() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, changed());
    let job = at_the_gate(&fleet, &home).await;
    fleet.pause_job(&job).await.expect("it pauses");

    let refused = fleet.approve_review(&job).await.expect_err("no worktree");

    assert!(matches!(refused, Adrift::Paused { .. }), "{refused:?}");
    match fleet.refusal(refused) {
        api::Refusal::IllegalMove(error) => assert_eq!(error.code, "fleet.paused"),
        other => panic!("a 409 was owed: {other:?}"),
    }
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::AwaitingReview,
        "and nothing moved"
    );
}

#[tokio::test]
async fn a_resume_with_a_full_pool_waits_and_the_turn_that_finds_a_slot_free_leases_it() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, changed());
    for n in 2..=8 {
        fleet.vcs().hold_slot(&root(&home), n, "an agent's session");
    }
    let job = at_the_gate(&fleet, &home).await;
    fleet.pause_job(&job).await.expect("it pauses");
    fleet.vcs().hold_slot(&root(&home), 1, "an agent's session");

    let waiting = fleet.resume_job(&job).await.expect("a resume is taken");

    assert_eq!(waiting.status(), JobStatus::AwaitingReview);
    assert!(waiting.pause().is_some_and(|pause| pause.resuming));
    assert_eq!(waiting.worktree_slot(), None);
    let turned = fleet.turn().await.expect("a turn");
    assert!(turned.reseated.is_empty(), "no slot is free");

    fleet.vcs().free_slot(&root(&home), 1);
    let turned = fleet.turn().await.expect("a turn");

    assert_eq!(turned.reseated, vec![job.clone()]);
    let back = fleet.load(&job).await.unwrap();
    assert_eq!(back.worktree_slot(), Some(1));
    assert!(back.pause().is_none());
    assert_eq!(back.status(), JobStatus::AwaitingReview);
}

#[tokio::test]
async fn a_paused_job_can_still_be_killed() {
    let home = TempDir::new();
    let fleet = a_pool_of_one(&home);
    let queued = running(&fleet, &home, "a running one").await;
    fleet.pause_job(&queued).await.expect("it pauses");
    let killed = fleet
        .kill_job(&queued)
        .await
        .expect("a paused Job is killed");
    assert_eq!(killed.status(), JobStatus::Killed);

    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, changed());
    let gated = at_the_gate(&fleet, &home).await;
    fleet.pause_job(&gated).await.expect("it pauses");
    let killed = fleet
        .kill_job(&gated)
        .await
        .expect("a paused gate Job is killed");
    assert_eq!(killed.status(), JobStatus::Killed);
}

#[tokio::test]
async fn a_dependent_stays_blocked_on_a_paused_upstream() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let upstream = running(&fleet, &home, "the upstream").await;
    fleet.pause_job(&upstream).await.expect("it pauses");
    let mut proposal = a_proposal("the dependent");
    proposal.dependencies = vec![ipc::DependencyEdge {
        direction: ipc::DependencyDirection::from_wire("depends_on").expect("a direction"),
        peer: ipc::JobId::from(&upstream),
    }];
    let dependent = fleet.propose(proposal).await.expect("a coupled Job");
    worktree_directory(&home, &dependent);
    fleet.approve(dependent.id()).await.expect("approved");

    admit(&fleet).await.unwrap();

    let waiting = fleet.load(dependent.id()).await.unwrap();
    assert_eq!(waiting.status(), JobStatus::Queued);
    assert_eq!(
        fleet.queued_reason(&waiting).await.unwrap().reason,
        Some(QueuedReason::BlockedByDependency)
    );
}

/// A Fleet over an `armada.yml` that can be saved again under it.
fn over_a_manifest(home: &TempDir) -> (Fixture, std::path::PathBuf, Reloads) {
    let dir = home.path().join("manifest");
    std::fs::create_dir_all(&dir).expect("a directory for the file");
    let file = dir.join("armada.yml");
    std::fs::write(&file, "version: 1\nid: 01FIXTUREMANIFEST\n").expect("the file");
    let (manifest, reloads) = Manifest::reloadable(&file).expect("the file loads");
    let mut fitted = fittings(home, changed());
    fitted.starting().manifest = manifest;
    fitted.noticing = crate::noticing::Noticing::every(Duration::ZERO);
    (crate::daemon::Fleet::assembled(fitted), file, reloads)
}

#[tokio::test]
async fn paused_reads_before_frozen_and_a_resume_in_a_frozen_repository_reads_frozen() {
    let home = TempDir::new();
    let (fleet, file, reloads) = over_a_manifest(&home);
    let job = running(&fleet, &home, "fix the reader").await;
    fleet.pause_job(&job).await.expect("it pauses");
    std::fs::write(&file, "version: 1\nid: 01FIXTUREMANIFEST\nfreeze: true\n").expect("the save");
    reloads.reread().expect("the save loads");

    let paused = fleet.load(&job).await.unwrap();
    assert_eq!(
        fleet.queued_reason(&paused).await.unwrap().reason,
        Some(QueuedReason::Paused),
        "paused reads before frozen"
    );

    let resumed = fleet.resume_job(&job).await.expect("a resume is taken");
    assert_eq!(resumed.status(), JobStatus::Queued);
    assert_eq!(
        fleet.queued_reason(&resumed).await.unwrap().reason,
        Some(QueuedReason::Frozen)
    );
    fleet.turn().await.expect("a turn");
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Queued);
}

// ------------------------------------------------------------- the wire

fn served(fleet: &Arc<Fixture>) -> Router {
    api::router(api::Served::sharing(
        Arc::clone(fleet),
        RunId::carried("01RUN"),
        fleet.events(),
    ))
}

async fn acted(app: &Router, job: &JobId, act: &str) -> (StatusCode, Vec<u8>) {
    call(app, "POST", &format!("/jobs/{}/{act}", job.as_str()), "").await
}

fn summary(body: &[u8]) -> JobSummary {
    ipc::decode("a Job summary", body).expect("a JobSummary")
}

fn code(body: &[u8]) -> String {
    ipc::decode::<WireError>("a wire error", body)
        .expect("an error body")
        .code
}

async fn log_of(fleet: &Fixture, home: &TempDir, job: &JobId) -> Vec<String> {
    let handle = fleet.load(job).await.unwrap().handle();
    crate::journal::read_from(&root(home), &handle, 0)
        .notes
        .into_iter()
        .map(|note| note.msg)
        .collect()
}

/// Every `job.paused` or `job.resumed` the stream carries, as the rows they
/// replaced, tagged with the kind.
fn pause_rows(delivered: Vec<ipc::Delivered>) -> Vec<(&'static str, JobSummary)> {
    delivered
        .into_iter()
        .filter_map(|one| match one.event {
            ipc::Event::JobPaused(paused) => Some(("job.paused", paused.job)),
            ipc::Event::JobResumed(resumed) => Some(("job.resumed", resumed.job)),
            _ => None,
        })
        .collect()
}

/// Everything published so far: a read that finds nothing within a beat has
/// found the end.
async fn heard(watching: &mut api::Subscription) -> Vec<ipc::Delivered> {
    let mut heard = Vec::new();
    while let Ok(Some(api::Next::Send(one))) =
        tokio::time::timeout(Duration::from_millis(200), watching.next()).await
    {
        heard.push(one);
    }
    heard
}

#[tokio::test]
async fn a_gate_job_paused_over_the_wire_keeps_its_status_and_says_so_to_every_client() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_reviewing_the_first_step(&home, changed()));
    let job = at_the_gate(&fleet, &home).await;
    let app = served(&fleet);
    let mut watching = fleet.events().subscribe();

    let (status, body) = acted(&app, &job, "park_job").await;

    assert_eq!(status, StatusCode::OK);
    let paused = summary(&body);
    assert_eq!(
        paused.status.as_wire(),
        "awaiting_review",
        "the real status"
    );
    assert_eq!(paused.queued_reason, None, "a gate Job is not queued");
    let marker = paused.paused.as_ref().expect("the marker rides beside it");
    assert_eq!((marker.by.as_str(), marker.resuming), ("person", false));
    let rows = pause_rows(heard(&mut watching).await);
    assert_eq!(rows.len(), 1, "one event, though no status moved");
    assert_eq!(rows[0].0, "job.paused");
    assert_eq!(rows[0].1.paused, paused.paused);
    let log = log_of(&fleet, &home, &job).await;
    assert!(
        log.iter().any(|line| line.contains("was paused")),
        "one line in the Job's log: {log:?}"
    );

    let (status, body) = acted(&app, &job, "resume_job").await;

    assert_eq!(status, StatusCode::OK);
    let back = summary(&body);
    assert_eq!(back.status.as_wire(), "awaiting_review");
    assert_eq!(back.paused, None, "the marker is lifted");
    let rows = pause_rows(heard(&mut watching).await);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "job.resumed");
    assert_eq!(rows[0].1.paused, None);
    let log = log_of(&fleet, &home, &job).await;
    assert!(
        log.iter().any(|line| line.contains("was resumed")),
        "{log:?}"
    );
}

#[tokio::test]
async fn a_running_job_paused_over_the_wire_reads_queued_for_a_pause() {
    let home = TempDir::new();
    let fleet = Arc::new(a_pool_of_one(&home));
    let job = running(&fleet, &home, "fix the reader").await;
    let app = served(&fleet);

    let (status, body) = acted(&app, &job, "park_job").await;

    assert_eq!(status, StatusCode::OK);
    let paused = summary(&body);
    assert_eq!(paused.status.as_wire(), "queued");
    assert_eq!(
        paused.queued_reason.map(|reason| reason.as_wire()),
        Some("paused")
    );
    assert!(paused.paused.is_some());

    // A queued Job goes into admission's line, still marked until a slot is
    // leased, and no longer reads paused.
    let (_, body) = acted(&app, &job, "resume_job").await;
    let resumed = summary(&body);
    assert!(resumed.paused.is_some_and(|marker| marker.resuming));
    assert_ne!(
        resumed.queued_reason.map(|reason| reason.as_wire()),
        Some("paused")
    );
}

#[tokio::test]
async fn each_refusal_over_the_wire_is_a_409_with_its_own_code() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_reviewing_the_first_step(&home, changed()));
    let job = at_the_gate(&fleet, &home).await;
    let app = served(&fleet);
    let refused = |(status, body): (StatusCode, Vec<u8>), expected: &str| {
        assert_eq!(status, StatusCode::CONFLICT, "{expected}");
        assert_eq!(code(&body), expected);
    };

    refused(acted(&app, &job, "resume_job").await, "fleet.not_paused");

    let proposed = fleet.propose(a_proposal("not yet approved")).await.unwrap();
    refused(
        acted(&app, proposed.id(), "park_job").await,
        "fleet.not_pausable",
    );

    let rerun = fleet.rechecking().take(&job).expect("a re-run is out");
    refused(acted(&app, &job, "park_job").await, "fleet.checks_running");
    drop(rerun);

    fleet.vcs().refuse_next_park(SlotParkRefused::OnNoBranch);
    let (status, body) = acted(&app, &job, "park_job").await;
    refused((status, body.clone()), "fleet.pause_refused");
    let error: WireError = ipc::decode("a wire error", &body).unwrap();
    assert!(
        error.message.contains("nothing changed"),
        "it carries the pool's reason: {}",
        error.message
    );

    assert_eq!(acted(&app, &job, "park_job").await.0, StatusCode::OK);
    refused(acted(&app, &job, "park_job").await, "fleet.already_paused");
    refused(acted(&app, &job, "approve_review").await, "fleet.paused");
}
