//! The line Fleet adds ahead of a message that names something, as a Job is told it.

use ipc::{SessionTag, TagKind};

use crate::session_host::mention_line;

fn tag(kind: TagKind, id: &str, title: &str, url: Option<&str>) -> (SessionTag, Option<String>) {
    (
        SessionTag {
            kind,
            id: id.to_string(),
            title: title.to_string(),
            url: url.map(str::to_string),
        },
        None,
    )
}

#[test]
fn an_issue_is_named_by_number_title_and_address() {
    let line = mention_line(&[tag(
        TagKind::Issue,
        "214",
        "The title",
        Some("https://forge.invalid/o/r/issues/214"),
    )]);
    assert_eq!(
        line.as_deref(),
        Some(r#"The person named: issue #214 "The title" (https://forge.invalid/o/r/issues/214)."#)
    );
}

#[test]
fn an_issue_named_by_a_trackers_key_is_said_as_the_key_without_a_hash() {
    let line = mention_line(&[tag(TagKind::Issue, "ABC-12", "Walk it", Some("https://acme.invalid/browse/ABC-12"))]);
    assert_eq!(
        line.as_deref(),
        Some(r#"The person named: issue ABC-12 "Walk it" (https://acme.invalid/browse/ABC-12)."#)
    );
}

#[test]
fn the_number_bridge_puts_in_a_title_is_not_said_twice() {
    let line = mention_line(&[
        tag(TagKind::Issue, "219", "#219 Walk it", None),
        tag(TagKind::PullRequest, "220", "#220 Land it", None),
    ])
    .expect("a line");
    assert_eq!(
        line,
        r#"The person named: issue #219 "Walk it"; pull request #220 "Land it"."#
    );
}

#[test]
fn a_title_that_only_looks_like_the_prefix_is_left_alone() {
    let line = mention_line(&[tag(TagKind::Issue, "21", "#219 Walk it", None)]).expect("a line");
    assert_eq!(line, r##"The person named: issue #21 "#219 Walk it"."##);
}

#[test]
fn a_pull_request_carries_its_address_only_when_it_has_one() {
    let with = mention_line(&[tag(
        TagKind::PullRequest,
        "5",
        "Five",
        Some("https://forge.invalid/o/r/pull/5"),
    )])
    .expect("a line");
    assert_eq!(
        with,
        r#"The person named: pull request #5 "Five" (https://forge.invalid/o/r/pull/5)."#
    );
    let without = mention_line(&[tag(TagKind::PullRequest, "5", "Five", None)]).expect("a line");
    assert_eq!(without, r#"The person named: pull request #5 "Five"."#);
}

#[test]
fn a_reported_bug_is_named_by_its_title() {
    let line = mention_line(&[tag(TagKind::Report, "01jreport", "Preview restart drops the lease", None)]);
    assert_eq!(
        line.as_deref(),
        Some(r#"The person named: reported bug "Preview restart drops the lease"."#)
    );
}

#[test]
fn a_terminal_is_named_by_its_id_and_name_and_the_line_says_to_pass_the_id() {
    let line = mention_line(&[tag(TagKind::Terminal, "t2", "Terminal 2", None)]);
    assert_eq!(
        line.as_deref(),
        Some(r#"The person named: terminal t2 "Terminal 2" (pass t2 as `terminal` to type_in_terminal or read_terminal)."#)
    );
}

#[test]
fn a_tool_is_said_as_the_armada_tool_the_agent_uses() {
    let line = mention_line(&[tag(TagKind::Tool, "takes", "Takes", None)]);
    assert_eq!(
        line.as_deref(),
        Some("The person asked for Takes: put the options to them with `add_part` of kind `takes`, one scene each, your pick first.")
    );
}

#[test]
fn the_api_client_names_each_requests_tool_the_agent_can_call() {
    let line = mention_line(&[tag(TagKind::Tool, "api-client", "API Client", None)]);
    assert_eq!(
        line.as_deref(),
        Some("The person asked for API Client: send it with `send_request` or `send_requests`, save it with `save_request_to_collection`, and read saved requests and their runs with `get_requests`, `list_request_runs` and `get_request_run`.")
    );
}

#[test]
fn a_tool_follows_what_the_message_names_and_an_unknown_tool_says_nothing() {
    let line = mention_line(&[tag(TagKind::Tool, "sketch", "Sketch", None), tag(TagKind::Issue, "219", "#219 Walk it", None)]);
    assert_eq!(
        line.as_deref(),
        Some("The person named: issue #219 \"Walk it\".\nThe person asked for Sketch: draw it with `show_sketch` as a `mock` of the screen.")
    );
    assert_eq!(mention_line(&[tag(TagKind::Tool, "kettle", "Kettle", None)]), None);
}

#[test]
fn a_saved_request_is_said_by_its_collection_and_request_to_load_and_send() {
    let line = mention_line(&[tag(TagKind::Request, "c1/r7", "List orders", None)]);
    assert_eq!(
        line.as_deref(),
        Some("The person tagged the saved request \"List orders\": load it with `get_requests` (collection `c1`, request `r7`) and send it with `send_request` by that id.")
    );
}

#[test]
fn a_new_request_is_said_as_one_to_build_send_and_save() {
    let line = mention_line(&[tag(TagKind::Request, "new", "New request", None)]);
    assert_eq!(
        line.as_deref(),
        Some("The person asked for a new request: build it, send it with `send_request`, and save it with `save_request_to_collection`.")
    );
}

#[test]
fn a_request_whose_id_has_an_empty_half_says_nothing() {
    for id in ["/r7", "c1/", "/", "c1"] {
        assert_eq!(mention_line(&[tag(TagKind::Request, id, "List orders", None)]), None, "{id}");
    }
}

#[test]
fn a_request_follows_what_the_message_names_and_a_tool_comes_before_it() {
    let line = mention_line(&[
        tag(TagKind::Request, "c1/r7", "List orders", None),
        tag(TagKind::Tool, "sketch", "Sketch", None),
        tag(TagKind::Issue, "219", "#219 Walk it", None),
    ]);
    assert_eq!(
        line.as_deref(),
        Some("The person named: issue #219 \"Walk it\".\nThe person asked for Sketch: draw it with `show_sketch` as a `mock` of the screen.\nThe person tagged the saved request \"List orders\": load it with `get_requests` (collection `c1`, request `r7`) and send it with `send_request` by that id.")
    );
}
