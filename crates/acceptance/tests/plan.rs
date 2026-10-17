//! Plan's claim: **a running Job says how far through its plan it is.** Every
//! assertion is made against what crossed [`ipc::encode`], as `board.rs`'s are,
//! and the apparatus is [`bench::plan`]. Every step of the claim is carried:
//! #893 to #895 and #897 here, and #896 and #898 as the values their screens draw.
//! #1006 widens it: a step may keep its own product and record the plan
//! beside it, and neither a plan step nor a following one is ever told to plan
//! a task for the checks that already run on their own.
//!
//! | Not proved here | Why not, and what proves it |
//! |---|---|
//! | That job detail's Plan region draws the plan | Nothing here renders. `JobDetail.work_plan` is asserted below; `planOf` is exercised only by the `PlanPartwayDone` and `PlanWithADroppedTask` stories |
//! | That the Board's row draws "2 of 4" | The same. `JobSummary.tasks` is asserted below, the dropped task left out of the figure; `taskFigureOf` and `taskBarSegmentsOf` are tested in `board.test.ts` |
//!
//! Which step is given which tool is `fleet::work_plan`, `pub(crate)`, and is
//! asserted in `fleet`'s own tests beside the store's append-only history.

// The bench is shared with the other milestones' tests and none of them uses
// all of it.
#[allow(dead_code)]
mod bench;


use core_model::NotAnUpdate;
use ipc::mcp::{NotAnArgument, PlanArgument};
use ipc::{ChangedBy, Event, JobPlanChanged};

use bench::board::received_detail;
use bench::plan::called;
use bench::plan::received_event;
use bench::plan::received_row;
use bench::plan::refused;
use bench::plan::Planned;
use bench::plan::IMPLEMENT;
use bench::plan::PLAN;

const FOUR_TASKS: &str = r#"{"approach":"Stop the reader at the end, then cover the bound",
    "tasks":[{"title":"Stop read_to before end","note":"","scope":["crates/store/src/read.rs"],"expects":"a test for the last row"},
             {"title":"Cover the last row","note":"","scope":[],"expects":""},
             {"title":"Check the writer's bound","note":"","scope":[],"expects":""},
             {"title":"Note the bound in the module","note":"","scope":[],"expects":""}]}"#;







/// The plan step's Drone records four tasks, and the plan reaches the wire
/// whole, with the row beside it counting them.
#[test]
fn a_plan_steps_drone_records_four_tasks_and_the_plan_reaches_the_wire() {
    let mut planned = Planned::created("fix the reader's bound");
    let call = called("record_plan", FOUR_TASKS);
    assert_eq!(call.tool, "record_plan");
    planned.kept(call, PLAN, 1);

    let opened = received_detail(&planned.detail());
    let plan = opened.work_plan.expect("the plan crossed");
    assert_eq!(
        plan.approach,
        "Stop the reader at the end, then cover the bound"
    );
    assert_eq!(
        plan.tasks
            .iter()
            .map(|task| (task.id.as_str(), task.state.as_wire()))
            .collect::<Vec<_>>(),
        [
            ("T1", "open"),
            ("T2", "open"),
            ("T3", "open"),
            ("T4", "open")
        ]
    );
    assert_eq!(plan.tasks[0].scope, ["crates/store/src/read.rs"]);
    assert_eq!(plan.tasks[0].expects, "a test for the last row");
    assert!(plan.tasks[0].note.is_empty());
    assert!(
        plan.tasks[1].scope.is_empty(),
        "a task naming no paths sends none, rather than an empty entry"
    );
    assert!(
        matches!(&plan.recorded_by, ChangedBy::Step { step_id, attempt: 1 } if step_id.as_str() == PLAN),
        "the plan says which run of which step recorded it"
    );
    let counted = opened.job.tasks.expect("the nested row counts the plan");
    assert_eq!(
        (counted.done, counted.working, counted.open, counted.dropped),
        (0, 0, 4, 0)
    );
}


/// Implement completes two tasks and drops one with a reason. The row counts
/// them, the detail carries the reason, and the event says the plan moved.
#[test]
fn implement_completes_two_tasks_and_drops_one_with_a_reason() {
    let mut planned = Planned::created("fix the reader's bound");
    planned.kept(called("record_plan", FOUR_TASKS), PLAN, 1);
    for arguments in [
        r#"{"task":"T1","state":"working","reason":"","shown":""}"#,
        r#"{"task":"T1","state":"done","reason":"","shown":""}"#,
        r#"{"task":"T2","state":"done","reason":"","shown":""}"#,
        r#"{"task":"T3","state":"dropped","reason":"the writer's bound was never inclusive","shown":""}"#,
    ] {
        planned.kept(called("update_task", arguments), IMPLEMENT, 1);
    }

    let tasks = received_row(&planned.row())
        .tasks
        .expect("a row with a plan counts it");
    assert_eq!(
        (tasks.done, tasks.working, tasks.open, tasks.dropped),
        (2, 0, 1, 1)
    );
    assert_eq!(
        (tasks.done, tasks.done + tasks.working + tasks.open),
        (2, 3),
        "2 of 3: a dropped task is not counted against the figure"
    );

    let plan = received_detail(&planned.detail())
        .work_plan
        .expect("the plan crossed");
    assert_eq!(plan.tasks[2].state.as_wire(), "dropped");
    assert_eq!(
        plan.tasks[2].reason.as_deref(),
        Some("the writer's bound was never inclusive")
    );
    assert_eq!(plan.tasks[3].state.as_wire(), "open");

    // A done task may be reopened; a dropped one stays dropped, and says so.
    let reopened = r#"{"task":"T1","state":"working","reason":"","shown":""}"#;
    assert!(planned
        .judged(called("update_task", reopened), IMPLEMENT, 1)
        .is_ok());
    let undropped = r#"{"task":"T3","state":"open","reason":"","shown":""}"#;
    assert_eq!(
        planned.judged(called("update_task", undropped), IMPLEMENT, 1),
        Err(core_model::PlanRefused::StaysDropped {
            named: core_model::TaskId::read("T3").expect("an id")
        })
    );

    let event = Event::JobPlanChanged(JobPlanChanged {
        job_id: planned.job.id().into(),
        tasks,
        task: None,
        state: None,
        group: None,
        actor: core_model::Actor::Drone.into(),
        at: planned.row().created_at,
    });
    assert_eq!(event.kind(), "job.plan_changed");
    assert_eq!(received_event(&event), event);
}

/// A drop with no reason is refused by name, as the tool error the Drone reads,
/// and the plan it would have changed is as it was.
#[test]
fn a_drop_with_no_reason_is_refused_by_name_and_changes_nothing() {
    let mut planned = Planned::created("fix the reader's bound");
    planned.kept(called("record_plan", FOUR_TASKS), PLAN, 1);
    let before = received_detail(&planned.detail()).work_plan;

    let (why, sent) = refused(
        "update_task",
        r#"{"task":"T2","state":"dropped","reason":"","shown":""}"#,
    );
    assert_eq!(
        why,
        NotAnArgument::Planning(PlanArgument::NotAnUpdate(
            NotAnUpdate::DroppedWithoutAReason
        ))
    );
    assert!(sent.contains("needs a `reason`"), "{sent}");
    assert!(sent.contains(r#""isError":true"#), "{sent}");
    assert_eq!(received_detail(&planned.detail()).work_plan, before);
}

/// The plan step's retry replaces the plan whole; a following step's retry
/// records nothing, so the states it left stand.
#[test]
fn a_plan_steps_retry_replaces_the_plan_and_a_following_steps_retry_keeps_its_states() {
    let mut planned = Planned::created("fix the reader's bound");
    let first_try =
        r#"{"approach":"A first try","tasks":[{"title":"One","note":"","scope":[],"expects":""}]}"#;
    planned.kept(called("record_plan", first_try), PLAN, 1);
    planned.kept(called("record_plan", FOUR_TASKS), PLAN, 2);
    let plan = received_detail(&planned.detail())
        .work_plan
        .expect("a plan");
    assert_eq!(plan.tasks.len(), 4, "the second run's plan, whole");
    assert!(matches!(
        plan.recorded_by,
        ChangedBy::Step { attempt: 2, .. }
    ));

    let done = r#"{"task":"T1","state":"done","reason":"","shown":""}"#;
    planned.kept(called("update_task", done), IMPLEMENT, 1);
    let working = r#"{"task":"T2","state":"working","reason":"","shown":""}"#;
    planned.kept(called("update_task", working), IMPLEMENT, 2);
    let tasks = received_row(&planned.row()).tasks.expect("counted");
    assert_eq!((tasks.done, tasks.working, tasks.open), (1, 1, 2));
}




