//! An error Bridge showed the person, as it reaches the Operator: with the Operator's session not running it
//! reaches nobody and stays where it is, and the same fault is one id in a local day.

use ipc::operator_errors::{BridgeErrorField, ReportBridgeError};

use super::autopilot_decisions_repository::jobs_titled;
use super::operator_sources::{rulings, switched};
use super::session_host::rig_fitted;
use crate::operator::fingerprint;

const SUBJECT: &str = "Build failed: restart exited 1";

fn card(code: &str, message: &str, job: Option<&str>) -> ReportBridgeError {
    ReportBridgeError {
        code: code.into(),
        message: message.into(),
        fields: vec![BridgeErrorField { key: "exit".into(), value: "1".into() }],
        chain: vec!["restart exited 1".into(), "no such file".into()],
        logs: vec!["/tmp/restart.log".into()],
        commit: Some("e1e0644c02".into()),
        job_id: job.map(str::to_string),
        at: "2026-11-06T09:00:00.000Z".into(),
    }
}

#[test]
fn what_differs_between_two_of_the_same_fault_is_not_in_its_fingerprint() {
    let same = |one: &str, other: &str| assert_eq!(fingerprint("c", one), fingerprint("c", other), "{one} / {other}");
    same("Build failed: restart exited 1", "Build failed: restart exited 137");
    same("Cannot read /Users/user/a/run.log", "Cannot read /tmp/b.log");
    same("Worktree 01JABC123XYZ is gone", "Worktree 01JQQQ999ZZZ is gone");
    same("Commit deadbeef12 is missing", "Commit cafebabe99 is missing");
    same("Retry 3 of 5 failed", "Retry 4 of 5 failed");

    assert_ne!(fingerprint("c", "Build failed: restart exited 1"), fingerprint("c", "Build failed: restart was killed"));
    assert_ne!(fingerprint("c", "Build failed"), fingerprint("d", "Build failed"));
    let print = fingerprint("c", "Build failed");
    assert!(print.len() == 16 && print.chars().all(|one| one.is_ascii_hexdigit()), "{print}");
}

#[tokio::test]
async fn an_error_is_taken_and_reaches_nobody_while_the_operators_session_is_not_running() {
    let rig = rig_fitted(|_| {});
    rig.agents_start("at_once");
    switched(&rig, "features.operator", true).await;

    let taken = rig.fleet.bridge_error_taken(card("bridge.fleet.build_failed", SUBJECT, None)).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert!(!taken.repeated);
    let parts: Vec<&str> = taken.id.split(':').collect();
    assert!(parts.len() == 3 && parts[0] == "error" && parts[1].len() == 16 && parts[2].len() == 10, "{}", taken.id);
    assert!(rulings(&rig).await.is_empty());
    assert_eq!(jobs_titled(&rig, SUBJECT).await, 0, "no Job starts from an error");
}

#[tokio::test]
async fn while_the_operator_is_off_an_error_is_accepted_and_nothing_arrives() {
    let rig = rig_fitted(|_| {});
    switched(&rig, "features.operator", false).await;

    let taken = rig.fleet.bridge_error_taken(card("bridge.fleet.build_failed", SUBJECT, None)).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert!(!taken.repeated);
    assert!(rulings(&rig).await.is_empty());
}
