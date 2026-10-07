//! A Trigger or an added step that blocks and fails holds its Job, at the place
//! its moment stands in front of, until the owner reruns it or skips it or a
//! repair fixes it. Commands are real programs; the Drone and the pull request
//! are fakes.

use std::sync::Arc;

use core_model::{Actor, EscalationTrigger, JobId, JobStatus, TransitionReason, TriggerState};
use ipc::{HoldAct, JobAlertKind, TriggerFiringState as Wire};
use testkit::{Delivering, FakeHarness, FakeVcs, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::tests::admitted::started;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fitted_over, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::tests::triggering::{a_fleet, machine, manifest, to_the_delivering_step, Files};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn blocking(name: &str, moment: &str, repair: bool) -> config::TriggerWritten {
    machine(
        &format!("{name}.yml"),
        &format!(
            "name: {name}\nwhen: {moment}\ncommand: deploy_qa\non_failure:\n  block: true\n  repair: {repair}\n"
        ),
    )
}

/// `deploy_qa` passes once `flag` exists.
fn a_fleet_with(home: &TempDir, files: &Arc<Files>, flag: &str) -> Arc<Fixture> {
    Arc::new(a_fleet(
        home,
        files,
        manifest(Some(&format!("test -f {flag}"))),
        Delivering::default(),
    ))
}

fn flag_in(home: &TempDir) -> String {
    format!("{}/flag", home.path().display())
}

fn act(trigger: &str) -> HoldAct {
    HoldAct {
        trigger: Some(trigger.to_string()),
        addition: None,
    }
}

async fn the_firings(fleet: &Fixture, job: &JobId) -> Vec<core_model::TriggerFiring> {
    fleet.store().lock().await.trigger_firings(job).unwrap()
}

async fn stopped_on_the_hold(fleet: &Fixture, job: &JobId) -> bool {
    let held = fleet.load(job).await.unwrap();
    held.status() == JobStatus::Escalated
        && matches!(
            fleet.last_reason(job).await,
            Ok(Some(TransitionReason::Escalation(
                EscalationTrigger::TriggerHeld
            )))
        )
}

async fn proposed_and_approved(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(home, &job);
    fleet.approve(job.id()).await.unwrap();
    job.id().clone()
}

#[tokio::test]
async fn a_pr_opened_hold_stops_the_approval_and_the_merge_until_a_rerun_passes() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    let files = Arc::new(Files::default());
    files.say(vec![blocking("deploy", "pr_opened", false)]);
    let fleet = a_fleet_with(&home, &files, &flag);
    let id = to_the_delivering_step(&fleet, &home).await;

    let [only] = the_firings(&fleet, &id)
        .await
        .try_into()
        .expect("one firing");
    assert_eq!((only.state, only.exit_code), (TriggerState::Held, Some(1)));
    // The pull request is out and nothing escalates: the Drone works on to the gate.
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Running);
    let row = fleet
        .published(&fleet.load(&id).await.unwrap())
        .await
        .unwrap();
    let alert = row.alert.expect("the row carries the bell");
    assert_eq!(
        (alert.kind, alert.trigger.as_str()),
        (JobAlertKind::Held, "deploy")
    );

    let wire = ipc::JobId::from(&id);
    let working = Arc::clone(&fleet)
        .hold_rerun(wire.clone(), act("deploy"))
        .await;
    assert_eq!(
        working
            .expect_err("a Drone is in the tree")
            .error()
            .code
            .as_str(),
        "fleet.hold_job_working"
    );

    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    assert_eq!(
        fleet.load(&id).await.unwrap().status(),
        JobStatus::AwaitingReview,
        "a held gate is a person's, whatever the policy says"
    );
    let approving = fleet.approved(&id, Actor::Human).await.expect_err("held");
    assert!(
        matches!(approving, Adrift::TriggerHolds { .. }),
        "{approving:?}"
    );
    assert_eq!(
        fleet.refusal(approving).error().code.as_str(),
        "fleet.trigger_holds"
    );
    let merging = fleet.merge_pull_request(&id).await.expect_err("held");
    assert!(
        matches!(merging, Adrift::TriggerHolds { .. }),
        "{merging:?}"
    );
    assert_eq!(
        fleet.load(&id).await.unwrap().status(),
        JobStatus::AwaitingReview,
        "nothing moved"
    );

    let again = Arc::clone(&fleet)
        .hold_rerun(wire.clone(), act("deploy"))
        .await
        .expect("ran again");
    assert_eq!((again.state, again.released), (Wire::Held, false));

    std::fs::write(&flag, "").unwrap();
    let passed = Arc::clone(&fleet)
        .hold_rerun(wire, act("deploy"))
        .await
        .expect("ran again");
    assert_eq!((passed.state, passed.released), (Wire::Passed, true));
    assert_eq!(
        the_firings(&fleet, &id).await[0].state,
        TriggerState::Passed
    );
    assert!(fleet
        .published(&fleet.load(&id).await.unwrap())
        .await
        .unwrap()
        .alert
        .is_none());
    fleet
        .approved(&id, Actor::Human)
        .await
        .expect("the gate is open");
    assert_eq!(
        fleet.load(&id).await.unwrap().status(),
        JobStatus::CompletedSuccess
    );
}

#[tokio::test]
async fn skipping_a_pr_opened_hold_opens_the_gate_and_records_who_skipped_it() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![blocking("deploy", "pr_opened", false)]);
    let fleet = a_fleet_with(&home, &files, &flag_in(&home));
    let id = to_the_delivering_step(&fleet, &home).await;
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let skipped = fleet
        .hold_skip(ipc::JobId::from(&id), act("deploy"))
        .await
        .expect("skipped");
    assert_eq!((skipped.state, skipped.released), (Wire::Skipped, true));

    let [only] = the_firings(&fleet, &id)
        .await
        .try_into()
        .expect("one firing");
    assert_eq!(only.state, TriggerState::Skipped);
    assert_eq!(only.skipped, Some(core_model::TriggerSkipped::ByOwner));
    fleet
        .approved(&id, Actor::Human)
        .await
        .expect("the gate is open");
}

#[tokio::test]
async fn a_step_starts_hold_stops_the_job_before_its_drone_and_a_rerun_lets_it_start() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    let files = Arc::new(Files::default());
    files.say(vec![machine(
        "begin.yml",
        "name: begin\nwhen: step_starts\nstep: implement\ncommand: deploy_qa\non_failure:\n  block: true\n",
    )]);
    let fleet = a_fleet_with(&home, &files, &flag);
    let id = proposed_and_approved(&fleet, &home).await;

    let held = started(&fleet, &id)
        .await
        .expect_err("held before its Drone");
    assert!(matches!(held, Adrift::TriggerHolds { .. }), "{held:?}");
    assert!(stopped_on_the_hold(&fleet, &id).await);
    assert!(fleet.load(&id).await.unwrap().assigned_drone().is_none());
    assert!(
        fleet.harness().configured().is_empty(),
        "no Drone was configured"
    );
    let blocked = fleet.alerts(None).await.unwrap().blocked;
    assert!(
        blocked
            .iter()
            .any(|one| one.why.as_deref().is_some_and(|why| why.contains("begin"))),
        "{blocked:?}"
    );

    let wire = ipc::JobId::from(&id);
    let again = Arc::clone(&fleet)
        .hold_rerun(wire.clone(), act("begin"))
        .await
        .unwrap();
    assert_eq!((again.state, again.released), (Wire::Held, false));
    assert!(stopped_on_the_hold(&fleet, &id).await, "still held");

    std::fs::write(&flag, "").unwrap();
    let passed = Arc::clone(&fleet)
        .hold_rerun(wire, act("begin"))
        .await
        .unwrap();
    assert_eq!((passed.state, passed.released), (Wire::Passed, true));
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Queued);

    let job = started(&fleet, &id).await.unwrap();
    assert_eq!(job.status(), JobStatus::Running);
    assert_eq!(
        the_firings(&fleet, &id).await.len(),
        1,
        "its Trigger is not fired a second time on the way in"
    );
}

#[tokio::test]
async fn a_step_passes_hold_stops_the_next_step_and_a_skip_lets_it_start_without_firing_again() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![machine(
        "tidy.yml",
        "name: tidy\nwhen: step_passes\nstep: implement\ncommand: deploy_qa\non_failure:\n  block: true\n",
    )]);
    let fleet = a_fleet_with(&home, &files, &flag_in(&home));
    let id = proposed_and_approved(&fleet, &home).await;
    started(&fleet, &id).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let _ = fleet.turn().await;

    assert!(stopped_on_the_hold(&fleet, &id).await);
    let job = fleet.load(&id).await.unwrap();
    assert_eq!(
        job.step(&core_model::StepId::new("implement"))
            .unwrap()
            .state(),
        core_model::StepState::Advanced,
        "the step passed and the next did not start a Drone"
    );
    let summarise = core_model::StepId::new("summarise");
    assert_eq!(
        fleet.harness().configured().len(),
        1,
        "only the first step's Drone"
    );
    assert_eq!(job.current_step_id(), Some(&summarise));

    let skipped = fleet
        .hold_skip(ipc::JobId::from(&id), act("tidy"))
        .await
        .unwrap();
    assert!(skipped.released);
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Queued);
    started(&fleet, &id).await.unwrap();
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Running);
    assert_eq!(fleet.harness().configured().len(), 2);
    let [only] = the_firings(&fleet, &id)
        .await
        .try_into()
        .expect("one firing");
    assert_eq!(only.skipped, Some(core_model::TriggerSkipped::ByOwner));
}

#[tokio::test]
async fn a_failure_that_does_not_block_holds_nothing_and_neither_does_the_last_steps_pass() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![
        machine(
            "quiet.yml",
            "name: quiet\nwhen: step_starts\nstep: implement\ncommand: deploy_qa\n",
        ),
        // Nothing comes after the last step, so there is nothing to stand in front of.
        machine(
            "last.yml",
            "name: last\nwhen: step_passes\nstep: summarise\ncommand: deploy_qa\non_failure:\n  block: true\n",
        ),
    ]);
    let fleet = a_fleet_with(&home, &files, &flag_in(&home));
    let id = proposed_and_approved(&fleet, &home).await;
    started(&fleet, &id).await.expect("no hold");
    assert_eq!(
        the_firings(&fleet, &id).await[0].state,
        TriggerState::Failed
    );
    assert!(fleet.holds_on(&id).await.unwrap().is_empty());
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let last = the_firings(&fleet, &id)
        .await
        .into_iter()
        .find(|one| one.name == "last")
        .expect("it fired");
    assert_eq!(
        last.state,
        TriggerState::Failed,
        "recorded as it failed, and holding nothing"
    );
    assert!(!last.on_failure.block);
    assert!(fleet.holds_on(&id).await.unwrap().is_empty());
}

fn harness_that_repairs_by(drone: &str) -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            &format!(
                "IFS= read -r line; case \"$line\" in *\"REPAIR THE TRIGGER\"*) {drone};; *) sleep 30;; esac"
            ),
        ],
    )
}

fn a_repairing_fleet(home: &TempDir, drone: &str, flag: &str) -> Arc<Fixture> {
    let files = Arc::new(Files::default());
    files.say(vec![machine(
        "begin.yml",
        "name: begin\nwhen: step_starts\nstep: implement\ncommand: deploy_qa\non_failure:\n  block: true\n  repair: true\n",
    )]);
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        harness_that_repairs_by(drone),
        FakeVcs::new().delivering(Delivering::default()),
    );
    fittings.starting().manifest = manifest(Some(&format!("test -f {flag}")));
    fittings.locating = Arc::new(files);
    Arc::new(Fleet::assembled(fittings))
}

#[tokio::test]
async fn a_repair_that_ends_passed_lets_the_hold_go_by_itself() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    let fleet = a_repairing_fleet(&home, &format!("touch {flag}"), &flag);
    let id = proposed_and_approved(&fleet, &home).await;
    started(&fleet, &id).await.expect_err("held");
    assert!(stopped_on_the_hold(&fleet, &id).await);
    assert_eq!(
        the_firings(&fleet, &id).await[0].state,
        TriggerState::Repairing
    );

    let wire = ipc::JobId::from(&id);
    let refused = Arc::clone(&fleet)
        .hold_rerun(wire.clone(), act("begin"))
        .await
        .expect_err("under repair");
    assert_eq!(refused.error().code.as_str(), "fleet.hold_repairing");
    let refused = fleet
        .hold_skip(wire.clone(), act("begin"))
        .await
        .expect_err("under repair");
    assert_eq!(refused.error().code.as_str(), "fleet.hold_repairing");

    assert!(fleet.repair_next().await);
    assert_eq!(
        the_firings(&fleet, &id).await[0].state,
        TriggerState::FixReady
    );
    assert!(
        stopped_on_the_hold(&fleet, &id).await,
        "the fix is not placed yet"
    );
    let refused = Arc::clone(&fleet)
        .hold_rerun(wire, act("begin"))
        .await
        .expect_err("a fix waits");
    assert_eq!(refused.error().code.as_str(), "fleet.hold_has_a_fix");
    let row = fleet
        .published(&fleet.load(&id).await.unwrap())
        .await
        .unwrap();
    assert_eq!(
        row.alert.expect("a bell").kind,
        JobAlertKind::FixReady,
        "the fix waiting on his choice is what he can act on"
    );

    fleet
        .choose_trigger_fix(&id, "begin", core_model::FixChoice::NewPr)
        .await
        .expect("placed");
    assert_eq!(
        the_firings(&fleet, &id).await[0].state,
        TriggerState::Passed
    );
    assert_eq!(
        fleet.load(&id).await.unwrap().status(),
        JobStatus::Queued,
        "let go"
    );
    started(&fleet, &id).await.unwrap();
    assert_eq!(
        the_firings(&fleet, &id).await.len(),
        1,
        "not fired again on the way in"
    );
}

#[tokio::test]
async fn two_failed_repair_tries_leave_it_held_and_the_owner_can_still_skip() {
    let home = TempDir::new();
    let flag = format!("{}/never", home.path().display());
    let fleet = a_repairing_fleet(&home, "true", &flag);
    let id = proposed_and_approved(&fleet, &home).await;
    started(&fleet, &id).await.expect_err("held");

    assert!(fleet.repair_next().await);
    let [only] = the_firings(&fleet, &id)
        .await
        .try_into()
        .expect("one firing");
    assert_eq!(only.state, TriggerState::Held, "held and not failed");
    assert_eq!(only.repair.tries, core_model::REPAIR_TRIES);
    assert!(stopped_on_the_hold(&fleet, &id).await);

    let skipped = fleet
        .hold_skip(ipc::JobId::from(&id), act("begin"))
        .await
        .unwrap();
    assert!(skipped.released);
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Queued);
}

#[tokio::test]
async fn an_added_step_that_blocks_holds_the_job_and_its_rerun_lets_it_go() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    let files = Arc::new(Files::default());
    let fleet = a_fleet_with(&home, &files, &flag);
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    let added = ipc::AddStep {
        runs: ipc::AddedRuns::Script {
            command: "deploy_qa".into(),
        },
        when: ipc::TriggerMoment::StepPasses,
        step: ipc::StepId::carried("implement"),
        block: true,
        repair: false,
    };
    fleet
        .approve_as_left(
            job.id(),
            &ipc::ApproveDispatch {
                additions: Some(vec![added]),
                ..ipc::ApproveDispatch::default()
            },
        )
        .await
        .unwrap();
    let id = job.id().clone();
    started(&fleet, &id).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let _ = fleet.turn().await;

    assert!(stopped_on_the_hold(&fleet, &id).await);
    let detail = fleet.job_detail(ipc::JobId::from(&id)).await.unwrap();
    assert_eq!(detail.additions[0].state, Wire::Held);

    std::fs::write(&flag, "").unwrap();
    let wire = ipc::JobId::from(&id);
    let both = HoldAct {
        trigger: Some("x".into()),
        addition: Some("a1".into()),
    };
    let refused = Arc::clone(&fleet)
        .hold_rerun(wire.clone(), both)
        .await
        .expect_err("both");
    assert_eq!(refused.error().code.as_str(), "fleet.no_hold_named");
    let passed = Arc::clone(&fleet)
        .hold_rerun(
            wire.clone(),
            HoldAct {
                trigger: None,
                addition: Some("a1".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!((passed.state, passed.released), (Wire::Passed, true));
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Queued);
    let none = fleet
        .hold_skip(wire, act("nothing"))
        .await
        .expect_err("nothing holds it");
    assert_eq!(
        (none.status(), none.error().code.as_str()),
        (409, "fleet.no_hold")
    );
}

#[tokio::test]
async fn a_rerun_with_no_worktree_to_run_in_is_refused_and_leaves_the_hold() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![blocking("begin", "step_starts", false)]);
    let fleet = a_fleet_with(&home, &files, &flag_in(&home));
    let id = proposed_and_approved(&fleet, &home).await;
    started(&fleet, &id).await.expect_err("held");
    let tree = crate::tests::daemon::spec_held(&home, &fleet.load(&id).await.unwrap())
        .unwrap()
        .worktree_path();
    std::fs::remove_dir_all(tree).unwrap();

    let refused = Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act("begin"))
        .await
        .expect_err("nowhere to run");
    assert_eq!(refused.error().code.as_str(), "fleet.hold_no_worktree");
    assert!(stopped_on_the_hold(&fleet, &id).await);
}
