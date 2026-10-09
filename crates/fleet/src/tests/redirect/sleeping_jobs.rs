//! Sleep mode and an escalated Job: answered once with the owner's own act, or held.

use std::sync::Arc;

use core_model::{EscalationTrigger, JobStatus, Target};

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

async fn escalated_on(fleet: &Fixture, home: &TempDir, trigger: EscalationTrigger) -> core_model::JobId {
    if trigger == EscalationTrigger::DroneGone {
        use crate::tests::admitted::dispatched;
        use crate::tests::daemon::{a_proposal, worktree_directory};
        let job = fleet.propose(a_proposal("tidy the parser")).await.unwrap();
        worktree_directory(home, &job);
        dispatched(fleet, job.id()).await.unwrap();
        let job = job.id().clone();
        let slot = fleet.slot_of(&job).await.expect("its dispatch took a slot");
        let mut held = slot.lock().await;
        fleet.stood_down(&job, &mut held).await.expect("the Drone ends");
        drop(held);
        escalate(fleet, &job, trigger).await;
        return job;
    }
    let job = started_titled(fleet, home, "tidy the parser").await;
    escalate(fleet, &job, trigger).await;
    job
}

async fn escalate(fleet: &Fixture, job: &core_model::JobId, trigger: EscalationTrigger) {
    let mut record = fleet.load(job).await.unwrap();
    if let Some(step) = core_model::StepLevelTrigger::of(trigger) {
        let id = core_model::StepId::new("implement");
        record = fleet.move_step(&record, &id, core_model::StepTarget::Stopped(step)).await.unwrap();
    }
    fleet.move_job(&record, Target::Escalated(trigger), core_model::Actor::Fleet).await.unwrap();
}

#[tokio::test]
async fn an_unanswered_ask_is_steered_like_a_stall() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    night(&fleet).await;
    let job = escalated_on(&fleet, &home, EscalationTrigger::AskUnanswered).await;
    let fleet = Arc::new(fleet);
    Arc::clone(&fleet).sleep_pass().await.unwrap();
    let kept = rows(&fleet).await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!((kept[0].kind.as_str(), kept[0].id.as_str()), ("decided", format!("job:{}", job.as_str()).as_str()));
    assert!(!kept[0].text.starts_with("Restarted"), "{kept:?}");
    let heard = fleet.the_only_slot().await.lock().await.as_ref().map(|at| at.heard().len()).unwrap();
    assert!(heard > 0, "the Drone was told");
}

#[tokio::test]
async fn a_run_that_ended_is_restarted_once_a_night() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    night(&fleet).await;
    let job = escalated_on(&fleet, &home, EscalationTrigger::DroneGone).await;
    assert!(rows(&fleet).await.is_empty(), "a retried escalation is not held");
    let fleet = Arc::new(fleet);
    Arc::clone(&fleet).sleep_pass().await.unwrap();
    let kept = rows(&fleet).await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0].kind, "decided", "{kept:?}");
    assert!(kept[0].text.starts_with("Restarted the step"), "{kept:?}");
    assert_ne!(fleet.load(&job).await.unwrap().status(), JobStatus::Escalated, "the step restarted");
    let before = job_moves(&fleet, &job).await;
    Arc::clone(&fleet).sleep_pass().await.unwrap();
    assert_eq!(rows(&fleet).await.len(), 1, "{:?}", rows(&fleet).await);
    assert_eq!(job_moves(&fleet, &job).await, before, "not restarted a second time");
}

#[tokio::test]
async fn an_undecided_gate_is_asked_again() {
    use crate::tests::daemon::a_fleet_judged_by;
    use crate::tests::regating::{judged_then_summarised, undecided};
    use testkit::{FakeJudge, FakeWorkProduct};
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]).but_refusing("a worktree that would not read"),
        judged_then_summarised(),
        FakeJudge::with_no_objection(),
    );
    let job = undecided(&fleet, &home).await;
    night(&fleet).await;
    fleet.work().reads_now();
    let fleet = Arc::new(fleet);
    Arc::clone(&fleet).sleep_pass().await.unwrap();
    let kept = rows(&fleet).await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0].kind, "decided", "{kept:?}");
    assert!(kept[0].text.starts_with("Ran the gate again"), "{kept:?}");
    assert_ne!(fleet.load(&job).await.unwrap().status(), JobStatus::Escalated);
}

#[tokio::test]
async fn a_policy_block_stays_blocked() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    night(&fleet).await;
    let job = escalated_on(&fleet, &home, EscalationTrigger::BlockedByPolicy).await;
    let before = job_moves(&fleet, &job).await;
    let fleet = Arc::new(fleet);
    Arc::clone(&fleet).sleep_pass().await.unwrap();
    let kept = rows(&fleet).await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0].kind, "blocked");
    assert_eq!(job_moves(&fleet, &job).await, before);
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
