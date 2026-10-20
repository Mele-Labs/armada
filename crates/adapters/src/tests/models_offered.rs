//! What the harness says it offers becomes the aliases, and nothing else does.
//!
//! The answers below are the shape the CLI sent, cut to the rows that matter. The
//! one case that starts a process starts a script standing in for the CLI, so
//! nothing here needs a credential or the network.

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use crate::models_offered::models_in;
use crate::HeadlessAgent;

use super::scratch::Scratch;

/// A default, four aliases and two pinned versions, as the CLI lists them.
const ANSWER: &str = concat!(
    r#"{"type":"control_response","response":{"subtype":"success","request_id":"models","response":{"commands":[],"models":["#,
    r#"{"value":"default","resolvedModel":"claude-sonnet-5-5","displayName":"Default (recommended)","description":"Sonnet 5.5","supportedEffortLevels":["low","high"]},"#,
    r#"{"value":"opus","resolvedModel":"claude-opus-5-5","displayName":"Opus 5.5","description":"For complex work","supportedEffortLevels":["low","medium","high","xhigh","max"]},"#,
    r#"{"value":"fable","resolvedModel":"claude-fable-5-1","displayName":"Fable 5.1","description":"For the toughest challenges","supportedEffortLevels":["low","max"]},"#,
    r#"{"value":"sonnet","resolvedModel":"claude-sonnet-5-5","displayName":"Sonnet 5.5","description":"Most efficient"},"#,
    r#"{"value":"haiku","resolvedModel":"claude-haiku-5-5","displayName":"Haiku 5.5","description":"Fastest"},"#,
    r#"{"value":"claude-opus-4-8","resolvedModel":"claude-opus-4-8","displayName":"Opus 4.8","description":"Pinned"},"#,
    r#"{"value":"claude-haiku-4-5-20251001","resolvedModel":"claude-haiku-4-5-20251001","displayName":"Haiku 4.5","description":"Pinned"}"#,
    r#"]}}}"#,
);

#[test]
fn an_answer_becomes_the_alias_rows_with_the_default_and_pinned_versions_dropped() {
    let rows = models_in(ANSWER.as_bytes()).expect("the answer").expect("read");
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["opus", "fable", "sonnet", "haiku"]);
    let opus = &rows[0];
    assert_eq!(opus.display, "Opus 5.5");
    assert_eq!(opus.version, "claude-opus-5-5");
    assert_eq!(opus.description, "For complex work");
    assert_eq!(opus.efforts, ["low", "medium", "high", "xhigh", "max"]);
    assert!(rows[2].efforts.is_empty(), "a row that lists no effort carries none");
}

#[test]
fn a_line_that_is_not_the_answer_is_passed_over() {
    for line in [
        r#"{"type":"system","subtype":"commands_changed"}"#,
        r#"{"type":"control_response","response":{"subtype":"success","request_id":"other","response":{"models":[]}}}"#,
        "not json at all",
    ] {
        assert!(models_in(line.as_bytes()).is_none(), "{line}");
    }
}

#[test]
fn an_answer_with_no_alias_in_it_says_so() {
    let only_pinned = r#"{"type":"control_response","response":{"subtype":"success","request_id":"models","response":{"models":[{"value":"claude-opus-4-8","resolvedModel":"claude-opus-4-8"}]}}}"#;
    let why = models_in(only_pinned.as_bytes()).expect("the answer").expect_err("none by an alias");
    assert!(why.contains("no model by an alias"), "{why}");
    let refused = r#"{"type":"control_response","response":{"subtype":"error","request_id":"models","error":"not signed in"}}"#;
    let why = models_in(refused.as_bytes()).expect("the answer").expect_err("refused");
    assert!(why.contains("not signed in"), "{why}");
}

/// A script that stands in for the CLI: it reads the question and prints `says`.
fn standing_in(scratch: &Scratch, says: &str) -> HeadlessAgent {
    scratch.file("answer.json", says);
    let script = scratch.file(
        "cli",
        &format!("#!/bin/sh\nread asked\ncat '{}'\necho\nsleep 30\n", scratch.path().join("answer.json").display()),
    );
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("executable");
    HeadlessAgent::at(script.to_string_lossy())
}

#[test]
fn a_program_that_answers_is_asked_and_stopped() {
    let scratch = Scratch::new();
    let agent = standing_in(&scratch, ANSWER);
    let started = std::time::Instant::now();
    let rows = agent.ask_models(Duration::from_secs(10)).expect("read");
    assert_eq!(rows.len(), 4);
    assert!(started.elapsed() < Duration::from_secs(10), "it did not wait out the script's sleep");
}

#[test]
fn a_program_that_never_answers_gives_up_with_the_time_it_waited() {
    let scratch = Scratch::new();
    let agent = standing_in(&scratch, r#"{"type":"system"}"#);
    let why = agent.ask_models(Duration::from_secs(1)).expect_err("nothing came back");
    assert!(why.contains("did not answer within 1 seconds"), "{why}");
}

#[test]
fn a_program_that_is_not_there_says_which() {
    let why = HeadlessAgent::at("/nowhere/at/all/cli")
        .ask_models(Duration::from_secs(1))
        .expect_err("not started");
    assert!(why.contains("/nowhere/at/all/cli"), "{why}");
}

#[test]
fn the_built_in_names_include_the_alias_the_constant_missed() {
    assert_eq!(HeadlessAgent::models(), ["opus", "fable", "sonnet", "haiku"]);
}
