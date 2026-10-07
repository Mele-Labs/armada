//! The three ways back from a pilot. `docs/concepts/pilot.md`, *Evidence*. #368.

use std::sync::Arc;

use core_model::{JobId, JobStatus, PilotReason, StepId, StepState, StepTarget};
use store::{AttachmentState, Holder};
use testkit::FakeWorkProduct;

use crate::adrift::Adrift;
use crate::piloting::Unpilotable;
use crate::tests::admitted::admit;
use crate::tests::daemon::{a_fleet, a_proposal};
use crate::tests::piloting::{changed, running, Fixture};
use crate::tests::tmp::TempDir;

async fn piloted(fleet: &Fixture, home: &TempDir, session: Option<&str>) -> JobId {
    let id = running(fleet, home, "a Job a person took over").await;
    fleet
        .take_over(&id, PilotReason::TakeOver, session)
        .await
        .expect("taken over");
    id
}

/// Every step of a running Job advanced, which is what attesting waits for.
async fn running_with_every_step_advanced(fleet: &Fixture, home: &TempDir) -> JobId {
    let id = running(fleet, home, "everything advanced").await;
    let job = fleet.load(&id).await.unwrap();
    let job = fleet
        .move_step(&job, &StepId::new("implement"), StepTarget::Advanced)
        .await
        .unwrap();
    let job = fleet
        .move_step(&job, &StepId::new("summarise"), StepTarget::Running)
        .await
        .unwrap();
    fleet
        .move_step(&job, &StepId::new("summarise"), StepTarget::Advanced)
        .await
        .unwrap();
    id
}

fn journal(home: &TempDir, handle: &str) -> Vec<String> {
    crate::journal::read_from(&home.path().to_string_lossy(), handle, 0)
        .notes
        .into_iter()
        .map(|note| note.msg)
        .collect()
}

#[tokio::test]
async fn submitting_runs_the_gates_and_the_job_leaves_piloted_with_no_drone() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, changed()));
    let id = piloted(&fleet, &home, Some("pilot-1")).await;

    let after = Fleet::submit_for_verification(Arc::clone(&fleet), &id)
        .await
        .expect("the gates run");

    assert_eq!(after.status(), JobStatus::Queued, "a step passed, one is left");
    assert_eq!(
        after.step(&StepId::new("implement")).map(|row| row.state()),
        Some(StepState::Advanced),
        "the gate that a Drone's work passes, passed on the person's"
    );
    assert!(after.assigned_drone().is_none(), "no Drone is put on it");
    let row = fleet.summarised(&after).await.unwrap();
    assert_eq!(row.piloted.and_then(|pilot| pilot.exit).as_deref(), Some("submitted"));
    // The Session gives the worktree back and the Job holds it again.
    let store = fleet.store().lock().await;
    let held = store.attachments_of(&Holder::session("pilot-1")).unwrap();
    assert!(
        held.iter().all(|row| row.state == AttachmentState::GivenBack),
        "{held:?}"
    );
    let job_rows = store.attachments_of(&Holder::job(id.as_str())).unwrap();
    assert!(
        job_rows
            .iter()
            .any(|row| row.kind == "slot" && row.state == AttachmentState::Standing),
        "{job_rows:?}"
    );
}

#[tokio::test]
async fn a_submission_the_gates_refuse_leaves_the_job_piloted_and_the_pilot_standing() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::untouched()));
    let id = piloted(&fleet, &home, None).await;

    let after = Fleet::submit_for_verification(Arc::clone(&fleet), &id)
        .await
        .expect("the gates ran");

    assert_eq!(after.status(), JobStatus::Piloted, "nothing changed in the worktree");
    let row = fleet.summarised(&after).await.unwrap();
    assert!(
        row.piloted.is_some_and(|pilot| pilot.exit.is_none()),
        "the person is still working"
    );
    assert!(
        journal(&home, &after.handle())
            .iter()
            .any(|line| line.contains("submitted the step for verification")),
        "the run is in the Job's log"
    );
}

#[tokio::test]
async fn submitting_a_restart_step_hands_the_worktree_to_a_fresh_drone_at_the_stopped_step() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, changed()));
    let id = running(&fleet, &home, "restart it").await;
    fleet
        .take_over(&id, PilotReason::RestartStep, None)
        .await
        .expect("taken over");

    let queued = Fleet::submit_for_verification(Arc::clone(&fleet), &id)
        .await
        .expect("handed on");

    assert_eq!(queued.status(), JobStatus::Queued);
    assert_eq!(
        queued.step(&StepId::new("implement")).map(|row| row.state()),
        Some(StepState::Stopped),
        "no gate ran: the step waits for its Drone as a restart leaves it"
    );
    admit(&fleet).await.expect("admission runs");
    assert!(
        fleet.working_on().await.contains(&id),
        "a fresh Drone is on the worktree"
    );
}

#[tokio::test]
async fn attesting_is_refused_while_a_step_has_not_advanced() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = piloted(&fleet, &home, None).await;

    let refused = fleet
        .attest_complete(&id, Some("done by hand"))
        .await
        .expect_err("a step has not advanced");

    assert!(
        matches!(
            refused,
            Adrift::CannotPilot {
                why: Unpilotable::StepsNotAdvanced { .. },
                ..
            }
        ),
        "{refused:?}"
    );
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Piloted);
}

#[tokio::test]
async fn an_attested_job_completes_recorded_as_attested_and_never_as_verified() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running_with_every_step_advanced(&fleet, &home).await;
    fleet
        .take_over(&id, PilotReason::TakeOver, Some("pilot-2"))
        .await
        .expect("taken over");

    let done = fleet
        .attest_complete(&id, Some("the plan was wrong and the outcome is right"))
        .await
        .expect("attested");

    assert_eq!(done.status(), JobStatus::CompletedSuccess);
    let row = fleet.summarised(&done).await.unwrap();
    let pilot = row.piloted.expect("the pilot is on the row");
    assert_eq!(pilot.exit.as_deref(), Some("attested"));
    assert_eq!(
        pilot.note.as_deref(),
        Some("the plan was wrong and the outcome is right")
    );
    assert!(
        journal(&home, &done.handle())
            .iter()
            .any(|line| line.contains("not verified")),
        "the log says no gate ran"
    );
    let store = fleet.store().lock().await;
    assert!(store
        .attachments_of(&Holder::session("pilot-2"))
        .unwrap()
        .iter()
        .all(|row| row.state == AttachmentState::GivenBack));
}

#[tokio::test]
async fn closing_as_superseded_is_its_own_terminal_state_and_releases_a_dependant_with_a_warning() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let upstream = piloted(&fleet, &home, None).await;
    let mut waiting = a_proposal("waits on the upstream");
    waiting.dependencies = vec![ipc::DependencyEdge {
        direction: ipc::DependencyDirection::from_wire("depends_on").expect("a direction"),
        peer: ipc::JobId::carried(upstream.as_str()),
    }];
    let dependant = fleet.propose(waiting).await.expect("a dependant");
    fleet.approve(dependant.id()).await.expect("approved");
    admit(&fleet).await.unwrap();
    assert_eq!(
        fleet.load(dependant.id()).await.unwrap().status(),
        JobStatus::Queued,
        "a piloted upstream is not finished, so the dependant waits"
    );

    let closed = fleet
        .close_as_superseded(&upstream, Some("landed by hand in another branch"))
        .await
        .expect("superseded");

    assert_eq!(closed.status(), JobStatus::Superseded);
    let row = fleet.summarised(&closed).await.unwrap();
    assert_eq!(row.piloted.and_then(|pilot| pilot.exit).as_deref(), Some("superseded"));
    admit(&fleet).await.unwrap();
    assert_eq!(
        fleet.load(dependant.id()).await.unwrap().status(),
        JobStatus::Running,
        "the dependency is released and the dependant runs"
    );
    assert!(
        journal(&home, &dependant.handle())
            .iter()
            .any(|line| line.contains("closed as superseded")),
        "and its own log says the upstream never landed as planned"
    );
}

use crate::daemon::Fleet;
