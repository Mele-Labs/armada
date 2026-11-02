//! `armada job`'s reading of the door: what it lists, what it sends and what it makes of the answer.
//!
//! The list is written by `door::answer`, so what is read here is what a Fleet sends and not a
//! copy of its shape.

use serde_json::{json, Value};

use crate::door::{answer, Answered, Shape, ASKS_A_PERSON};
use crate::job_cli::{acts, list_message, outcome, Act, Outcome};
use crate::door::{read, Asked, CallId};

fn tools_id() -> CallId {
    match read(list_message().as_bytes(), &[]) {
        Asked::Tools { id } => id,
        other => panic!("the list message is not a tools/list: {other:?}"),
    }
}

fn listed() -> Vec<Act> {
    let shapes = vec![
        Shape { operation: "get_events_since", method: "GET", path: "/events/since", query: &["since"] },
        Shape { operation: "list_sessions", method: "GET", path: "/sessions", query: &["q", "state"] },
        Shape { operation: "waiting_for", method: "POST", path: "/sessions/waiting", query: &[] },
        Shape { operation: "start_another_session", method: "POST", path: "/sessions/start_another", query: &[] },
        Shape { operation: ASKS_A_PERSON, method: "POST", path: "/sessions/permission", query: &[] },
    ];
    let text = answer(Answered::Tools {
        id: tools_id(),
        shapes,
        always_loads: None,
    })
    .expect("a list encodes");
    acts(text.as_bytes()).expect("the list reads")
}

fn act<'a>(all: &'a [Act], name: &str) -> &'a Act {
    all.iter().find(|act| act.name == name).unwrap_or_else(|| panic!("no act {name}"))
}

fn pairs(words: &[(&str, &str)]) -> Vec<(String, String)> {
    words.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
}

fn arguments(message: &str) -> Value {
    let message: Value = serde_json::from_str(message).expect("a call is JSON");
    assert_eq!(message["method"], "tools/call");
    message["params"]["arguments"].clone()
}

#[test]
fn the_list_is_the_door_s_with_the_permission_tool_left_out() {
    let all = listed();
    assert!(all.iter().all(|act| act.name != ASKS_A_PERSON), "the harness's tool is not an act");
    assert_eq!(
        all.iter().map(|act| act.name.as_str()).collect::<Vec<_>>(),
        ["get_events_since", "list_jobs", "waiting_for", "start_another_job"],
        "the door lists a Job's name where it has one"
    );
    let sessions = act(&all, "list_jobs");
    assert_eq!(sessions.fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>(), ["q", "state"]);
    assert!(!sessions.takes_body);
    assert!(act(&all, "waiting_for").takes_body);
}

#[test]
fn a_name_the_schema_lists_is_a_top_level_argument_and_any_other_is_in_the_body() {
    let all = listed();
    let query = arguments(
        &act(&all, "list_jobs").call_message(&pairs(&[("state", "ended")]), None, None).expect("a query"),
    );
    assert_eq!(query, json!({ "state": "ended" }));

    let body = arguments(
        &act(&all, "start_another_job")
            .call_message(&pairs(&[("title", "Fix the login"), ("manifest_id", "web")]), None, None)
            .expect("a command"),
    );
    assert_eq!(body, json!({ "body": { "title": "Fix the login", "manifest_id": "web" } }));
}

#[test]
fn a_json_argument_is_the_body_and_a_name_value_word_is_added_to_it() {
    let all = listed();
    let call = act(&all, "waiting_for")
        .call_message(&pairs(&[("note", "x")]), Some(r#"{"items":[{"id":"a","text":"Restart"}]}"#), None)
        .expect("a body");
    assert_eq!(
        arguments(&call),
        json!({ "body": { "items": [{ "id": "a", "text": "Restart" }], "note": "x" } })
    );
    assert!(act(&all, "waiting_for").call_message(&[], Some("[1]"), None).is_err(), "an array is not a body");
    assert!(act(&all, "waiting_for").call_message(&[], Some("{"), None).is_err(), "text is not a body");
}

#[test]
fn a_value_is_text_unless_it_is_a_boolean_a_number_or_json() {
    let all = listed();
    let call = act(&all, "waiting_for")
        .call_message(
            &pairs(&[
                ("on", "true"),
                ("seconds", "30"),
                ("list", r#"["a"]"#),
                ("title", "2024-notes"),
                ("quoted", r#""30""#),
                ("broken", "{not json"),
            ]),
            None,
            None,
        )
        .expect("values");
    assert_eq!(
        arguments(&call)["body"],
        json!({ "on": true, "seconds": 30, "list": ["a"], "title": "2024-notes", "quoted": "30", "broken": "{not json" })
    );
}

#[test]
fn a_read_that_is_given_a_field_it_does_not_take_is_refused_with_the_ones_it_does() {
    let all = listed();
    let why = act(&all, "list_jobs")
        .call_message(&pairs(&[("title", "x")]), None, None)
        .expect_err("not a field");
    assert!(why.contains("`title` is not a field of `list_jobs`") && why.contains("q, state"), "{why}");
    let why = act(&all, "get_events_since")
        .call_message(&[], Some(r#"{"a":1}"#), None)
        .expect_err("no body");
    assert!(why.contains("takes no body"), "{why}");
}

#[test]
fn the_jobs_own_id_goes_where_the_act_takes_one_and_never_over_what_was_given() {
    let all = listed();
    let command = act(&all, "waiting_for");
    assert_eq!(
        arguments(&command.call_message(&[], None, Some("job-1")).expect("a command"))["body"]["session_id"],
        "job-1"
    );
    assert_eq!(
        arguments(&command.call_message(&pairs(&[("session_id", "other")]), None, Some("job-1")).expect("given"))
            ["body"]["session_id"],
        "other"
    );
    let read = act(&all, "list_jobs").call_message(&[], None, Some("job-1")).expect("a read");
    assert_eq!(arguments(&read), json!({}), "a read that names no session is not given one");
}

#[test]
fn a_refusal_is_the_tool_error_or_the_rpc_error_and_either_way_fleet_s_sentence() {
    let done = json!({"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"ok\":true}"}],"isError":false}});
    assert_eq!(outcome(done.to_string().as_bytes()), Outcome::Done(r#"{"ok":true}"#.to_string()));

    let refused = json!({"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"not this job's to do"}],"isError":true}});
    assert_eq!(outcome(refused.to_string().as_bytes()), Outcome::Refused("not this job's to do".to_string()));

    let rpc = json!({"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"no such method: x"}});
    assert_eq!(outcome(rpc.to_string().as_bytes()), Outcome::Refused("no such method: x".to_string()));

    assert!(matches!(outcome(b"<html>"), Outcome::Refused(_)));
}
