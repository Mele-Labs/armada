//! What a Commander's act reads as in the thread: the figure it was taken on.
//!
//! A door call carries its path segments at the top level and its fields under
//! `body`. Each case is one such call as the harness records it.

use adapter_traits::DroneEvent;

use crate::transcript::read;

fn detail_of(tool: &str, input: &str) -> String {
    let line = format!(
        r#"{{"type":"assistant","message":{{"content":[
             {{"type":"tool_use","id":"a","name":"{tool}","input":{input}}}]}}}}"#
    );
    let read = read(&line);
    let [DroneEvent::Called { detail, .. }] = &read[..] else {
        panic!("{read:?}");
    };
    detail.shown().to_string()
}

#[test]
fn a_turn_cap_raise_carries_the_new_cap() {
    assert_eq!(
        detail_of(
            "mcp__armada__raise_turn_cap",
            r#"{"job_id":"01JOB9","body":{"turn_cap":600}}"#
        ),
        "600"
    );
}

#[test]
fn a_cost_cap_raise_carries_the_new_ceiling_in_dollars() {
    let raise = |micros: u64| {
        detail_of(
            "mcp__armada-fleet__raise_cost_cap",
            &format!(r#"{{"job_id":"01JOB9","body":{{"cost_cap_micros":{micros}}}}}"#),
        )
    };
    assert_eq!(raise(12_000_000), "$12");
    assert_eq!(raise(12_500_000), "$12.50");
}

#[test]
fn a_task_restart_and_an_edit_carry_the_task() {
    for tool in ["mcp__armada__restart_task", "mcp__armada__edit_task"] {
        assert_eq!(
            detail_of(tool, r#"{"job_id":"01JOB9","task_id":"T7","body":{"title":"Narrower"}}"#),
            "T7",
            "{tool}"
        );
    }
}

#[test]
fn a_repair_carries_what_broke() {
    assert_eq!(
        detail_of(
            "mcp__armada__send_repair_drone",
            r#"{"job_id":"01JOB9","body":{"broke":"merge_conflict"}}"#
        ),
        "merge_conflict"
    );
}

#[test]
fn an_added_dropped_or_moved_task_carries_what_it_was_done_to() {
    assert_eq!(
        detail_of(
            "mcp__armada__add_task",
            r#"{"job_id":"01JOB9","body":{"title":"Draw the ask","after":"T7"}}"#
        ),
        "Draw the ask"
    );
    assert_eq!(
        detail_of(
            "mcp__armada__drop_task",
            r#"{"job_id":"01JOB9","body":{"task":"T4","reason":"done elsewhere"}}"#
        ),
        "T4"
    );
    assert_eq!(
        detail_of(
            "mcp__armada__move_plan",
            r#"{"job_id":"01JOB9","body":{"group":"G3","task":"T4","after":"T9"}}"#
        ),
        "T4 to G3"
    );
    assert_eq!(
        detail_of(
            "mcp__armada__move_plan",
            r#"{"job_id":"01JOB9","body":{"group":"G3"}}"#
        ),
        "G3"
    );
}

#[test]
fn a_redirect_to_one_drone_carries_the_drone() {
    assert_eq!(
        detail_of(
            "mcp__armada__redirect_one_drone",
            r#"{"job_id":"01JOB9","drone_id":"01DRONE","body":{"instruction":"stop"}}"#
        ),
        "01DRONE"
    );
}

#[test]
fn an_act_that_sent_no_figure_keeps_the_job_it_was_on() {
    assert_eq!(
        detail_of("mcp__armada__raise_turn_cap", r#"{"job_id":"01JOB9"}"#),
        "01JOB9"
    );
    assert_eq!(
        detail_of("mcp__armada__restart_step", r#"{"job_id":"01JOB9","body":{}}"#),
        "01JOB9"
    );
}

/// Another tool's `body` or `task_id` is its own: a string, a number or a list
/// there leaves its row as it was and its line readable.
#[test]
fn another_tools_body_and_task_id_are_left_alone() {
    assert_eq!(
        detail_of(
            "mcp__github__create_issue",
            r#"{"query":"bug","body":"a long text","task_id":{"n":1}}"#
        ),
        "bug"
    );
    assert_eq!(
        detail_of("mcp__other__thing", r#"{"query":"q","body":["a",1],"task_id":7}"#),
        "q"
    );
}

/// A Drone's own `add_task` sends its fields flat, with no `body`, and is not a
/// Commander's act.
#[test]
fn a_drones_own_add_task_is_not_read_as_a_commanders() {
    assert_eq!(
        detail_of("mcp__armada__add_task", r#"{"title":"Tidy","after":"T1"}"#),
        ""
    );
}
