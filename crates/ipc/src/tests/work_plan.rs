//! A Job's plan as Bridge receives it: whole on the detail, counted on the row,
//! pointed at by `job.plan_changed` — and absent, never null, where there is none.

use core_model::{
    Approach, Attempt, DropReason, NewTask, PlanAuthor, PlanChange, PlanEntry, StepId, TaskId,
    TaskUpdate, Timestamp,
};

use crate::tests::job;
use crate::{decode, encode, ChangedBy, Event, JobPlanChanged, JobSummary, WorkPlan};

fn a_plan() -> core_model::WorkPlan {
    let at = Timestamp::from_rfc3339("2026-09-13T10:00:00.000Z");
    let by = PlanAuthor::Step {
        step_id: StepId::new("plan"),
        attempt: Attempt::FIRST,
    };
    let entries = [
        PlanEntry {
            change: PlanChange::Recorded {
                approach: Approach::new("Bound the reader").expect("an approach"),
                tasks: vec![
                    NewTask::new("Stop at the end", "read.rs:41", &[], "").expect("a title"),
                    NewTask::new("Cover the bound", "", &[], "").expect("a title"),
                ],
            },
            by: by.clone(),
            at: at.clone(),
        },
        PlanEntry {
            change: PlanChange::Updated {
                task: TaskId::read("T2").expect("an id"),
                to: TaskUpdate::Dropped(DropReason::new("T1's test covers it").expect("a reason")),
                shown: None,
            },
            by: PlanAuthor::Person,
            at,
        },
    ];
    core_model::WorkPlan::fold(&entries)
        .expect("replays")
        .expect("a plan")
}

#[test]
fn a_plan_crosses_whole_and_reads_back_as_itself() {
    let sent = WorkPlan::from(&a_plan());
    let body = encode(&sent).expect("plain data");
    let received: WorkPlan = decode("a plan", body.as_bytes()).expect("reads back");

    assert_eq!(received, sent);
    assert_eq!(
        received.recorded_by,
        ChangedBy::Step {
            step_id: crate::StepId::from(&StepId::new("plan")),
            attempt: 1
        }
    );
    assert!(body.contains(r#""by":"step""#), "{body}");
    assert_eq!(received.tasks[0].id, "T1");
    assert_eq!(received.tasks[1].state.as_wire(), "dropped");
    assert_eq!(
        received.tasks[1].reason.as_deref(),
        Some("T1's test covers it")
    );
    assert!(
        !body.contains(r#""detail":"""#) && !body.contains("null"),
        "an empty detail and an absent reason are left out: {body}"
    );
}

#[test]
fn a_task_marked_working_carries_its_windows_and_one_never_marked_carries_none() {
    let mut history = vec![PlanEntry {
        change: PlanChange::Recorded {
            approach: Approach::new("Bound the reader").expect("an approach"),
            tasks: vec![
                NewTask::new("Stop at the end", "", &[], "").expect("a title"),
                NewTask::new("Cover the bound", "", &[], "").expect("a title"),
            ],
        },
        by: PlanAuthor::Person,
        at: Timestamp::from_rfc3339("2026-09-13T10:00:00.000Z"),
    }];
    for (to, at) in [
        (TaskUpdate::Working, "2026-09-13T10:01:00.000Z"),
        (TaskUpdate::Done, "2026-09-13T10:02:00.000Z"),
        (TaskUpdate::Working, "2026-09-13T10:03:00.000Z"),
    ] {
        history.push(PlanEntry {
            change: PlanChange::Updated {
                task: TaskId::read("T1").expect("an id"),
                to,
                shown: None,
            },
            by: PlanAuthor::Person,
            at: Timestamp::from_rfc3339(at),
        });
    }
    let plan = core_model::WorkPlan::fold(&history)
        .expect("replays")
        .expect("a plan");
    let body = encode(&WorkPlan::from(&plan)).expect("plain data");
    let received: WorkPlan = decode("a plan", body.as_bytes()).expect("reads back");

    let windows = &received.tasks[0].working_windows;
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].entered.as_str(), "2026-09-13T10:01:00.000Z");
    assert_eq!(
        windows[0].left.as_ref().map(|at| at.as_str()),
        Some("2026-09-13T10:02:00.000Z")
    );
    assert_eq!(windows[1].left, None, "still working, so no end is sent");
    assert!(received.tasks[1].working_windows.is_empty());
    assert_eq!(
        body.matches("working_windows").count(),
        1,
        "left out on the task never marked: {body}"
    );
}

#[test]
fn a_row_with_no_plan_carries_no_task_field() {
    let summary = JobSummary::from(&job());
    let body = encode(&summary).expect("plain data");
    assert!(!body.contains("tasks"), "{body}");

    let mut counted = summary;
    counted.tasks = Some(a_plan().counts().into());
    let body = encode(&counted).expect("plain data");
    let received: JobSummary = decode("a row", body.as_bytes()).expect("reads back");
    let tasks = received.tasks.expect("the counts crossed");
    assert_eq!((tasks.open, tasks.dropped), (1, 1));
}

#[test]
fn a_plan_change_travels_under_the_name_the_inventory_declares() {
    let event = Event::JobPlanChanged(JobPlanChanged::recorded(
        job().id(),
        &a_plan(),
        core_model::Actor::Drone,
        &Timestamp::from_rfc3339("2026-09-13T10:01:00.000Z"),
    ));
    assert_eq!(event.kind(), "job.plan_changed");
    let body = encode(&event).expect("plain data");
    let received: Event = decode("an event", body.as_bytes()).expect("reads back");
    assert_eq!(received, event);
}

/// 23.1: a change that moved one task names it and its new state, and a whole
/// recording names neither, so a 23.0 body reads exactly as it did.
#[test]
fn a_task_that_moved_is_named_with_its_state_and_a_recording_names_none() {
    let at = Timestamp::from_rfc3339("2026-09-13T10:01:00.000Z");
    let recorded = JobPlanChanged::recorded(job().id(), &a_plan(), core_model::Actor::Drone, &at);
    let body = encode(&recorded).expect("plain data");
    assert!(
        !body.contains("\"task\"") && !body.contains("\"state\""),
        "{body}"
    );

    let t1 = core_model::TaskId::read("T1").expect("an id");
    let moved =
        JobPlanChanged::task_moved(job().id(), &a_plan(), t1, core_model::Actor::Fleet, &at);
    let body = encode(&moved).expect("plain data");
    let received: JobPlanChanged = decode("a plan change", body.as_bytes()).expect("reads back");
    assert_eq!(received.task.as_deref(), Some("T1"));
    assert_eq!(
        received.state.map(|state| state.as_wire()),
        a_plan().task(t1).map(|task| task.state().as_wire())
    );
}

/// 22.0's two counts are left out at zero, so a row with no handed-in or failed
/// task reads exactly as it did before them, and present where one stands.
#[test]
fn the_two_new_counts_are_absent_at_zero_and_carried_when_a_task_stands_there() {
    let none = encode(&crate::TaskCounts::default()).expect("encodes");
    assert!(
        !none.contains("handed_in") && !none.contains("failed"),
        "{none}"
    );
    let before: crate::TaskCounts = decode(
        "task counts",
        br#"{"done":1,"working":0,"open":2,"dropped":0}"#,
    )
    .expect("a Fleet before 22.0's counts still read");
    assert_eq!((before.handed_in, before.failed), (0, 0));
    let some = crate::TaskCounts {
        handed_in: 2,
        failed: 1,
        ..crate::TaskCounts::default()
    };
    let text = encode(&some).expect("encodes");
    assert!(
        text.contains(r#""handed_in":2"#) && text.contains(r#""failed":1"#),
        "{text}"
    );
}

/// Both new states cross under the spellings Bridge matches on.
#[test]
fn a_task_state_crosses_as_its_registry_spelling() {
    for (state, said) in [
        (core_model::TaskState::HandedIn, "\"handed_in\""),
        (core_model::TaskState::Failed, "\"failed\""),
    ] {
        assert_eq!(
            encode(&crate::TaskState::from(state)).expect("encodes"),
            said
        );
    }
}
