//! The claim, written before the code: **a `checks_green` gate closes on Checks Fleet ran at the pushed
//! commit, and a test red at the base on main as well does not hold it.** `docs/concepts/session/the-plan.md`,
//! *Gates*.

use ipc::{PlanGate, PlanGroupInput, SessionId, SessionPlanGroup, SessionRecord};

use super::session_host::{rig_placing, Rig};
use super::session_plan::{group, plan, read, recorded, running};
use crate::gate_checks::{verdict, Asked, Ended, GateRuns, Verdict};

const SHA: &str = "1b500c4e9a7d3f20c8b1a64d5e7f0392a1c4b8de";

fn names(of: &[&str]) -> Vec<String> {
    of.iter().map(|name| name.to_string()).collect()
}

#[test]
fn every_test_green_closes_and_so_does_a_change_that_touched_no_test() {
    assert_eq!(verdict("1b500c4", &Ended::Passed), Verdict::Green { fact: "Checks passed at 1b500c4".into(), already_red: Vec::new() });
    assert_eq!(
        verdict("1b500c4", &Ended::NoTestTouched),
        Verdict::Green { fact: "Checks passed at 1b500c4: no test touched".into(), already_red: Vec::new() }
    );
}

#[test]
fn a_test_red_here_and_not_at_the_base_holds_the_gate() {
    let ended = Ended::Failed { failed: names(&["tests::a", "tests::b"]), red_at_base: names(&["tests::a"]) };
    assert_eq!(verdict("1b500c4", &ended), Verdict::Red { failed: names(&["tests::b"]), already_red: names(&["tests::a"]) });
}

#[test]
fn only_tests_already_red_on_main_close_it_and_are_counted() {
    let ended = Ended::Failed { failed: names(&["tests::a", "tests::b"]), red_at_base: names(&["tests::b", "tests::a", "tests::z"]) };
    assert_eq!(
        verdict("1b500c4", &ended),
        Verdict::Green { fact: "Checks passed at 1b500c4, 2 already red on main".into(), already_red: names(&["tests::a", "tests::b"]) }
    );
}

#[test]
fn a_failure_that_names_no_test_holds_the_gate() {
    let ended = Ended::Failed { failed: Vec::new(), red_at_base: Vec::new() };
    assert_eq!(verdict("1b500c4", &ended), Verdict::Red { failed: Vec::new(), already_red: Vec::new() });
}

#[test]
fn a_newer_push_replaces_the_jobs_queued_run_and_a_repository_runs_one_at_a_time() {
    let runs = GateRuns::default();
    let push = |job: &str, branch: &str| Asked { job: job.into(), worktree: format!("/w/{job}"), branch: Some(branch.into()) };
    runs.asked("armada", push("j1", "first"));
    runs.asked("armada", push("j2", "other"));
    runs.asked("armada", push("j1", "second"));
    assert_eq!(runs.due(), ["armada"]);
    assert!(runs.due().is_empty(), "it is going already");
    assert_eq!(runs.next("armada"), Some(push("j1", "second")), "the newer push stands in the older one's place");
    runs.asked("armada", push("j3", "late"));
    assert_eq!(runs.next("armada"), Some(push("j2", "other")));
    assert_eq!(runs.next("armada"), Some(push("j3", "late")));
    assert_eq!(runs.next("armada"), None);
    runs.asked("armada", push("j4", "again"));
    assert_eq!(runs.due(), ["armada"], "a push after it ended starts it again");
}

fn gated(title: &str, gate: PlanGate) -> PlanGroupInput {
    PlanGroupInput { gate, ..group(None, title, "rust-engineer", &["crates/fleet"], &[(None, &format!("Do {title}"))]) }
}

async fn first_group(rig: &Rig, id: &SessionId) -> SessionPlanGroup {
    let record: SessionRecord = read(rig, id).await;
    record.plan.expect("a plan").groups.remove(0)
}

async fn with_a_checks_gate(rig: &Rig) -> SessionId {
    let id = running(rig).await;
    recorded(rig, plan(&id, vec![gated("Build it", PlanGate::ChecksGreen)])).await;
    id
}

#[tokio::test]
async fn a_run_shows_as_running_then_green_closes_the_gate_and_takes_the_state_away() {
    let rig = rig_placing(None);
    let id = with_a_checks_gate(&rig).await;
    assert!(rig.fleet.gate_check_began(id.as_str(), SHA).await);
    let running = first_group(&rig, &id).await;
    let checks = running.gate_checks.expect("a run in flight");
    assert_eq!((checks.commit.as_str(), checks.state.as_str()), ("1b500c4", "running"));
    assert!(running.gate_closed.is_none());

    rig.fleet.gate_checked(id.as_str(), SHA, Ended::Passed).await;
    let closed = first_group(&rig, &id).await;
    let close = closed.gate_closed.expect("closed");
    assert_eq!((close.fact.as_str(), close.commit.as_deref()), ("Checks passed at 1b500c4", Some("1b500c4")));
    assert!(closed.gate_checks.is_none(), "absent once the gate closes");
}

#[tokio::test]
async fn a_red_run_names_what_failed_and_what_was_red_on_main_and_the_gate_stays_open() {
    let rig = rig_placing(None);
    let id = with_a_checks_gate(&rig).await;
    let ended = Ended::Failed { failed: names(&["tests::a", "tests::b"]), red_at_base: names(&["tests::a"]) };
    rig.fleet.gate_checked(id.as_str(), SHA, ended).await;
    let group = first_group(&rig, &id).await;
    assert!(group.gate_closed.is_none());
    let checks = group.gate_checks.expect("red");
    assert_eq!(checks.state, "red");
    assert_eq!((checks.failed, checks.already_red), (names(&["tests::b"]), names(&["tests::a"])));

    // The next push fixes it: it closes, with nothing left of the red run.
    rig.fleet.gate_checked(id.as_str(), "2c611d5f00aa", Ended::Passed).await;
    let group = first_group(&rig, &id).await;
    assert_eq!(group.gate_closed.map(|one| one.fact), Some("Checks passed at 2c611d5".to_string()));
    assert!(group.gate_checks.is_none());
}

#[tokio::test]
async fn only_tests_already_red_close_the_gate_and_the_close_names_them() {
    let rig = rig_placing(None);
    let id = with_a_checks_gate(&rig).await;
    let ended = Ended::Failed { failed: names(&["tests::a"]), red_at_base: names(&["tests::a"]) };
    rig.fleet.gate_checked(id.as_str(), SHA, ended).await;
    let close = first_group(&rig, &id).await.gate_closed.expect("closed");
    assert_eq!(close.fact, "Checks passed at 1b500c4, 1 already red on main");
    assert_eq!(close.already_red, names(&["tests::a"]));
}

#[tokio::test]
async fn a_run_for_a_gate_that_closed_meanwhile_starts_and_keeps_nothing() {
    let rig = rig_placing(None);
    let id = with_a_checks_gate(&rig).await;
    rig.fleet.gate_checked(id.as_str(), SHA, Ended::NoTestTouched).await;
    let closed = first_group(&rig, &id).await.gate_closed;
    assert!(!rig.fleet.gate_check_began(id.as_str(), "2c611d5f00aa").await, "its gate is closed");
    rig.fleet.gate_checked(id.as_str(), "2c611d5f00aa", Ended::Failed { failed: names(&["tests::a"]), red_at_base: Vec::new() }).await;
    let after = first_group(&rig, &id).await;
    assert_eq!(after.gate_closed, closed, "the first fact stands");
    assert!(after.gate_checks.is_none());
}
