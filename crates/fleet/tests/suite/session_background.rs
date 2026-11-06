//! A hosted session's `armada check` runs in the background, so the person can reach the
//! session while it runs. #2415.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{GateAnswer, SessionGate, SessionId};

use super::session_host::{denied, reason, rig_placing, Rig};

async fn bash(rig: &Rig, id: &SessionId, input: &str) -> GateAnswer {
    rig.gate(id, "Bash", input).await
}

#[tokio::test]
async fn a_lone_check_is_moved_into_the_background_and_otherwise_kept() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    let answer = bash(
        &rig,
        &id,
        r#"{"command":"armada check test x 2>&1 | tail -25","description":"Run tests"}"#,
    )
    .await;

    let GateAnswer::Rewrite { rewritten } = answer else {
        panic!("not rewritten: {answer:?}");
    };
    assert_eq!(rewritten.decision, "allow");
    assert_eq!(rewritten.input["run_in_background"], true);
    assert_eq!(rewritten.input["command"], "armada check test x 2>&1 | tail -25");
    assert_eq!(rewritten.input["description"], "Run tests");
}

/// In the background the commit would run unseen.
#[tokio::test]
async fn a_check_chained_to_a_write_is_refused_and_told_to_run_alone() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    let answer = bash(&rig, &id, r#"{"command":"armada check test x && git commit -m y"}"#).await;

    assert!(denied(&answer), "{answer:?}");
    assert!(reason(&answer).contains("line of its own"), "{}", reason(&answer));
}

#[tokio::test]
async fn a_call_already_in_the_background_or_with_no_check_is_not_rewritten() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    for input in [
        r#"{"command":"armada check test x","run_in_background":true}"#,
        r#"{"command":"git status"}"#,
    ] {
        let answer = bash(&rig, &id, input).await;
        assert!(!matches!(answer, GateAnswer::Rewrite { .. }), "{input}: {answer:?}");
    }
}

/// A subagent handed a background result ends its turn and reports before the check ends.
#[tokio::test]
async fn a_subagents_check_stays_in_the_foreground() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let call = format!(
        r#"{{"session_id":"{}","cwd":"{}","tool_name":"Bash","tool_input":{{"command":"armada check test x"}},"agent_id":"a1"}}"#,
        id.as_str(),
        rig.root,
    );
    let gate: SessionGate = ipc::decode("a gate", call.as_bytes()).expect("a gate");

    let answer = Arc::clone(&rig.fleet).gate_session_call(gate).await.expect("answered");

    assert!(!matches!(answer, GateAnswer::Rewrite { .. }), "{answer:?}");
}
