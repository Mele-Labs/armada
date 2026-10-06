//! What `armada land --status` tells a branch whose Check failed while the
//! turn still runs. The Python suite drives the same through real git.

use crate::land::outcome::TOGETHER;
use crate::land::{
    exit_for, heads_up, merge_outcome, Outcome, OutcomePatch, OutcomeState, StateDir, HEADS_UP,
};
use crate::tests::TempDir;

fn held(detail: &str, own: &[&str], state: OutcomeState) -> Outcome {
    let dir = TempDir::new();
    let at = StateDir::for_testing(dir.path().join("armada-land"));
    merge_outcome(
        &at,
        "fix/x",
        state,
        detail,
        "09:00:00",
        OutcomePatch {
            logs: Some(vec![
                "/logs/test.log".to_string(),
                "/logs/ui.log".to_string(),
            ]),
            own_failures: Some(own.iter().map(|name| name.to_string()).collect()),
            ..OutcomePatch::default()
        },
    )
    .expect("an outcome")
}

#[test]
fn a_failure_the_base_is_green_for_gets_its_own_code_and_names_the_log() {
    let held = held("running ui (test, ui)", &["test"], OutcomeState::Gating);
    assert_eq!(exit_for(&held, "main"), HEADS_UP);
    let said = heads_up(&held, "main");
    assert_eq!(said.len(), 1);
    assert!(said[0].starts_with("test failed (log /logs/test.log); main is green for it."));
    assert!(said[0].contains("still running its other Checks; do not push this branch"));
}

#[test]
fn a_turn_with_no_such_failure_keeps_the_old_codes() {
    for (state, code) in [
        (OutcomeState::Waiting, 3),
        (OutcomeState::Gating, 3),
        (OutcomeState::Merging, 3),
        (OutcomeState::Red, 4),
        (OutcomeState::Stopped, 7),
    ] {
        let held = held("x", &[], state);
        assert_eq!(exit_for(&held, "main"), code);
        assert!(heads_up(&held, "main").is_empty());
    }
}

#[test]
fn a_finished_turn_is_its_verdict_whatever_it_failed_on_the_way() {
    let held = held("red", &["test"], OutcomeState::Red);
    assert_eq!(exit_for(&held, "main"), 4);
}

#[test]
fn a_batch_is_told_it_is_a_heads_up_for_the_whole_batch() {
    let detail = format!("running ui (test, ui){TOGETHER}fix/y");
    let held = held(&detail, &["test"], OutcomeState::Gating);
    assert_eq!(exit_for(&held, "main"), HEADS_UP);
    let said = &heads_up(&held, "main")[0];
    assert!(said.contains("failed in this batch"));
    assert!(said.contains("heads-up for the whole batch"));
    assert!(said.contains("the split will name the branch at fault"));
}
