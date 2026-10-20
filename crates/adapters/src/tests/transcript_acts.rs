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

/// What a thread row says for the call: the tool, then the detail the way the session host writes it,
/// the whole where the row was cut.
fn row_of(tool: &str, input: &str) -> String {
    let line = format!(
        r#"{{"type":"assistant","message":{{"content":[
             {{"type":"tool_use","id":"a","name":"{tool}","input":{input}}}]}}}}"#
    );
    let read = read(&line);
    let [DroneEvent::Called { tool, detail, .. }] = &read[..] else {
        panic!("{read:?}");
    };
    format!("{tool} {}", detail.whole().unwrap_or(detail.shown()))
}

// Written as the row will carry it: `title`, `scene`, `against`, and the scene's keys in order.
const SKETCH: &str = r#"{"title":"Request flow","scene":{"edges":[],"nodes":[{"id":"a","kind":"box","title":"Bridge","x":0,"y":0}]}}"#;

#[test]
fn a_show_sketch_row_carries_the_call_s_arguments_unwrapped_from_the_door_s_body() {
    let row = row_of(
        "mcp__armada-fleet__show_sketch",
        &format!(r#"{{"body":{SKETCH}}}"#),
    );
    assert_eq!(row, format!("mcp__armada-fleet__show_sketch {SKETCH}"));
}

#[test]
fn a_show_sketch_row_names_what_it_is_drawn_against_and_leaves_the_session_id_out() {
    let row = row_of(
        "show_sketch",
        r#"{"body":{"session_id":"S1","title":"After","against":"Before","scene":{"nodes":[],"edges":[]}}}"#,
    );
    assert_eq!(
        row,
        r#"show_sketch {"title":"After","scene":{"edges":[],"nodes":[]},"against":"Before"}"#
    );
}

#[test]
fn a_show_sketch_scene_past_the_row_s_bound_is_kept_whole_in_the_row() {
    let nodes: Vec<String> = (0..60)
        .map(|n| format!(r#"{{"id":"n{n}","kind":"box","x":{n},"y":0,"body":"{}"}}"#, "word ".repeat(20).trim()))
        .collect();
    let scene = format!(r#"{{"nodes":[{}],"edges":[]}}"#, nodes.join(","));
    let row = row_of(
        "mcp__armada-fleet__show_sketch",
        &format!(r#"{{"body":{{"title":"Big","scene":{scene}}}}}"#),
    );
    let text = row.strip_prefix("mcp__armada-fleet__show_sketch ").unwrap();
    assert!(text.chars().count() > 5_000, "{}", text.chars().count());
    assert!(text.starts_with(r#"{"title":"Big","scene":{"#) && text.ends_with("}]}}"), "{}", &text[text.len() - 40..]);
    assert!(text.contains(r#""id":"n59""#), "the last node is in the row");
}

#[test]
fn a_show_sketch_call_with_no_title_or_no_scene_has_no_detail() {
    for body in [r#"{"scene":{"nodes":[],"edges":[]}}"#, r#"{"title":"Flow"}"#, r#""just text""#] {
        assert_eq!(detail_of("show_sketch", &format!(r#"{{"body":{body}}}"#)), "", "{body}");
    }
}

#[test]
fn another_tool_with_a_title_and_a_scene_keeps_its_own_reading() {
    // `add_task` reads its title, and a `scene` beside it is not carried.
    assert_eq!(
        detail_of(
            "mcp__armada__add_task",
            r#"{"job_id":"01JOB9","body":{"title":"Narrower","scene":{"nodes":[]}}}"#
        ),
        "Narrower"
    );
    assert_eq!(
        row_of("mcp__armada-fleet__show_window", r#"{"body":{"url":"http://localhost:5173/","title":"Page"}}"#),
        "mcp__armada-fleet__show_window "
    );
}

/// A `sign_in` is a login, so the line a thread keeps of the call never carries it.
#[test]
fn a_show_window_call_keeps_nothing_of_its_sign_in() {
    assert_eq!(
        row_of(
            "mcp__armada-fleet__show_window",
            r#"{"body":{"url":"https://staging.example.com/","sign_in":["https://staging.example.com/dev/login?token=abc"]}}"#
        ),
        "mcp__armada-fleet__show_window "
    );
}

#[test]
fn a_show_file_row_carries_the_path_as_the_agent_gave_it() {
    assert_eq!(
        row_of("mcp__armada-fleet__show_file", r#"{"body":{"path":"docs/notes.md","session_id":"S1"}}"#),
        "mcp__armada-fleet__show_file docs/notes.md"
    );
    assert_eq!(detail_of("show_file", r#"{"body":{"path":"/tmp/a.log"}}"#), "/tmp/a.log");
}

#[test]
fn a_show_file_call_with_no_path_has_no_detail_and_another_tool_s_path_is_not_read() {
    assert_eq!(detail_of("show_file", r#"{"body":{}}"#), "");
    assert_eq!(detail_of("mcp__armada__add_task", r#"{"job_id":"01JOB9","body":{"title":"T","path":"/x"}}"#), "T");
}
