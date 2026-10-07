//! Steps added to one Job: placed at approval or added underway, fired through
//! the Trigger path, refused where they would land behind the Job, and kept for
//! every Job. Commands are real programs run in the Job's worktree; the Drone
//! and the pull request are fakes.

use std::sync::Arc;

use api::Authoring;
use core_model::JobId;
use ipc::{AddStep, AddedRuns, TriggerFiringState, TriggerMoment};
use testkit::Delivering;

use crate::tests::admitted::started;
use crate::tests::daemon::{a_proposal, diff_evidence, note_evidence, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::tests::triggering::{a_fleet, log, manifest, Files};

fn script(command: &str, when: TriggerMoment, step: &str) -> AddStep {
    AddStep {
        runs: AddedRuns::Script {
            command: command.to_string(),
        },
        when,
        step: ipc::StepId::carried(step),
        block: false,
        repair: true,
    }
}

fn approval(additions: Vec<AddStep>) -> ipc::ApproveDispatch {
    ipc::ApproveDispatch {
        additions: Some(additions),
        ..ipc::ApproveDispatch::default()
    }
}

fn code(refusal: &api::Refusal) -> &str {
    &refusal.error().code
}

async fn states(
    fleet: &crate::tests::repositories::Fixture,
    job: &JobId,
) -> Vec<(String, TriggerFiringState)> {
    let detail = fleet.job_detail(ipc::JobId::from(job)).await.unwrap();
    detail
        .additions
        .iter()
        .map(|one| (one.id.clone(), one.state))
        .collect()
}

#[tokio::test]
async fn a_script_placed_at_approval_runs_once_its_step_passes_and_the_workflow_is_untouched() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet(
        &home,
        &files,
        manifest(Some("echo formatted-by-the-addition")),
        Delivering::default(),
    );
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    let before = fleet.load(job.id()).await.unwrap();
    fleet
        .approve_as_left(
            job.id(),
            &approval(vec![script(
                "deploy_qa",
                TriggerMoment::StepPasses,
                "implement",
            )]),
        )
        .await
        .unwrap();
    started(&fleet, job.id()).await.unwrap();

    assert_eq!(
        states(&fleet, job.id()).await,
        [("a1".to_string(), TriggerFiringState::Pending)]
    );
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let detail = fleet.job_detail(ipc::JobId::from(job.id())).await.unwrap();
    let [one] = detail.additions.as_slice() else {
        panic!("one addition: {:?}", detail.additions);
    };
    assert_eq!(one.state, TriggerFiringState::Passed);
    assert_eq!(one.exit_code, Some(0));
    assert_eq!(one.placed, ipc::AddedPlaced::Approval);
    assert!(one.repair && !one.block);
    assert!(log(&fleet, &home, &job).contains("formatted-by-the-addition"));

    let after = fleet.load(job.id()).await.unwrap();
    let ids = |job: &core_model::Job| {
        job.workflow()
            .steps()
            .iter()
            .map(|step| step.id().as_str().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(&before),
        ids(&after),
        "the frozen workflow is as it was"
    );
    assert_eq!(
        before.steps().len(),
        after.steps().len(),
        "no step row was written"
    );
}

#[tokio::test]
async fn a_gap_behind_the_current_step_is_refused_and_one_after_it_is_not() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet(&home, &files, manifest(Some("true")), Delivering::default());
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    fleet.approve(job.id()).await.unwrap();
    started(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    let wire = ipc::JobId::from(job.id());

    for (when, step, reason) in [
        (TriggerMoment::StepPasses, "implement", "passed"),
        (TriggerMoment::StepStarts, "implement", "started"),
        (TriggerMoment::StepStarts, "summarise", "started"),
        (TriggerMoment::PrOpened, "summarise", "started"),
    ] {
        let refused = fleet
            .add_job_step(wire.clone(), script("fmt", when, step))
            .await
            .expect_err("behind the Job");
        assert_eq!(refused.status(), 409);
        assert_eq!(code(&refused), "fleet.added_step_behind", "{when:?} {step}");
        assert!(
            matches!(refused.error().fields.get("reason"), Some(ipc::WireValue::Str(r)) if r == reason),
            "{when:?} {step}: {:?}",
            refused.error().fields
        );
    }
    assert!(
        states(&fleet, job.id()).await.is_empty(),
        "nothing was written"
    );

    let added = fleet
        .add_job_step(
            wire.clone(),
            script("fmt", TriggerMoment::StepPasses, "summarise"),
        )
        .await
        .expect("the current step's own pass is ahead");
    assert_eq!(added.state, TriggerFiringState::Pending);
    assert_eq!(added.placed, ipc::AddedPlaced::Running);
}

#[tokio::test]
async fn a_step_added_after_the_current_one_fires_when_the_job_gets_there() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet(
        &home,
        &files,
        manifest(Some("echo after-the-pull-request")),
        Delivering::default(),
    );
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    fleet.approve(job.id()).await.unwrap();
    started(&fleet, job.id()).await.unwrap();
    let wire = ipc::JobId::from(job.id());
    let mut heard = fleet.events().subscribe();

    fleet
        .add_job_step(
            wire.clone(),
            script("deploy_qa", TriggerMoment::PrOpened, "summarise"),
        )
        .await
        .expect("the pull request has not opened");
    fleet
        .add_job_step(
            wire.clone(),
            script("fmt", TriggerMoment::StepStarts, "summarise"),
        )
        .await
        .expect("the delivering step has not started");
    assert_eq!(
        states(&fleet, job.id()).await,
        [
            ("a1".to_string(), TriggerFiringState::Pending),
            ("a2".to_string(), TriggerFiringState::Pending)
        ]
    );

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    assert_eq!(
        states(&fleet, job.id()).await,
        [
            ("a1".to_string(), TriggerFiringState::Passed),
            ("a2".to_string(), TriggerFiringState::Passed)
        ]
    );
    let delivery = fleet.store().lock().await.delivery_for(job.id()).unwrap();
    assert!(delivery.pull_request.is_some());
    let job = fleet.load(job.id()).await.unwrap();
    assert!(log(&fleet, &home, &job).contains("after-the-pull-request"));

    let seen = crate::tests::under_review::published(&mut heard).await;
    let moves: Vec<_> = seen
        .iter()
        .filter_map(|event| match event {
            ipc::Event::JobAdditionChanged(changed) if changed.addition.id == "a1" => {
                Some((changed.addition.state, changed.removed))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        moves,
        [
            (TriggerFiringState::Pending, false),
            (TriggerFiringState::Running, false),
            (TriggerFiringState::Passed, false)
        ],
        "added, opened and ended: {seen:?}"
    );
}

#[tokio::test]
async fn a_skill_and_a_drone_step_are_recorded_skipped_and_say_so() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet(&home, &files, manifest(None), Delivering::default());
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    let runs = |runs| AddStep {
        runs,
        when: TriggerMoment::StepPasses,
        step: ipc::StepId::carried("implement"),
        block: false,
        repair: false,
    };
    fleet
        .approve_as_left(
            job.id(),
            &approval(vec![
                runs(AddedRuns::Skill {
                    skill: "tidy-up".into(),
                }),
                runs(AddedRuns::Drone {
                    brief: "read it twice".into(),
                }),
                script("not_declared", TriggerMoment::StepPasses, "implement"),
            ]),
        )
        .await
        .unwrap();
    started(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let detail = fleet.job_detail(ipc::JobId::from(job.id())).await.unwrap();
    let why: Vec<_> = detail
        .additions
        .iter()
        .map(|one| (one.state, one.skipped.as_ref().map(|skip| skip.reason)))
        .collect();
    use ipc::AddedSkipReason::{DroneStepNotRun, NotInThisRepo, SkillNotRun};
    assert_eq!(
        why,
        [
            (TriggerFiringState::Skipped, Some(SkillNotRun)),
            (TriggerFiringState::Skipped, Some(DroneStepNotRun)),
            (TriggerFiringState::Skipped, Some(NotInThisRepo)),
        ]
    );
}

#[tokio::test]
async fn an_addition_is_removed_before_it_fires_and_refused_after() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet(&home, &files, manifest(Some("true")), Delivering::default());
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    fleet.approve(job.id()).await.unwrap();
    started(&fleet, job.id()).await.unwrap();
    let wire = ipc::JobId::from(job.id());
    for step in ["implement", "summarise"] {
        fleet
            .add_job_step(wire.clone(), script("fmt", TriggerMoment::StepPasses, step))
            .await
            .unwrap();
    }

    fleet
        .remove_job_step(wire.clone(), ipc::RemoveAddedStep { id: "a2".into() })
        .await
        .expect("it has not fired");
    let gone = fleet
        .remove_job_step(wire.clone(), ipc::RemoveAddedStep { id: "a2".into() })
        .await
        .expect_err("already removed");
    assert_eq!(code(&gone), "fleet.no_such_addition");

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    let fired = fleet
        .remove_job_step(wire.clone(), ipc::RemoveAddedStep { id: "a1".into() })
        .await
        .expect_err("it has fired");
    assert_eq!(
        (fired.status(), code(&fired)),
        (409, "fleet.added_step_fired")
    );
    assert_eq!(
        states(&fleet, job.id()).await,
        [("a1".to_string(), TriggerFiringState::Passed)]
    );
}

#[tokio::test]
async fn an_approval_refuses_a_place_the_workflow_lacks_and_nothing_is_kept() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet(&home, &files, manifest(Some("true")), Delivering::default());
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();

    for added in [
        script("fmt", TriggerMoment::StepPasses, "nowhere"),
        script("fmt", TriggerMoment::PrOpened, "implement"),
        script("  ", TriggerMoment::StepPasses, "implement"),
    ] {
        let refused = fleet
            .approve_as_left(job.id(), &approval(vec![added]))
            .await
            .expect_err("an unplaceable step");
        assert!(refused.to_string().contains("added step"), "{refused}");
    }
    let held = fleet.load(job.id()).await.unwrap();
    assert_eq!(held.status(), core_model::JobStatus::AwaitingApproval);
    assert!(states(&fleet, job.id()).await.is_empty());

    let early = fleet
        .add_job_step(
            ipc::JobId::from(job.id()),
            script("fmt", TriggerMoment::StepPasses, "implement"),
        )
        .await
        .expect_err("not approved yet");
    assert_eq!(code(&early), "fleet.added_step_before_approval");
}

#[tokio::test]
async fn keeping_a_script_saves_a_trigger_that_applies_to_the_next_job() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet(
        &home,
        &files,
        manifest(Some("echo kept-and-fired")),
        Delivering::default(),
    );
    let first = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &first);
    fleet.approve(first.id()).await.unwrap();
    started(&fleet, first.id()).await.unwrap();
    let wire = ipc::JobId::from(first.id());
    fleet
        .add_job_step(
            wire.clone(),
            script("deploy_qa", TriggerMoment::StepPasses, "implement"),
        )
        .await
        .unwrap();

    let keep = |addition: &str, scope| ipc::SaveTrigger {
        scope,
        definition: "name: deploy\nwhen: step_passes\nstep: implement\ncommand: deploy_qa\n".into(),
        overwrite: false,
        kept_from: Some(ipc::KeptFrom {
            job_id: wire.clone(),
            addition_id: addition.into(),
        }),
    };
    let missing = fleet
        .save_trigger(keep("a9", ipc::TriggerScope::Machine), None)
        .await
        .expect_err("no such addition");
    assert_eq!(code(&missing), "fleet.no_such_addition");
    assert!(
        files.0.lock().unwrap().is_empty(),
        "refused before anything was written"
    );

    let saved = fleet
        .save_trigger(keep("a1", ipc::TriggerScope::Machine), None)
        .await
        .expect("kept");
    assert_eq!(saved.scope, ipc::TriggerScope::Machine);
    let detail = fleet.job_detail(wire.clone()).await.unwrap();
    assert_eq!(detail.additions[0].kept, Some(ipc::TriggerScope::Machine));

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let next = fleet.propose(a_proposal("fix the writer")).await.unwrap();
    worktree_directory(&home, &next);
    fleet.approve(next.id()).await.unwrap();
    started(&fleet, next.id()).await.unwrap();
    let frozen = fleet
        .store()
        .lock()
        .await
        .frozen_triggers(next.id())
        .unwrap();
    assert_eq!(
        frozen
            .iter()
            .map(|one| one.name.as_str())
            .collect::<Vec<_>>(),
        ["deploy"],
        "the kept Trigger is frozen onto the next Job at its approval"
    );
    assert!(
        fleet
            .job_detail(ipc::JobId::from(next.id()))
            .await
            .unwrap()
            .additions
            .is_empty(),
        "the first Job's addition is not the next one's"
    );
}

#[tokio::test]
async fn a_drone_step_cannot_be_kept_for_every_job() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet(&home, &files, manifest(None), Delivering::default());
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    fleet.approve(job.id()).await.unwrap();
    started(&fleet, job.id()).await.unwrap();
    let wire = ipc::JobId::from(job.id());
    fleet
        .add_job_step(
            wire.clone(),
            AddStep {
                runs: AddedRuns::Drone {
                    brief: "read it twice".into(),
                },
                when: TriggerMoment::StepPasses,
                step: ipc::StepId::carried("implement"),
                block: false,
                repair: false,
            },
        )
        .await
        .unwrap();
    let refused = fleet
        .save_trigger(
            ipc::SaveTrigger {
                scope: ipc::TriggerScope::Machine,
                definition: "name: read\nwhen: step_passes\ncommand: fmt\n".into(),
                overwrite: false,
                kept_from: Some(ipc::KeptFrom {
                    job_id: wire,
                    addition_id: "a1".into(),
                }),
            },
            None,
        )
        .await
        .expect_err("a Drone step is not a Trigger");
    assert_eq!(code(&refused), "fleet.unacceptable_addition");
    assert!(files.0.lock().unwrap().is_empty());
}
