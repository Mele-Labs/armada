//! A hosted Job's Check run names the `Tool` row of the agent's call that ran it, read off the
//! session's own stream: the one shell call running `armada check` as the run began, foreground or
//! backgrounded, and nothing where there is none or several.

use adapter_traits::{CallDetail, DroneEvent, TaskStatus};
use ipc::{SessionCheckAct, SessionCheckCall, SessionCheckState, SessionId, SessionRow};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::check_calls::runs_armada_check;
use crate::tests::slot_leases::{leased, By};

fn called(tool: &str, call: &str, command: &str) -> DroneEvent {
    DroneEvent::Called { tool: tool.into(), call: call.into(), detail: CallDetail::of(command) }
}

fn answered(call: &str) -> DroneEvent {
    DroneEvent::Answered { call: call.into(), failed: false }
}

/// A Job whose process is running, holding slot 2. Answers the Job and the slot's path.
async fn hosted_in_a_slot(rig: &Rig) -> (SessionId, String) {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    let slot = leased(&rig.root, 2, "fleet/topic", By::Job(id.as_str()));
    (id, slot)
}

/// The stream says `events`, then a marker call whose row says they were all heard.
async fn heard(rig: &Rig, id: &SessionId, mut events: Vec<DroneEvent>, marker: &str) {
    events.push(called("Read", marker, &format!("/tmp/{marker}")));
    rig.stand_in.says(0, events);
    eventually(|| async { tool_row(rig, id, &format!("Read /tmp/{marker}")).await.is_some() }).await;
}

async fn tool_row(rig: &Rig, id: &SessionId, text: &str) -> Option<String> {
    rig.fleet.rows_of(id.as_str()).await.ok()?.into_iter().find_map(|row| match row {
        SessionRow::Tool { id, text: said, .. } if said == text => Some(id),
        _ => None,
    })
}

async fn started(rig: &Rig, slot: &str) -> u64 {
    let call = SessionCheckCall {
        act: SessionCheckAct::Start,
        cwd: slot.into(),
        name: "test".into(),
        target: None,
        run: None,
        outcome: None,
        took_ms: None,
        log: None,
    };
    rig.fleet.session_check_reported(call).await.expect("begins").run.expect("the Job holds the slot")
}

async fn linked(rig: &Rig, id: &SessionId, run: u64) -> Option<String> {
    let rows = rig.fleet.rows_of(id.as_str()).await.expect("the thread");
    rows.into_iter()
        .find_map(|row| match row {
            SessionRow::Check { run: at, tool_row, .. } if at == run => Some(tool_row),
            _ => None,
        })
        .expect("a row for the run")
}

#[tokio::test]
async fn a_run_names_the_one_call_running_armada_check_and_keeps_it_as_it_ends() {
    let rig = rig_placing(None);
    let (id, slot) = hosted_in_a_slot(&rig).await;
    heard(&rig, &id, vec![called("Bash", "toolu_1", "armada check test reads")], "m1").await;
    let row = tool_row(&rig, &id, "Bash armada check test reads").await.expect("the call's row");

    let run = started(&rig, &slot).await;
    assert_eq!(linked(&rig, &id, run).await.as_deref(), Some(row.as_str()));

    let end = SessionCheckCall {
        act: SessionCheckAct::End,
        cwd: slot.clone(),
        name: "test".into(),
        target: None,
        run: Some(run),
        outcome: Some(SessionCheckState::Passed),
        took_ms: Some(10),
        log: Some(String::new()),
    };
    rig.fleet.session_check_reported(end).await.expect("ends");
    assert_eq!(linked(&rig, &id, run).await.as_deref(), Some(row.as_str()), "the ended row keeps it");
}

#[tokio::test]
async fn a_run_with_no_such_call_running_names_none() {
    let rig = rig_placing(None);
    let (id, slot) = hosted_in_a_slot(&rig).await;
    // One that ran `armada check` and answered, and one running something else.
    heard(
        &rig,
        &id,
        vec![called("Bash", "toolu_1", "armada check test reads"), answered("toolu_1"), called("Bash", "toolu_2", "cargo build")],
        "m1",
    )
    .await;
    let run = started(&rig, &slot).await;
    assert_eq!(linked(&rig, &id, run).await, None);
}

#[tokio::test]
async fn a_run_with_two_such_calls_running_names_none() {
    let rig = rig_placing(None);
    let (id, slot) = hosted_in_a_slot(&rig).await;
    heard(
        &rig,
        &id,
        vec![called("Bash", "toolu_1", "armada check test reads"), called("Bash", "toolu_2", "armada check test writes")],
        "m1",
    )
    .await;
    let run = started(&rig, &slot).await;
    assert_eq!(linked(&rig, &id, run).await, None);
}

/// The harness answers a backgrounded call before its command has begun: the task it started holds
/// it until that task ends.
#[tokio::test]
async fn a_backgrounded_call_is_named_until_its_task_ends() {
    let rig = rig_placing(None);
    let (id, slot) = hosted_in_a_slot(&rig).await;
    let backgrounded = DroneEvent::TaskStarted {
        task: "b1".into(),
        call: "toolu_1".into(),
        description: "tests".into(),
        backgrounded: true,
    };
    heard(
        &rig,
        &id,
        vec![called("Bash", "toolu_1", "git diff --name-only | armada check test --changed"), answered("toolu_1"), backgrounded],
        "m1",
    )
    .await;
    let row = tool_row(&rig, &id, "Bash git diff --name-only | armada check test --changed").await.expect("the call's row");
    let run = started(&rig, &slot).await;
    assert_eq!(linked(&rig, &id, run).await.as_deref(), Some(row.as_str()));

    let ended = DroneEvent::TaskEnded { task: "b1".into(), status: TaskStatus::Completed, output: None, exit: Some(0) };
    heard(&rig, &id, vec![ended], "m2").await;
    let later = started(&rig, &slot).await;
    assert_eq!(linked(&rig, &id, later).await, None);
}

#[test]
fn a_command_runs_armada_check_by_its_words() {
    for runs in [
        "armada check test reads",
        "cd crates && armada check typecheck",
        "git diff --name-only main | armada check test --changed",
        "taskpolicy -c utility ./target/debug/armada check test a b",
    ] {
        assert!(runs_armada_check(runs), "{runs}");
    }
    for runs_not in ["armada job status", "cargo check -p fleet", "armada worktree lease check", "echo armadacheck"] {
        assert!(!runs_armada_check(runs_not), "{runs_not}");
    }
}
