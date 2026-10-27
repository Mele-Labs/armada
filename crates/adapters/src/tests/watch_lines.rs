//! The readers Armada ships for its watch recipes, folded from answers written here, and the line
//! each prints read back as Fleet reads it.

use core_model::ReadingState;

use crate::watch_lines::buildkite::{self, Build, Job};
use crate::watch_lines::github::{self, CheckRun, Status};

fn run(name: &str, status: &str, conclusion: Option<&str>) -> CheckRun {
    CheckRun {
        name: name.to_string(),
        status: Some(status.to_string()),
        conclusion: conclusion.map(str::to_string),
        html_url: Some(format!("https://example.test/runs/{name}")),
        details_url: None,
    }
}

fn status(context: &str, state: &str) -> Status {
    Status { context: context.to_string(), state: state.to_string(), target_url: None }
}

fn step(name: &str, state: &str) -> Job {
    Job {
        kind: String::from("script"),
        name: Some(name.to_string()),
        label: None,
        step_key: None,
        state: Some(state.to_string()),
        soft_failed: false,
        retried: false,
        web_url: None,
    }
}

fn build(state: &str, jobs: Vec<Job>) -> Build {
    Build { number: Some(412), state: state.to_string(), web_url: Some(String::from("https://example.test/b/412")), jobs }
}

#[test]
fn check_runs_and_statuses_outside_the_prefix_are_not_read() {
    let read = github::reading(
        vec![run("deploy-staging", "completed", Some("success")), run("lint", "completed", Some("failure"))],
        vec![status("deploy-canary", "pending"), status("coverage", "failure")],
        "0123456789abcdef",
        "deploy",
    );
    let names: Vec<&str> = read.parts.iter().map(|part| part.name.as_str()).collect();
    assert_eq!(names, ["deploy-staging", "deploy-canary"]);
    assert_eq!(read.state, ReadingState::Running);
    assert!(!read.finished);
    assert!(read.says.contains("0123456"), "{}", read.says);
}

#[test]
fn a_commit_whose_every_part_passed_is_finished_and_one_skipped_is_failed() {
    let passed = github::reading(vec![run("deploy", "completed", Some("success"))], vec![status("deploy-qa", "success")], "abc", "deploy");
    assert_eq!((passed.state, passed.finished), (ReadingState::Passed, true));
    let skipped = github::reading(vec![run("deploy", "completed", Some("skipped"))], Vec::new(), "abc", "deploy");
    assert_eq!((skipped.state, skipped.finished), (ReadingState::Failed, true));
    assert_eq!(skipped.link.as_deref(), Some("https://example.test/runs/deploy"));
}

#[test]
fn nothing_matching_yet_is_waiting_and_not_finished() {
    let read = github::reading(Vec::new(), Vec::new(), "abc", "deploy");
    assert_eq!((read.state, read.finished), (ReadingState::Waiting, false));
}

#[test]
fn a_build_reads_its_latest_attempt_and_a_soft_failure_as_passed() {
    let mut retried = step("rspec", "failed");
    retried.retried = true;
    let mut soft = step("audit", "failed");
    soft.soft_failed = true;
    let read = buildkite::reading(build("running", vec![retried, step("rspec", "running"), soft]), "web");
    let parts: Vec<(&str, ReadingState)> = read.parts.iter().map(|part| (part.name.as_str(), part.state)).collect();
    assert_eq!(parts, [("rspec", ReadingState::Running), ("audit", ReadingState::Passed)]);
    assert_eq!(read.link.as_deref(), Some("https://example.test/b/412"));
    assert!(!read.finished);
}

#[test]
fn a_step_skipped_waits_until_the_build_ends_and_fails_then() {
    let going = buildkite::reading(build("running", vec![step("deploy", "skipped")]), "web");
    assert_eq!(going.parts[0].state, ReadingState::Waiting);
    let ended = buildkite::reading(build("failed", vec![step("tests", "passed"), step("deploy", "skipped")]), "web");
    assert_eq!((ended.state, ended.finished), (ReadingState::Failed, true));
    assert!(ended.says.contains("deploy"), "{}", ended.says);
}

#[test]
fn a_build_that_ended_with_no_step_still_finishes() {
    let read = buildkite::reading(build("canceled", Vec::new()), "web");
    assert_eq!((read.state, read.finished), (ReadingState::Failed, true));
}

#[test]
fn the_line_a_reader_prints_is_the_line_fleet_reads() {
    let read = buildkite::reading(build("failed", vec![step("tests", "passed"), step("deploy", "failed")]), "web");
    let line = ipc::watch_line(&read).expect("a reading writes");
    assert_eq!(ipc::read_watch_line(&format!("noise first\n{line}\n")).expect("it reads"), read);
}

#[test]
fn a_reader_refuses_values_it_does_not_take() {
    let refused = crate::read_watch("github-checks", &[String::from("abc")], std::path::Path::new("/"));
    assert!(refused.is_err_and(|why| why.contains("sha, context")));
    assert!(crate::read_watch("nothing", &[], std::path::Path::new("/")).is_err());
}

#[test]
fn every_shipped_recipe_names_a_reader_armada_has() {
    for (path, text) in crate::carried_watch_recipes() {
        let named = crate::WATCH_READERS.iter().any(|(reader, _)| text.contains(&format!("watch-line, {reader},")));
        assert!(named, "{} names no reader", path.display());
    }
}
