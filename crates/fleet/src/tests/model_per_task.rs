//! A model per task, driven through the fake harness and the store: spike 022,
//! slice 3. The hermetic claim is `crates/acceptance/tests/drone_per_task.rs`;
//! this is the spawn recording what it resolved, and the two routes refusing a
//! model `list_models` does not offer.

use std::sync::Arc;

use api::{Commands, Queries, Refusal};
use core_model::{Approach, EvidenceType, JobId, NewTask, PlanChange, TaskTier};
use testkit::FakeWorkProduct;
use verification::{Claimed, NotClaimed, ShownBy};

use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fittings, one, worktree_directory};
use crate::tests::drone_per_task::a_drone_per_task;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// The fixture Fleet offers these two, `tests::daemon`'s.
const OFFERED: &str = "another-model";

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

/// T1 hard, T2 easy, T3 left to Armada.
fn three_tiered() -> PlanChange {
    let task = |title: &str, tier| {
        NewTask::new(title, "", &[], "")
            .expect("a title")
            .at_tier(tier)
    };
    PlanChange::Recorded {
        approach: Approach::new("Bound the reader, cover it, say so").expect("an approach"),
        tasks: vec![
            task("Stop the reader at the end", Some(TaskTier::Difficult)),
            task("Cover the last row", Some(TaskTier::Easy)),
            task("Note the bound", None),
        ],
    }
}

fn edit(json: &str) -> ipc::EditTask {
    ipc::decode("an edit", json.as_bytes()).expect("decodes")
}

/// A Job whose plan is recorded and whose plan step has not yet passed its
/// gate — before the plan's gate, where an edit is meant to work.
async fn planned(home: &TempDir) -> (Arc<Fixture>, JobId) {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = one(a_drone_per_task());
    let fleet = Arc::new(Fleet::assembled(fittings));
    let job = fleet
        .propose(a_proposal_for("bound the reader", "fixture-per-task"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(home, &job);
    dispatched(&fleet, &job_id).await.expect("it dispatches");
    fleet
        .change_plan(&job_id, &three_tiered())
        .await
        .expect("the plan step records");
    (fleet, job_id)
}

async fn hand_in(fleet: &Fixture) {
    submitted_by_the_one(
        fleet,
        Call {
            evidence_type: EvidenceType::Diff,
            claimed: Claimed("Done."),
            shown_by: ShownBy("the diff"),
            not_claimed: NotClaimed(""),
            review: None,
        },
    )
    .await
    .expect("a hand-in is recorded");
}

/// Each task's Drone runs on what its tier, or a person, picked, and its row
/// says which: the map for T1's tier, a person's pick for T2 over a tier the
/// map leaves out, and the Job's own for T3.
#[tokio::test]
async fn each_tasks_drone_runs_on_its_resolved_model_and_its_row_says_which() {
    let home = TempDir::new();
    let (fleet, job) = planned(&home).await;
    let job_model = fleet.load(&job).await.expect("reads").model().clone();
    assert_ne!(job_model.as_str(), OFFERED, "the two must differ to tell");

    let refused = Commands::set_tiers(
        Arc::clone(&fleet),
        (&job).into(),
        ipc::SetTiers {
            tiers: ipc::TierModels {
                difficult: Some(OFFERED.to_string()),
                easy: Some("a-model-nobody-offers".to_string()),
                ..ipc::TierModels::default()
            },
        },
    )
    .await
    .expect_err("one model is not offered");
    assert!(
        matches!(refused, Refusal::IllegalMove(_)),
        "a 409, as set_model refuses one: {refused:?}"
    );
    let detail = Queries::get_job(&*fleet, (&job).into())
        .await
        .expect("the detail");
    assert!(detail.tiers.is_empty(), "a refused map changes nothing");

    Commands::set_tiers(
        Arc::clone(&fleet),
        (&job).into(),
        ipc::SetTiers {
            tiers: ipc::TierModels {
                difficult: Some(OFFERED.to_string()),
                ..ipc::TierModels::default()
            },
        },
    )
    .await
    .expect("an offered model is taken");
    let detail = Queries::get_job(&*fleet, (&job).into())
        .await
        .expect("the detail");
    assert_eq!(detail.tiers.difficult.as_deref(), Some(OFFERED));
    assert_eq!(detail.tiers.easy, None, "a tier left out stays out");

    // Before the plan's gate: a person picks T2's model.
    let plan = Commands::edit_task(
        Arc::clone(&fleet),
        (&job).into(),
        "T2".to_string(),
        edit(&format!(r#"{{"model":"{OFFERED}"}}"#)),
    )
    .await
    .expect("an open task takes an edit before the gate");
    assert_eq!(plan.tasks[1].model.as_deref(), Some(OFFERED));
    assert_eq!(plan.tasks[1].tier.map(|t| t.as_wire()), Some("easy"));

    submitted_by_the_one(
        &fleet,
        Call {
            evidence_type: EvidenceType::Plan,
            claimed: Claimed("Planned as three tasks."),
            shown_by: ShownBy("the plan recorded with record_plan"),
            not_claimed: NotClaimed(""),
            review: None,
        },
    )
    .await
    .expect("the plan step asked for a plan");
    fleet.turn().await.expect("the plan step's gate runs");
    for _ in 0..2 {
        hand_in(&fleet).await;
        fleet.turn().await.expect("the next task's Drone");
    }

    let listed = fleet.job_drones((&job).into()).await.expect("the Drones");
    let ran: Vec<(Option<&str>, Option<&str>)> = listed
        .drones
        .iter()
        .filter(|drone| drone.step_id.as_str() == "implement")
        .map(|drone| (drone.task.as_deref(), drone.model.as_deref()))
        .collect();
    assert_eq!(
        ran,
        [
            (Some("T1"), Some(OFFERED)),
            (Some("T2"), Some(OFFERED)),
            (Some("T3"), Some(job_model.as_str())),
        ]
    );
    let planner = listed
        .drones
        .iter()
        .find(|drone| drone.step_id.as_str() == "plan")
        .expect("the plan step's Drone");
    assert_eq!(
        planner.model.as_deref(),
        Some(job_model.as_str()),
        "a Drone on no task says which model it ran too"
    );
}

/// `edit_task` refuses a model `list_models` does not offer, a task in its
/// Drone's hands, and a body that changes nothing; the plan is untouched.
#[tokio::test]
async fn an_edit_is_refused_on_a_model_not_offered_a_working_task_and_an_empty_body() {
    let home = TempDir::new();
    let (fleet, job) = planned(&home).await;
    let send = |task: &str, body: &str| {
        Commands::edit_task(
            Arc::clone(&fleet),
            (&job).into(),
            task.to_string(),
            edit(body),
        )
    };
    let refused = send("T1", r#"{"title":"Again","model":"a-model-nobody-offers"}"#)
        .await
        .expect_err("not offered");
    assert!(
        matches!(refused, Refusal::IllegalMove(_)),
        "a 409, as set_model refuses one: {refused:?}"
    );
    let refused = send("T1", "{}").await.expect_err("changes nothing");
    assert!(matches!(refused, Refusal::Unacceptable(_)), "{refused:?}");
    let refused = send("T9", r#"{"title":"Again"}"#).await.expect_err("no T9");
    assert_eq!(code(&refused), "fleet.no_such_task");

    submitted_by_the_one(
        &fleet,
        Call {
            evidence_type: EvidenceType::Plan,
            claimed: Claimed("Planned as three tasks."),
            shown_by: ShownBy("the plan recorded with record_plan"),
            not_claimed: NotClaimed(""),
            review: None,
        },
    )
    .await
    .expect("the plan step asked for a plan");
    fleet.turn().await.expect("the plan step's gate runs");
    let refused = send("T1", r#"{"title":"Again"}"#)
        .await
        .expect_err("T1 is working");
    assert_eq!(code(&refused), "fleet.task_in_flight");
    let plan = fleet.plan_of(&job).await.expect("reads").expect("a plan");
    assert_eq!(plan.tasks()[0].title(), "Stop the reader at the end");

    // After the gate, an open task still takes one, and the change is a
    // person's on the record after the planner's recording.
    let plan = send("T3", r#"{"note":"Say where the bound is read","scope":[]}"#)
        .await
        .expect("an open task takes an edit after the gate");
    assert_eq!(plan.tasks[2].note, "Say where the bound is read");
    assert!(matches!(plan.recorded_by, ipc::ChangedBy::Step { .. }));
    let history = fleet
        .store()
        .lock()
        .await
        .plan_history(&job)
        .expect("reads");
    assert!(matches!(
        history.last().map(|entry| (&entry.change, &entry.by)),
        Some((PlanChange::Edited { .. }, core_model::PlanAuthor::Person))
    ));
}
