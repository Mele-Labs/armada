//! A plan's groups and the record of their runs: minted at a recording, kept
//! through an add, moved by `after`, and run until passed or failed for good.

use crate::{
    Approach, Attempt, EscalationTrigger, FailReason, GroupId, GroupRuns, GroupState, NewTask,
    PlanAuthor, PlanChange, PlanEntry, PlanRefused, StepId, StepLevelTrigger, StepVerdict, TaskId,
    TaskState, TaskUpdate, Timestamp, WorkPlan,
};

fn at(second: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-10-02T10:00:{second:02}.000Z"))
}

fn person(change: PlanChange) -> PlanEntry {
    PlanEntry {
        change,
        by: PlanAuthor::Person,
        at: at(0),
    }
}

/// Tasks titled by number, each in the group the planner numbered, `0` naming
/// none.
fn recorded(groups: &[u32]) -> PlanEntry {
    person(PlanChange::Recorded {
        approach: Approach::new("Bound the reader, then cover it").expect("an approach"),
        tasks: groups
            .iter()
            .zip(1..)
            .map(|(group, n)| {
                NewTask::new(&format!("task {n}"), "", &[], "")
                    .expect("a title")
                    .in_group(*group)
            })
            .collect(),
    })
}

fn task(id: &str) -> TaskId {
    TaskId::read(id).expect("a task id")
}

fn group(id: &str) -> GroupId {
    GroupId::read(id).expect("a group id")
}

fn plan(history: &[PlanEntry]) -> Result<WorkPlan, PlanRefused> {
    WorkPlan::fold(history).map(|plan| plan.expect("a plan"))
}

fn placed(plan: &WorkPlan) -> Vec<(String, String)> {
    plan.tasks()
        .iter()
        .map(|t| (t.id().to_string(), t.group().to_string()))
        .collect()
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

#[test]
fn a_recording_without_groups_is_one_group() {
    let plan = plan(&[recorded(&[0, 0, 0])]).expect("replays");
    assert_eq!(plan.groups(), [GroupId::FIRST]);
    assert_eq!(
        placed(&plan),
        pairs(&[("T1", "G1"), ("T2", "G1"), ("T3", "G1")])
    );
}

/// The planner's numbers are its own; Fleet mints `G1`, `G2` in order, and a
/// task naming none joins the one before it.
#[test]
fn a_recording_mints_a_group_for_each_number_in_order() {
    let plan = plan(&[recorded(&[3, 0, 7, 7])]).expect("replays");
    assert_eq!(plan.groups(), [group("G1"), group("G2")]);
    assert_eq!(
        placed(&plan),
        pairs(&[("T1", "G1"), ("T2", "G1"), ("T3", "G2"), ("T4", "G2")])
    );
}

#[test]
fn a_recording_that_goes_back_a_group_is_refused() {
    assert_eq!(
        plan(&[recorded(&[2, 1])]),
        Err(PlanRefused::GroupsOutOfOrder { named: 1 })
    );
}

#[test]
fn an_added_task_joins_the_group_of_the_task_it_follows_or_the_last() {
    let new = || NewTask::new("added", "", &[], "").expect("a title");
    let plan = plan(&[
        recorded(&[1, 2]),
        person(PlanChange::Added {
            task: new(),
            after: Some(task("T1")),
        }),
        person(PlanChange::Added {
            task: new(),
            after: None,
        }),
    ])
    .expect("replays");
    assert_eq!(
        placed(&plan),
        pairs(&[("T1", "G1"), ("T3", "G1"), ("T2", "G2"), ("T4", "G2")])
    );
}

/// By `after`, never by index, into any group; absent is first.
#[test]
fn a_task_moves_after_the_task_named_or_first_in_its_new_group() {
    let history = [
        recorded(&[1, 1, 2]),
        person(PlanChange::MovedTask {
            task: task("T1"),
            group: group("G2"),
            after: Some(task("T3")),
        }),
        person(PlanChange::MovedTask {
            task: task("T2"),
            group: group("G2"),
            after: None,
        }),
    ];
    let plan = plan(&history).expect("replays");
    assert_eq!(
        placed(&plan),
        pairs(&[("T2", "G2"), ("T3", "G2"), ("T1", "G2")])
    );
    assert_eq!(plan.tasks_in(group("G1")).count(), 0, "G1 stays, empty");
}

#[test]
fn a_task_cannot_move_after_a_task_in_another_group() {
    let refused = plan(&[
        recorded(&[1, 1, 2]),
        person(PlanChange::MovedTask {
            task: task("T1"),
            group: group("G2"),
            after: Some(task("T2")),
        }),
    ]);
    assert_eq!(
        refused,
        Err(PlanRefused::NotInGroup {
            named: task("T2"),
            group: group("G2"),
        })
    );
}

#[test]
fn a_group_moves_and_takes_its_tasks_with_it() {
    let plan = plan(&[
        recorded(&[1, 2, 3]),
        person(PlanChange::MovedGroup {
            group: group("G3"),
            after: None,
        }),
        person(PlanChange::MovedGroup {
            group: group("G1"),
            after: Some(group("G2")),
        }),
    ])
    .expect("replays");
    assert_eq!(plan.groups(), [group("G3"), group("G2"), group("G1")]);
    assert_eq!(
        placed(&plan),
        pairs(&[("T3", "G3"), ("T2", "G2"), ("T1", "G1")])
    );
}

#[test]
fn a_failed_task_keeps_its_reason_apart_from_a_drop() {
    let reason = FailReason::new("G1's Checks were still red on run 3").expect("a reason");
    let plan = plan(&[
        recorded(&[1]),
        person(PlanChange::Updated {
            task: task("T1"),
            to: TaskUpdate::Failed(reason),
            shown: None,
        }),
    ])
    .expect("replays");
    let failed = plan.task(task("T1")).expect("T1");
    assert_eq!(failed.state(), TaskState::Failed);
    assert_eq!(
        failed.failed_reason(),
        Some("G1's Checks were still red on run 3")
    );
    assert_eq!(failed.reason(), None, "`reason` is a drop's");
    assert_eq!(
        TaskUpdate::stored("failed", "G1's Checks were still red on run 3")
            .map(|update| update.state()),
        Ok(TaskState::Failed),
        "a failure Fleet kept replays"
    );
}

/// Red runs open the next on their own; the third red ends it, and the record
/// says each run's step attempt and verdict.
#[test]
fn a_groups_runs_are_counted_and_its_state_read_off_them() {
    let plan = plan(&[recorded(&[1, 1])]).expect("replays");
    let (g1, step) = (GroupId::FIRST, StepId::new("implement"));
    let red = StepVerdict::Failed(
        StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger"),
    );
    let mut moves = Vec::new();
    let runs = GroupRuns::fold(&moves);
    assert_eq!(runs.state(g1, &plan), GroupState::Pending);
    assert_eq!(runs.spent(g1).number(), 1);
    for n in 1..=3u32 {
        let runs = GroupRuns::fold(&moves);
        moves.push(runs.opening(g1, &step, Attempt::stored(n).expect("one-based"), at(n)));
        let runs = GroupRuns::fold(&moves);
        assert_eq!(runs.spent(g1).number(), n);
        let expected = if n == 1 {
            GroupState::Running
        } else {
            GroupState::Retrying
        };
        assert_eq!(runs.state(g1, &plan), expected);
        moves.push(runs.closing(g1, red, None, at(n + 10)).expect("open"));
    }
    let runs = GroupRuns::fold(&moves);
    assert_eq!(runs.state(g1, &plan), GroupState::Failed);
    assert!(!runs.passed(g1));
    assert_eq!(runs.started_at(g1), Some(&at(1)));
    assert_eq!(runs.ended_at(g1), Some(&at(13)));
    let filed: Vec<u32> = runs.attempts(g1).map(|a| a.step_attempt.number()).collect();
    assert_eq!(filed, [1, 2, 3]);
    assert_eq!(runs.closing(g1, red, None, at(30)), None, "nothing is open");
}
