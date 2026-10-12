//! What `get_job` says is wrong with a Job: a Drone gone quiet, and the
//! breakages, overlaps and failed ruling the detail already holds.
//!
//! The quiet Drone is the one case with a real child and a pushed clock; the
//! rest is the pure fold over a detail.

use std::sync::Arc;
use std::time::Duration;

use api::Queries;
use core_model::Timestamp;
use ipc::mcp::{AskQuestion, AskedOption};
use ipc::{JobIssue, SharedPath, Verdict};

use crate::drones_had::SoFar;
use crate::job_issues::{issues_of, quiet_since};
use crate::silence::Liveness;
use crate::tests::planted::Held;
use crate::tests::silence::{a_drone_that_goes_quiet, one_step, started, watching, QUIET_AFTER};
use crate::tests::tmp::TempDir;

async fn issues(fleet: &super::Fixture, job: &core_model::JobId) -> Vec<JobIssue> {
    fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
        .issues
}

/// Wait until the Drone's one call has reached its transcript, or the quiet it
/// is measured from is its spawn and the case proves less than it claims.
async fn until_it_has_called(fleet: &super::Fixture, job: &core_model::JobId) {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let drones = fleet
                .list_job_drones(ipc::JobId::from(job))
                .await
                .expect("the Drones read")
                .drones;
            if drones.iter().any(|drone| drone.activity.is_some()) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("its call reached its transcript");
}

fn at(second: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-10-09T09:00:{second:02}.000Z"))
}

/// A working Drone quiet past `quiet_after` is an issue naming it and when it
/// last wrote; the same Drone a moment before is not.
#[tokio::test]
async fn a_drone_quiet_past_the_threshold_is_a_stuck_drone() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching(
        &home,
        one_step(),
        a_drone_that_goes_quiet(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
    );
    let job = started(&fleet, &home).await;
    until_it_has_called(&fleet, &job).await;

    assert_eq!(issues(&fleet, &job).await, [], "it has only just spoken");

    clock.on(QUIET_AFTER.as_secs() + 60);
    let found = issues(&fleet, &job).await;
    assert_eq!(found.len(), 1, "{found:?}");
    let JobIssue::StuckDrone { step_id, .. } = &found[0] else {
        panic!("a quiet Drone is a stuck one: {found:?}");
    };
    assert_eq!(step_id.as_str(), "implement");
}

/// A Drone with a question outstanding is waiting on a person, however long.
#[tokio::test]
async fn a_drone_waiting_on_an_answer_is_not_stuck() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching(
        &home,
        one_step(),
        a_drone_that_goes_quiet(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
    );
    let job = started(&fleet, &home).await;
    until_it_has_called(&fleet, &job).await;
    fleet
        .ask_question(
            &job,
            AskQuestion {
                question: "Should it be its own Job?".to_string(),
                options: vec![
                    AskedOption {
                        label: "Its own Job".to_string(),
                        consequence: "dispatch it first".to_string(),
                    },
                    AskedOption {
                        label: "Fold it in".to_string(),
                        consequence: "the first Job that needs it adds it".to_string(),
                    },
                ],
            },
        )
        .await
        .expect("a working Drone may ask");

    clock.on(QUIET_AFTER.as_secs() + 60);
    assert_eq!(issues(&fleet, &job).await, []);
}

/// A Drone whose run has ended is at rest, and a Drone that wrote a while ago
/// but not past the threshold is working.
#[test]
fn only_a_working_drone_past_the_threshold_is_quiet() {
    let spawned = at(0);
    let now = at(59);
    let wrote = |when: &str| SoFar {
        last_wrote: Some(ipc::Instant::carried(when)),
        ..SoFar::default()
    };
    let limit = Duration::from_secs(30);

    assert_eq!(
        quiet_since(&wrote("2026-10-09T09:00:10.000Z"), &spawned, &now, limit),
        Some(ipc::Instant::carried("2026-10-09T09:00:10.000Z"))
    );
    assert_eq!(
        quiet_since(&wrote("2026-10-09T09:00:40.000Z"), &spawned, &now, limit),
        None,
        "nineteen seconds is inside thirty"
    );
    let resting = SoFar {
        at_rest_since: Some(ipc::Instant::carried("2026-10-09T09:00:10.000Z")),
        ..wrote("2026-10-09T09:00:10.000Z")
    };
    assert_eq!(quiet_since(&resting, &spawned, &now, limit), None, "at rest");
    assert_eq!(
        quiet_since(&SoFar::default(), &spawned, &now, limit),
        Some(ipc::Instant::from(&spawned)),
        "a Drone that has written nothing is quiet since it was spawned"
    );
}

/// Breakages, overlaps and a failed ruling each appear once, a Drone first.
#[tokio::test]
async fn breakages_overlaps_and_a_failed_ruling_each_appear_once() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching(
        &home,
        one_step(),
        a_drone_that_goes_quiet(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
    );
    let job = started(&fleet, &home).await;
    let mut detail = fleet.get_job(ipc::JobId::from(&job)).await.expect("reads");

    let broken = |test: &str| ipc::ClaimedBreakage {
        check: "test".to_string(),
        test: test.to_string(),
        failure: "assertion failed".to_string(),
        fix: ipc::JobId::carried("01FIX"),
        fix_title: "Fix it".to_string(),
        reported_by: ipc::JobId::carried("01REPORTER"),
        reported_by_title: None,
        waiting: Vec::new(),
        held_off: Vec::new(),
    };
    detail.breakages = vec![broken("a::b"), broken("a::b"), broken("c::d")];
    let overlapping = ipc::ScopeOverlap {
        job_id: ipc::JobId::carried("01OTHER"),
        title: "Another".to_string(),
        status: ipc::JobStatus::from(core_model::JobStatus::Running),
        paths: vec![SharedPath {
            path: "crates/fleet".to_string(),
            this_step: None,
            other_step: None,
        }],
    };
    detail.write_scope_overlaps = Some(vec![overlapping.clone(), overlapping]);
    detail.steps[0].last_verdict = Some(Verdict {
        attempt: 2,
        named: "failed".to_string(),
        trigger: Some("check_failed".to_string()),
    });
    let step_id = detail.steps[0].step_id.clone();
    let stuck = JobIssue::StuckDrone {
        drone_id: ipc::DroneId::carried("drone-1"),
        step_id: step_id.clone(),
        quiet_since: ipc::Instant::carried("2026-10-09T09:00:00.000Z"),
    };

    assert_eq!(
        issues_of(&detail, Some(stuck.clone())),
        [
            stuck,
            JobIssue::FailedVerdict {
                step_id,
                attempt: 2,
                trigger: Some("check_failed".to_string()),
            },
            JobIssue::Breakage {
                check: "test".to_string(),
                test: "a::b".to_string()
            },
            JobIssue::Breakage {
                check: "test".to_string(),
                test: "c::d".to_string()
            },
            JobIssue::WriteScopeOverlap {
                job_id: ipc::JobId::carried("01OTHER")
            },
        ]
    );

    detail.steps[0].last_verdict = Some(Verdict {
        attempt: 3,
        named: "passed".to_string(),
        trigger: None,
    });
    detail.breakages.clear();
    detail.write_scope_overlaps = Some(Vec::new());
    assert_eq!(issues_of(&detail, None), [], "a Job with nothing wrong");
}
