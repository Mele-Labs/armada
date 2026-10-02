//! A task's tier and a person's edit replay after a reopen, a Job's tier map is
//! replaced whole, and the model each Drone ran is kept. Spike 022, slice 3.

use core_model::{
    Approach, DroneId, ModelName, NewTask, PlanChange, TaskEdit, TaskId, TaskTier, TierModels, Ulid,
};

use crate::tests::attempt::{on_its_first_run, step_id};
use crate::tests::{at, job_id, open, TempDir};
use crate::PlanHand;

fn model(name: &str) -> ModelName {
    ModelName::new(name).expect("a model name")
}

fn task(id: &str) -> TaskId {
    TaskId::read(id).expect("a task id")
}

/// The planner's tier and a person's edit, model included, read back as they
/// were kept, and the recording still reads as the planner's.
#[test]
fn a_tier_and_a_persons_edit_survive_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01MODELPERTASK");
    let step = step_id();
    let job = job_id("01MODELPERTASK");
    let at = at("2026-10-02T10:05:00.000Z");
    let recorded = PlanChange::Recorded {
        approach: Approach::new("Bound the reader").expect("an approach"),
        tasks: vec![
            NewTask::new(
                "Stop at the end",
                "",
                &["crates/store/src/read.rs"],
                "a test",
            )
            .expect("a title")
            .at_tier(Some(TaskTier::Difficult)),
            NewTask::new("Tidy", "", &[], "").expect("a title"),
        ],
    };
    store
        .change_plan(&job, &recorded, PlanHand::Step(&step), &at)
        .expect("kept");
    let edit = TaskEdit::new(
        None,
        Some("the bound is read in `next`"),
        Some(&[]),
        None,
        Some(model("a-model-of-my-own")),
    )
    .expect("an edit");
    store
        .change_plan(
            &job,
            &PlanChange::Edited {
                task: task("T1"),
                edit,
            },
            PlanHand::Person,
            &at,
        )
        .expect("a person's edit is kept");
    drop(store);

    let plan = open(&dir)
        .work_plan(&job)
        .expect("the history replays")
        .expect("a plan");
    let t1 = plan.task(task("T1")).expect("T1");
    assert_eq!(t1.tier(), Some(TaskTier::Difficult));
    assert_eq!(t1.title(), "Stop at the end", "a field not edited is kept");
    assert_eq!(t1.note(), "the bound is read in `next`");
    assert!(t1.scope().is_empty(), "scope sent as [] clears it");
    assert_eq!(t1.expects(), "a test");
    assert_eq!(t1.model(), Some(&model("a-model-of-my-own")));
    assert_eq!(plan.task(task("T2")).and_then(|t| t.tier()), None);
    assert!(matches!(
        plan.recorded_by(),
        core_model::PlanAuthor::Step { .. }
    ));
}

/// A tier left out of the map is a row that is not there, and a second map
/// replaces the first rather than adding to it.
#[test]
fn a_tier_map_is_replaced_whole_and_a_tier_left_out_is_not_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01MODELPERTASK");
    let job = job_id("01MODELPERTASK");
    assert!(store.tier_models(&job).expect("reads").is_empty());
    let first = TierModels::default()
        .with(TaskTier::Difficult, model("strong"))
        .with(TaskTier::Easy, model("cheap"));
    store.set_tier_models(&job, &first).expect("kept");
    assert_eq!(store.tier_models(&job).expect("reads"), first);
    let second = TierModels::default().with(TaskTier::Medium, model("middling"));
    store.set_tier_models(&job, &second).expect("kept");
    assert_eq!(store.tier_models(&job).expect("reads"), second);
    assert!(store
        .set_tier_models(&job_id("01NOSUCHJOB"), &second)
        .is_err());
}

#[test]
fn the_model_each_drone_ran_is_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01MODELPERTASK");
    let job = job_id("01MODELPERTASK");
    let drone = DroneId::carried(Ulid::carried("01DRONEONE"));
    store
        .record_drone_model(&job, &drone, &model("strong"))
        .expect("kept");
    assert_eq!(
        store.drone_models(&job).expect("reads"),
        [(drone, "strong".to_string())]
    );
}
