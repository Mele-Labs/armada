//! A plan act signed as the caller who made it: a person or Helm,
//! each through one path with one set of refusals. `#2168`.
//!
//! **The caller is the transport's word**, so every act here is handed a
//! `Steerer` as the route would place it, and what is read back is what the
//! record, the event and the wire each say. `plan_signed_by_helm` is the same
//! claim through the real router.

use std::sync::Arc;
use std::time::Duration;

use api::{Commands, Next, Refusal, Steerer, Subscription};
use core_model::{
    Actor, Approach, NewTask, PlanAuthor, PlanChange, StepId, TaskId, TaskUpdate, Timestamp,
    WhenRefused,
};
use store::{Moved, PlanHand};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::{a_fleet, a_proposal};
use crate::tests::groups::five_judged;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// What each caller is recorded as, in the three places it is read.
struct Caller {
    by: Steerer,
    actor: Actor,
    author: PlanAuthor,
    task: ipc::TaskBy,
}

fn the_callers() -> [Caller; 2] {
    [
        Caller {
            by: Steerer::Person,
            actor: Actor::Human,
            author: PlanAuthor::Person,
            task: ipc::TaskBy::Person,
        },
        Caller {
            by: Steerer::Helm,
            actor: Actor::Helm,
            author: PlanAuthor::Helm,
            task: ipc::TaskBy::Helm,
        },
    ]
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

fn implement() -> StepId {
    StepId::new("implement")
}

fn at(second: u32) -> Timestamp {
    Timestamp::from_rfc3339(&format!("2026-10-12T10:00:{second:02}.000Z"))
}

/// A Job at the gate with four open tasks in one group, recorded by a step.
async fn a_job_with_a_plan(home: &TempDir) -> (Arc<Fixture>, ipc::JobId) {
    let fleet = Arc::new(a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"])));
    let job = fleet
        .propose(a_proposal("cover the writer's bound"))
        .await
        .expect("a Job at the gate");
    let mut store = fleet.store().lock().await;
    store
        .change_plan(
            job.id(),
            &PlanChange::Recorded {
                approach: Approach::new("Bound the reader").expect("an approach"),
                tasks: ["Stop at the end", "Cover it", "Check the writer", "Note it"]
                    .iter()
                    .map(|title| NewTask::new(title, "", &[], "").expect("a title"))
                    .collect(),
            },
            PlanHand::Step(&implement()),
            &at(0),
        )
        .expect("recorded");
    drop(store);
    let id = ipc::JobId::from(job.id());
    (fleet, id)
}

async fn told(watching: &mut Subscription) -> Vec<ipc::JobPlanChanged> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(30), watching.next()).await
    {
        if let ipc::Event::JobPlanChanged(changed) = delivered.event {
            seen.push(changed);
        }
    }
    seen
}

async fn last_author(fleet: &Fixture, job: &ipc::JobId) -> PlanAuthor {
    fleet
        .store()
        .lock()
        .await
        .plan_history(&job.to_domain())
        .expect("the history")
        .last()
        .expect("a change")
        .by
        .clone()
}

fn the_add() -> ipc::AddTask {
    ipc::AddTask {
        title: "Cover the empty file".to_string(),
        note: String::new(),
        scope: Vec::new(),
        expects: String::new(),
        after: String::new(),
    }
}

fn the_wire_mark<'a>(plan: &'a ipc::WorkPlan, task: &str) -> Option<&'a ipc::TaskBy> {
    plan.tasks
        .iter()
        .find(|t| t.id == task)
        .unwrap_or_else(|| panic!("the plan holds {task}"))
        .by
        .as_ref()
}

#[tokio::test]
async fn each_caller_adds_edits_drops_and_moves_a_task_as_its_own() {
    for caller in the_callers() {
        let home = TempDir::new();
        let (fleet, job) = a_job_with_a_plan(&home).await;
        let mut watching: Subscription = fleet.events().subscribe();
        let who = format!("{:?}", caller.by);

        let added = Commands::add_task(Arc::clone(&fleet), job.clone(), the_add(), caller.by)
            .await
            .expect("an add");
        assert_eq!(added.tasks.len(), 5, "{who}: the fifth task");
        assert_eq!(last_author(&fleet, &job).await, caller.author, "{who}: add");

        let edited = Commands::edit_task(
            Arc::clone(&fleet),
            job.clone(),
            "T1".to_string(),
            ipc::EditTask {
                title: Some("Stop one row early".to_string()),
                ..ipc::EditTask::default()
            },
            caller.by,
        )
        .await
        .expect("an edit");
        assert_eq!(last_author(&fleet, &job).await, caller.author, "{who}: edit");

        let dropped = Commands::drop_task(
            Arc::clone(&fleet),
            job.clone(),
            ipc::DropTask {
                task: "T2".to_string(),
                reason: "covered by T1".to_string(),
            },
            caller.by,
        )
        .await
        .expect("a drop");
        assert_eq!(last_author(&fleet, &job).await, caller.author, "{who}: drop");

        let moved = Commands::move_plan(
            Arc::clone(&fleet),
            job.clone(),
            ipc::MovePlan {
                group: "G1".to_string(),
                task: Some("T3".to_string()),
                after: Some("T4".to_string()),
            },
            caller.by,
        )
        .await
        .expect("a move");
        assert_eq!(last_author(&fleet, &job).await, caller.author, "{who}: move");

        // The event names the caller, once per act.
        let published = told(&mut watching).await;
        let actors: Vec<Actor> = published.iter().map(|c| c.actor.domain()).collect();
        assert_eq!(actors, vec![caller.actor; 4], "{who}: four acts, four events");

        // The wire: each task an act touched wears the caller's hand, and the
        // one nothing touched wears none.
        assert_eq!(the_wire_mark(&added, "T5"), Some(&caller.task), "{who}");
        assert_eq!(the_wire_mark(&edited, "T1"), Some(&caller.task), "{who}");
        assert_eq!(the_wire_mark(&dropped, "T2"), Some(&caller.task), "{who}");
        assert_eq!(the_wire_mark(&moved, "T3"), Some(&caller.task), "{who}");
        assert_eq!(the_wire_mark(&moved, "T4"), None, "{who}: T4 was never touched");
        assert!(
            matches!(moved.recorded_by, ipc::ChangedBy::Step { .. }),
            "{who}: the recording is still the step's"
        );
    }
}

/// A task Helm changed after a person did wears the later
/// hand, and a step marking it afterwards does not take it off.
#[tokio::test]
async fn a_task_wears_whoever_last_changed_it_and_a_step_does_not_wipe_it() {
    let home = TempDir::new();
    let (fleet, job) = a_job_with_a_plan(&home).await;
    let edit = |title: &str| ipc::EditTask {
        title: Some(title.to_string()),
        ..ipc::EditTask::default()
    };
    for (by, expected) in [
        (Steerer::Person, ipc::TaskBy::Person),
        (Steerer::Helm, ipc::TaskBy::Helm),
    ] {
        let plan = Commands::edit_task(
            Arc::clone(&fleet),
            job.clone(),
            "T1".to_string(),
            edit("again"),
            by,
        )
        .await
        .expect("an edit");
        assert_eq!(the_wire_mark(&plan, "T1"), Some(&expected));
    }

    fleet
        .store()
        .lock()
        .await
        .change_plan(
            &job.to_domain(),
            &PlanChange::Updated {
                task: TaskId::read("T1").expect("an id"),
                to: TaskUpdate::Working,
                shown: None,
            },
            PlanHand::Step(&implement()),
            &at(5),
        )
        .expect("a step marks it");
    let plan = ipc::WorkPlan::from(
        &fleet
            .plan_of(&job.to_domain())
            .await
            .expect("reads")
            .expect("a plan"),
    );
    assert_eq!(
        the_wire_mark(&plan, "T1"),
        Some(&ipc::TaskBy::Helm),
        "a step marking it working leaves Helm's hand on it"
    );
}

/// **One path, one set of refusals.** What a person is told no to, Helm is
/// told no to in the same words, and nothing is kept.
#[tokio::test]
async fn every_caller_meets_the_same_refusals_and_none_leaves_a_change() {
    for caller in the_callers() {
        let home = TempDir::new();
        let (fleet, job) = a_job_with_a_plan(&home).await;
        let who = format!("{:?}", caller.by);
        {
            let mut store = fleet.store().lock().await;
            let domain = job.to_domain();
            store
                .change_plan(
                    &domain,
                    &PlanChange::Updated {
                        task: TaskId::read("T1").expect("an id"),
                        to: TaskUpdate::Done,
                        shown: None,
                    },
                    PlanHand::Step(&implement()),
                    &at(1),
                )
                .expect("T1 done");
            store
                .change_plan(
                    &domain,
                    &PlanChange::Updated {
                        task: TaskId::read("T2").expect("an id"),
                        to: TaskUpdate::Working,
                        shown: None,
                    },
                    PlanHand::Step(&implement()),
                    &at(2),
                )
                .expect("T2 working");
        }
        let kept = || async {
            fleet
                .store()
                .lock()
                .await
                .plan_history(&job.to_domain())
                .expect("the history")
                .len()
        };
        let before = kept().await;

        let drop_done = Commands::drop_task(
            Arc::clone(&fleet),
            job.clone(),
            ipc::DropTask {
                task: "T1".to_string(),
                reason: "not needed".to_string(),
            },
            caller.by,
        )
        .await
        .expect_err("T1 is done");
        assert_eq!(code(&drop_done), "fleet.task_already_settled", "{who}");

        let move_working = Commands::move_plan(
            Arc::clone(&fleet),
            job.clone(),
            ipc::MovePlan {
                group: "G1".to_string(),
                task: Some("T2".to_string()),
                after: Some("T4".to_string()),
            },
            caller.by,
        )
        .await
        .expect_err("T2 is working");
        assert_eq!(code(&move_working), "fleet.task_in_flight", "{who}");

        let edit_working = Commands::edit_task(
            Arc::clone(&fleet),
            job.clone(),
            "T2".to_string(),
            ipc::EditTask {
                title: Some("changed".to_string()),
                ..ipc::EditTask::default()
            },
            caller.by,
        )
        .await
        .expect_err("T2 is working");
        assert_eq!(code(&edit_working), "fleet.task_in_flight", "{who}");

        let blank_edit = Commands::edit_task(
            Arc::clone(&fleet),
            job.clone(),
            "T3".to_string(),
            ipc::EditTask {
                title: Some("  ".to_string()),
                ..ipc::EditTask::default()
            },
            caller.by,
        )
        .await
        .expect_err("a blank title");
        assert!(matches!(blank_edit, Refusal::Unacceptable(_)), "{who}");

        let not_failed = Commands::restart_task(
            Arc::clone(&fleet),
            job.clone(),
            "T3".to_string(),
            ipc::RestartTask::default(),
            caller.by,
        )
        .await
        .expect_err("T3 has not failed");
        assert_eq!(code(&not_failed), "fleet.task_not_failed", "{who}");

        assert_eq!(kept().await, before, "{who}: a refused act keeps nothing");
    }
}

/// A restart reopens the task under the caller's hand and moves the Job under
/// the caller's name, so the log and the plan say the same one asked.
#[tokio::test]
async fn each_caller_restarts_a_task_as_its_own() {
    for caller in the_callers() {
        let home = TempDir::new();
        let (fleet, job, _) = five_judged(&home, WhenRefused::PerCriterion).await;
        let who = format!("{:?}", caller.by);

        Commands::restart_task(
            Arc::clone(&fleet),
            ipc::JobId::from(&job),
            "T5".to_string(),
            ipc::RestartTask::default(),
            caller.by,
        )
        .await
        .expect("a task in a refused group restarts");

        let history = fleet
            .store()
            .lock()
            .await
            .plan_history(&job)
            .expect("the history");
        assert_eq!(history.last().expect("the reopening").by, caller.author, "{who}");
        let mover = fleet
            .store()
            .lock()
            .await
            .events_for(&job)
            .expect("the Job's log")
            .iter()
            .rev()
            .find_map(|event| match event.moved() {
                Moved::Job { .. } => Some(event.actor()),
                Moved::Step { .. } | Moved::Drone { .. } => None,
            })
            .expect("the Job moved");
        assert_eq!(mover, caller.actor, "{who}: the restart's move");
    }
}
