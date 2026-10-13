//! A Commander's repair of a Job that broke before its first step, and of a
//! repair that is out when Fleet restarts. `#2170`.
//!
//! The Drone is `tests::job_repair`'s shell. A Job here fails its preparation
//! (`setup.requires` names `/bin/ls ready`, which fails until a file called
//! `ready` is in the worktree) and the repair Drone is the one that makes it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::Change;
use config::Manifest;
use core_model::{Actor, EscalationTrigger, Job, JobId, JobStatus, StepState};
use ipc::Breakage;
use store::Moved;
use testkit::{FakeHarness, FakeJudge, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::helm::Hosting;
use crate::tests::admitted::{dispatched, started};
use crate::tests::commander_conversation::{said, settled, woke_rows, Hosted, Scripted};
use crate::tests::daemon::{a_proposal, diff_evidence, fitted_with, one, worktree_directory};
use crate::tests::job_repair::{
    a_fleet, escalated, gated, harness, log_of, repairs, send, until, until_restarted, IMPLEMENT,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// A Fleet whose Manifest requires a file the repair Drone is to write.
fn a_fleet_requiring_ready(
    home: &TempDir,
    host: &Arc<Scripted>,
    harness: FakeHarness,
) -> Arc<Hosted> {
    a_fleet_requiring_ready_and(home, host, harness, "")
}

/// The same, with more Manifest after `setup`.
fn a_fleet_requiring_ready_and(
    home: &TempDir,
    host: &Arc<Scripted>,
    harness: FakeHarness,
    more: &str,
) -> Arc<Hosted> {
    let mut fitted = fitted_with(home, FakeWorkProduct::inherited(&[]), harness);
    fitted.starting().workflows = one(gated());
    fitted.starting().manifest = Manifest::parse(
        Path::new("armada.yml"),
        &format!(
            "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  bootstrap:\n    run: /bin/ls ready\n\
             setup:\n  requires: [bootstrap]\n{more}"
        ),
    )
    .expect("a manifest that requires one command");
    fitted.judge = Arc::new(FakeJudge::that_fails("no model is asked about a repair"));
    Arc::new(Fleet::assembled(fitted).hosting_helm_on(Arc::clone(host) as Arc<dyn Hosting>))
}

/// A Job whose preparation failed: escalated `not_prepared`, no step entered,
/// no Drone, and the worktree on disk.
async fn unprepared(fleet: &Arc<Hosted>, home: &TempDir) -> (JobId, PathBuf) {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(home, &job);
    let refused = dispatched(fleet, job.id())
        .await
        .expect_err("a worktree that could not be prepared is not dispatched");
    assert!(matches!(refused, Adrift::NotPrepared { .. }), "{refused:?}");
    let record = fleet.load(job.id()).await.expect("the Job reads");
    assert_eq!(record.status(), JobStatus::Escalated);
    assert!(record.steps().iter().all(|step| step.state() == StepState::NotStarted));
    let worktree = fleet
        .surviving_worktree(&record)
        .expect("the worktree is on disk");
    (job.id().clone(), PathBuf::from(worktree.path()))
}

/// The last move of the Job's own, and who signed it.
async fn last_mover(fleet: &Hosted, job: &JobId) -> Actor {
    fleet
        .store()
        .lock()
        .await
        .events_for(job)
        .unwrap()
        .iter()
        .rev()
        .find_map(|event| match event.moved() {
            Moved::Job { .. } => Some(event.actor()),
            Moved::Step { .. } | Moved::Drone { .. } => None,
        })
        .expect("the Job moved")
}

/// **A conflict found while the worktree was being prepared gets a repair, and
/// the Job goes back to its first step.** The repair's file is that step's
/// work, so its own Check rules on it; the same step with nothing written fails.
#[tokio::test]
async fn a_conflict_found_in_preparation_gets_a_repair_and_the_first_steps_gate_sees_it() {
    for repaired in [true, false] {
        let home = TempDir::new();
        let host = Scripted::answering();
        let fleet = a_fleet_requiring_ready(
            &home,
            &host,
            harness("touch ready; rm -f .unmerged", true),
        );
        let (job, worktree) = unprepared(&fleet, &home).await;
        std::fs::write(worktree.join("lead.ts"), "export const lead = 1;\n").unwrap();
        fleet
            .work()
            .unmerged_until_gone(&worktree.join(".unmerged"), &["lead.ts"]);

        let sent = send(&fleet, &job, Breakage::MergeConflict).await.expect("sent");
        // Written after the answer, as the Drone writes: the step's baseline is
        // the worktree as the repair found it.
        if repaired {
            fleet
                .work()
                .wrote(&[("src/parse.rs", Change::Modified)]);
        }
        until_restarted(&fleet, &job).await;

        assert_eq!(sent.step_id.as_str(), IMPLEMENT, "the first step");
        assert_eq!(repairs(&fleet).len(), 1);
        let back = fleet.load(&job).await.unwrap();
        assert_eq!(back.status(), JobStatus::Queued, "back at the first step, not the owner's");
        assert_eq!(last_mover(&fleet, &job).await, Actor::Commander);
        let log = log_of(&home, &fleet, &job).await;
        assert!(log.contains("sent a repair Drone onto the Job's branch"), "{log}");

        started(&fleet, &job).await.expect("the first step's Drone is on it");
        let running = fleet.load(&job).await.unwrap();
        assert_eq!(
            running.step(&core_model::StepId::new(IMPLEMENT)).unwrap().state(),
            StepState::Running
        );
        submitted_by_the_one(&fleet, diff_evidence()).await.expect("handed in");
        let turned = fleet.turn().await.expect("the gate ruled");

        let advanced = turned.ruled().is_some_and(|ruling| ruling.advanced());
        assert_eq!(advanced, repaired, "the gate saw the repair's commit: {repaired}");
    }
}

/// **A Job that never had a Drone is no Job a repair answers, unless its
/// preparation is what failed.** The Drone cannot edit the Manifest, would
/// meet the same refusal from the same harness, and would add its load to a
/// machine that has none to spare.
#[tokio::test]
async fn the_kinds_that_are_the_manifests_or_the_machines_are_still_refused() {
    let wont_start = |home: &TempDir, host: &Arc<Scripted>| {
        let mut fitted = fitted_with(
            home,
            FakeWorkProduct::inherited(&[]),
            FakeHarness::refusing("no agent binary on this machine"),
        );
        fitted.starting().workflows = one(gated());
        Arc::new(Fleet::assembled(fitted).hosting_helm_on(Arc::clone(host) as Arc<dyn Hosting>))
    };
    let not_configurable = |home: &TempDir, host: &Arc<Scripted>| {
        let mut fitted = fitted_with(home, FakeWorkProduct::inherited(&[]), harness("true", true));
        fitted.starting().workflows = one(gated());
        fitted.host.mcp_config = String::from("etc/armada/mcp.json");
        Arc::new(Fleet::assembled(fitted).hosting_helm_on(Arc::clone(host) as Arc<dyn Hosting>))
    };
    for (made, trigger) in [
        (&wont_start as &dyn Fn(&TempDir, &Arc<Scripted>) -> Arc<Hosted>, EscalationTrigger::WouldNotStart),
        (&not_configurable, EscalationTrigger::NotConfigurable),
    ] {
        let home = TempDir::new();
        let host = Scripted::answering();
        let fleet = made(&home, &host);
        let job = fleet.propose(a_proposal("a Job no Drone can run")).await.unwrap();
        worktree_directory(&home, &job);
        dispatched(&fleet, job.id()).await.expect_err("no Drone ran");
        let record: Job = fleet.load(job.id()).await.unwrap();
        assert_eq!(record.status(), JobStatus::Escalated);
        assert_eq!(
            fleet.last_reason(job.id()).await.unwrap(),
            Some(core_model::TransitionReason::Escalation(trigger.clone()))
        );

        for broke in [Breakage::Dependencies, Breakage::Worktree, Breakage::MergeConflict] {
            let refused = send(&fleet, job.id(), broke).await.expect_err("not a repair's");
            assert!(
                refused.error().message.contains("without stopping a step"),
                "{}: {}",
                trigger.as_wire(),
                refused.error().message
            );
        }
        assert!(repairs(&fleet).is_empty(), "no Drone was spent on {}", trigger.as_wire());
    }
}

/// **The hold on a repair is kept, so a Fleet that restarts does not leave the
/// Job held with nothing running.** The next Fleet finds the repair gone with
/// the process that read it, and treats it as one that handed nothing in: the
/// Commander is woken, once, and that counts toward the loop bound.
#[tokio::test]
async fn a_fleet_restarted_with_a_repair_out_wakes_the_commander_and_releases_the_job() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let go = home.path().join("go");
    let waiting = format!("while [ ! -f {} ]; do sleep 0.02; done", go.display());
    let first = a_fleet(&home, &host, harness(&waiting, true));
    let job = escalated(&first, &home).await;
    send(&first, &job, Breakage::Dependencies).await.expect("sent");
    assert!(first.repairing().holds(&job));

    let host_after = Scripted::answering();
    let second = a_fleet(&home, &host_after, harness("true", true));
    second.reconcile().await.expect("the boot read");
    assert!(second.repairing().holds(&job), "the hold survived the restart");
    let refused = second.restart_step(&job, None).await.expect_err("held until it is dealt with");
    assert!(refused.to_string().contains("repair Drone is on"), "{refused}");

    let turned = second.turn().await.expect("a turn");
    second.woken(&turned).await;
    settled(&second, &job).await;

    assert!(!second.repairing().holds(&job), "and it is given back with the wake");
    assert_eq!(
        woke_rows(&said(&second, &job).await),
        ["The repair Drone on step implement ended with nothing handed in"]
    );
    assert_eq!(host_after.carried().len(), 1, "the Commander is running again");
    assert_eq!(second.load(&job).await.unwrap().status(), JobStatus::Escalated);
    assert!(
        second.store().lock().await.repairs_out().unwrap().is_empty(),
        "and the row is gone"
    );
    let noted = second
        .store()
        .lock()
        .await
        .commander_wakes_noted(&job, "repair_handed_nothing_in", IMPLEMENT)
        .unwrap();
    assert_eq!(noted, 1, "it counts toward the loop bound");
    let log = log_of(&home, &second, &job).await;
    assert!(log.contains("Fleet restarted while the repair Drone was out"), "{log}");

    // And what it does with the wake is a repair like any other.
    send(&second, &job, Breakage::Dependencies).await.expect("a second repair is not refused");
    std::fs::write(&go, "").unwrap();
    until("the second repair handed in", || async {
        second.load(&job).await.unwrap().status() != JobStatus::Escalated
    })
    .await;
}

/// A repair that finished before Fleet stopped leaves no row, so the next boot
/// finds nothing to wake.
#[tokio::test]
async fn a_repair_that_ended_leaves_nothing_for_the_next_boot() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let first = a_fleet(&home, &host, harness("true", true));
    let job = escalated(&first, &home).await;
    send(&first, &job, Breakage::Dependencies).await.expect("sent");
    until_restarted(&first, &job).await;
    until("the row was given back", || async {
        first.store().lock().await.repairs_out().unwrap().is_empty()
    })
    .await;

    let host_after = Scripted::answering();
    let second = a_fleet(&home, &host_after, harness("true", true));
    second.reconcile().await.expect("the boot read");
    let turned = second.turn().await.expect("a turn");
    second.woken(&turned).await;

    assert!(!second.repairing().holds(&job));
    assert!(host_after.carried().is_empty(), "nobody is woken for a repair that finished");
}

/// **The span a Job claimed before its preparation failed is still its own when
/// it begins again.** The index allows one per Job, so a second claim would have
/// escalated the repaired Job as `not_configurable` on its way to its first step.
#[tokio::test]
async fn a_repaired_job_keeps_the_ports_it_claimed_before_its_preparation_failed() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = a_fleet_requiring_ready_and(
        &home,
        &host,
        harness("touch ready", true),
        "ports:\n  storybook: {}\n",
    );
    let (job, _) = unprepared(&fleet, &home).await;
    let claimed = fleet.store().lock().await.port_span_for_job(&job).unwrap();
    assert!(claimed.is_some(), "the span was claimed before preparation ran");

    send(&fleet, &job, Breakage::MissingBinary).await.expect("sent");
    until_restarted(&fleet, &job).await;
    let back = started(&fleet, &job).await.expect("begins again");

    assert_eq!(back.status(), JobStatus::Running);
    assert_eq!(
        fleet.store().lock().await.port_span_for_job(&job).unwrap(),
        claimed,
        "the same span"
    );
}
