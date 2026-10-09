//! Sleep mode and an escalated Job: answered once with the owner's own act, or held.

use std::sync::Arc;

use core_model::{JobStatus, Target};

use super::{a_drone_that_answers, a_fleet_with, job_moves, stalled, Fixture};
use crate::tests::tmp::TempDir;

async fn night(fleet: &Fixture) {
    fleet.store().lock().await.begin_sleep("2000-01-01T00:00:00.000Z").unwrap();
}

async fn rows(fleet: &Fixture) -> Vec<store::SleepRow> {
    fleet.store().lock().await.sleep_rows().unwrap()
}

#[tokio::test]
async fn an_escalated_job_is_answered_once_and_recorded_as_decided() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    night(&fleet).await;
    let job = stalled(&fleet, &home).await;
    assert!(rows(&fleet).await.is_empty(), "an answerable escalation is not held");
    let fleet = Arc::new(fleet);
    Arc::clone(&fleet).sleep_pass().await.unwrap();
    let kept = rows(&fleet).await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!((kept[0].kind.as_str(), kept[0].id.as_str()), ("decided", format!("job:{}", job.as_str()).as_str()));
    let heard = fleet.the_only_slot().await.lock().await.as_ref().map(|at| at.heard().len()).unwrap();
    Arc::clone(&fleet).sleep_pass().await.unwrap();
    assert_eq!(rows(&fleet).await.len(), 1, "a Job is answered once");
    let again = fleet.the_only_slot().await.lock().await.as_ref().map(|at| at.heard().len()).unwrap();
    assert_eq!(heard, again, "the second pass sent nothing");
}

#[tokio::test]
async fn a_destructive_escalation_stays_blocked() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    night(&fleet).await;
    let job = started_destructive(&fleet, &home).await;
    let before = job_moves(&fleet, &job).await;
    let fleet = Arc::new(fleet);
    Arc::clone(&fleet).sleep_pass().await.unwrap();
    let kept = rows(&fleet).await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0].kind, "blocked");
    assert_eq!(job_moves(&fleet, &job).await, before, "nothing touched the Job");
}

#[tokio::test]
async fn with_sleep_off_no_job_is_touched() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = stalled(&fleet, &home).await;
    let before = job_moves(&fleet, &job).await;
    let fleet = Arc::new(fleet);
    Arc::clone(&fleet).sleep_pass().await.unwrap();
    assert!(rows(&fleet).await.is_empty());
    assert_eq!(job_moves(&fleet, &job).await, before);
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Escalated);
}

async fn started_destructive(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = started_titled(fleet, home, "delete the old parser tables").await;
    let record = fleet.load(&job).await.unwrap();
    fleet
        .move_job(&record, Target::Escalated(core_model::EscalationTrigger::Stalled), core_model::Actor::Fleet)
        .await
        .unwrap();
    job
}

async fn started_titled(fleet: &Fixture, home: &TempDir, title: &str) -> core_model::JobId {
    use crate::tests::admitted::dispatched;
    use crate::tests::daemon::{a_proposal, worktree_directory};
    let job = fleet.propose(a_proposal(title)).await.unwrap();
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.unwrap();
    super::settled(fleet).await;
    job.id().clone()
}
