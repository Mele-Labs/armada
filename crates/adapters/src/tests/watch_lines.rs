//! The readers Armada ships for its watch recipes, folded from answers written here, and the line
//! each prints read back as Fleet reads it.

use std::collections::BTreeMap;

use core_model::ReadingState;

use crate::watch_lines::buildkite::{self, Build, Job};
use crate::watch_lines::github::{self, Annotation, CheckRun, Status};

fn run(name: &str, status: &str, conclusion: Option<&str>) -> CheckRun {
    CheckRun {
        id: None,
        name: name.to_string(),
        status: Some(status.to_string()),
        conclusion: conclusion.map(str::to_string),
        html_url: Some(format!("https://example.test/runs/{name}")),
        details_url: None,
        output: None,
    }
}

fn status(context: &str, state: &str) -> Status {
    Status { context: context.to_string(), state: state.to_string(), target_url: None, description: None }
}

fn step(name: &str, state: &str) -> Job {
    Job {
        id: Some(format!("job-{name}")),
        kind: String::from("script"),
        name: Some(name.to_string()),
        label: None,
        step_key: None,
        state: Some(state.to_string()),
        soft_failed: false,
        retried: false,
        web_url: None,
        log_url: None,
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
        &BTreeMap::new(),
    );
    let names: Vec<&str> = read.parts.iter().map(|part| part.name.as_str()).collect();
    assert_eq!(names, ["deploy-staging", "deploy-canary"]);
    assert_eq!(read.state, ReadingState::Running);
    assert!(!read.finished);
    assert!(read.says.contains("0123456"), "{}", read.says);
}

#[test]
fn a_commit_whose_every_part_passed_is_finished_and_one_skipped_is_failed() {
    let passed = github::reading(vec![run("deploy", "completed", Some("success"))], vec![status("deploy-qa", "success")], "abc", "deploy", &BTreeMap::new());
    assert_eq!((passed.state, passed.finished), (ReadingState::Passed, true));
    let skipped = github::reading(vec![run("deploy", "completed", Some("skipped"))], Vec::new(), "abc", "deploy", &BTreeMap::new());
    assert_eq!((skipped.state, skipped.finished), (ReadingState::Failed, true));
    assert_eq!(skipped.link.as_deref(), Some("https://example.test/runs/deploy"));
}

#[test]
fn nothing_matching_yet_is_waiting_and_not_finished() {
    let read = github::reading(Vec::new(), Vec::new(), "abc", "deploy", &BTreeMap::new());
    assert_eq!((read.state, read.finished), (ReadingState::Waiting, false));
}

#[test]
fn a_build_reads_its_latest_attempt_and_a_soft_failure_as_passed() {
    let mut retried = step("rspec", "failed");
    retried.retried = true;
    let mut soft = step("audit", "failed");
    soft.soft_failed = true;
    let read = buildkite::reading(build("running", vec![retried, step("rspec", "running"), soft]), "web", &BTreeMap::new());
    let parts: Vec<(&str, ReadingState)> = read.parts.iter().map(|part| (part.name.as_str(), part.state)).collect();
    assert_eq!(parts, [("rspec", ReadingState::Running), ("audit", ReadingState::Passed)]);
    assert_eq!(read.link.as_deref(), Some("https://example.test/b/412"));
    assert!(!read.finished);
}

#[test]
fn a_step_skipped_waits_until_the_build_ends_and_fails_then() {
    let going = buildkite::reading(build("running", vec![step("deploy", "skipped")]), "web", &BTreeMap::new());
    assert_eq!(going.parts[0].state, ReadingState::Waiting);
    let ended = buildkite::reading(build("failed", vec![step("tests", "passed"), step("deploy", "skipped")]), "web", &BTreeMap::new());
    assert_eq!((ended.state, ended.finished), (ReadingState::Failed, true));
    assert!(ended.says.contains("deploy"), "{}", ended.says);
}

#[test]
fn a_build_that_ended_with_no_step_still_finishes() {
    let read = buildkite::reading(build("canceled", Vec::new()), "web", &BTreeMap::new());
    assert_eq!((read.state, read.finished), (ReadingState::Failed, true));
}

#[test]
fn the_line_a_reader_prints_is_the_line_fleet_reads() {
    let read = buildkite::reading(build("failed", vec![step("tests", "passed"), step("deploy", "failed")]), "web", &BTreeMap::new());
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

#[derive(serde::Deserialize)]
struct LogAnswer {
    content: String,
}

#[derive(serde::Deserialize)]
struct RunsAnswer {
    check_runs: Vec<CheckRun>,
}

/// A failed step carries its own link, the exact error lines in its log, and the end of the log.
#[test]
fn a_failed_step_gives_its_link_its_error_lines_and_the_end_of_its_log() {
    let build: Build = ipc::decode("a fixture", include_str!("fixtures/watch-build-failed.json").as_bytes()).expect("a build");
    let log: LogAnswer = ipc::decode("a fixture", include_str!("fixtures/watch-job-log.json").as_bytes()).expect("a log");
    let logs = BTreeMap::from([(String::from("j-provision"), log.content)]);
    let read = buildkite::reading(build, "web", &logs);
    let provision = read.parts.iter().find(|part| part.name == "provision").expect("provision");
    assert_eq!(provision.state, ReadingState::Failed);
    assert_eq!(provision.link.as_deref(), Some("https://ci.example.test/acme-shop/web/builds/5120#j-provision"));
    assert_eq!(
        provision.error.as_deref(),
        Some("Error: cloud lookup for region qa-east returned no account\nProvisioning failed after 3 attempts"),
        "exact lines, colour and timestamp marks taken out"
    );
    let tail = provision.tail.as_deref().expect("the end of its log");
    assert_eq!(tail.lines().count(), 50);
    assert!(tail.ends_with("Downloading module 100%\nProvisioning failed after 3 attempts"), "a redrawn line keeps what it last read: {tail}");
    let build = read.parts.iter().find(|part| part.name == "build").expect("build");
    assert_eq!((build.error.as_deref(), build.tail.as_deref()), (None, None), "a passed step carries no error");
}

/// A failed run gives its annotations as `path:line: message`, else its own title and summary.
#[test]
fn a_failed_check_run_gives_its_annotations_or_its_output() {
    let runs: RunsAnswer = ipc::decode("a fixture", include_str!("fixtures/watch-check-runs.json").as_bytes()).expect("runs");
    let marked: Vec<Annotation> = ipc::decode("a fixture", include_str!("fixtures/watch-annotations.json").as_bytes()).expect("annotations");
    let read = github::reading(runs.check_runs, Vec::new(), "7c1e0b4aa", "deploy", &BTreeMap::from([(901, marked)]));
    let part = |name: &str| read.parts.iter().find(|part| part.name == name).expect(name);
    let qa = part("deploy-qa");
    assert_eq!(qa.link.as_deref(), Some("https://forge.example.test/acme-shop/web/runs/901"));
    assert_eq!(qa.error.as_deref(), Some("infra/qa/provision.tf:12: Provision: cloud lookup for region qa-east returned no account"), "the failure annotation only");
    assert!(qa.tail.as_deref().is_some_and(|tail| tail.ends_with("returned no account\nexit 1")));
    assert_eq!(part("deploy-docs").error.as_deref(), Some("Docs deploy failed\nThe upload was refused."), "no annotations: its title and summary");
    assert_eq!(part("deploy-build").error, None);
}

#[test]
fn a_parts_evidence_survives_the_line_fleet_reads() {
    let build: Build = ipc::decode("a fixture", include_str!("fixtures/watch-build-failed.json").as_bytes()).expect("a build");
    let log: LogAnswer = ipc::decode("a fixture", include_str!("fixtures/watch-job-log.json").as_bytes()).expect("a log");
    let read = buildkite::reading(build, "web", &BTreeMap::from([(String::from("j-provision"), log.content)]));
    let line = ipc::watch_line(&read).expect("a reading writes");
    assert_eq!(ipc::read_watch_line(&line).expect("it reads"), read);
}
