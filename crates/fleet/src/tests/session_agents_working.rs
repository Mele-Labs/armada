//! A process with an agent still working is not ended between turns: a quiet spell and the move into
//! a slot both wait for it, and a hung one stops holding them. `docs/concepts/session/a-job-fleet-hosts.md`.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent, TaskStatus};
use api::{HostedSessions, Sessions};
use ipc::{AttachmentReport, SessionFact, SessionId, SessionReport, SessionRow, SessionTurn};
use super::planted::Held;
use super::session_host::{denied, eventually, rig_quiet_after, Rig};
use super::session_queue::{close, queued_and_idle, resumes, the_keeper, with_slot_one_held};
use super::tmp::TempDir;
use crate::session_host::Heard;

const AGENT: &str = "x1";

/// A rig whose processes are quiet at once, on a clock the test moves.
fn quiet_at_once() -> (Rig, Arc<Held>) {
    let clock = Arc::new(Held::started());
    let held = Arc::clone(&clock);
    let rig = rig_quiet_after(TempDir::new(), Duration::ZERO, move |fittings| fittings.clock = held);
    (rig, clock)
}

async fn turn_running(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    id
}

async fn idle(rig: &Rig, id: &SessionId) {
    eventually(|| async { rig.turn(id).await == SessionTurn::Idle }).await;
}

async fn agent_started(rig: &Rig, id: &SessionId) {
    let report = SessionReport {
        harness: adapters::HOSTED_HARNESS.into(),
        session_id: id.clone(),
        fact: SessionFact::Attached {
            attachment: AttachmentReport {
                kind: "subagent".into(),
                target: AGENT.into(),
                detail: [("type".to_string(), "explorer".to_string()), ("description".to_string(), "sweep".to_string())].into_iter().collect(),
            },
        },
    };
    rig.fleet.report_session(report).await.expect("kept");
}

/// The agent's transcript, ending at `at` on `reason`, in the file the CLI keeps for it.
fn transcript(rig: &Rig, id: &SessionId, reason: &str, at: &str) {
    let dir = std::path::Path::new(&rig.fleet.host().home).join(adapters::SESSIONS).join("a-project").join(id.as_str()).join("subagents");
    std::fs::create_dir_all(&dir).expect("its directory");
    let content = match reason {
        "end_turn" => r#"[{"type":"text","text":"Done."}]"#,
        _ => r#"[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/a"}}]"#,
    };
    let line = format!(
        r#"{{"type":"assistant","uuid":"s1","isSidechain":true,"agentId":"{AGENT}","timestamp":"{at}","message":{{"role":"x","stop_reason":"{reason}","content":{content}}}}}"#
    );
    std::fs::write(dir.join(format!("agent-{AGENT}.jsonl")), line + "\n").expect("its transcript");
}

fn agent_finished(rig: &Rig, id: &SessionId) {
    transcript(rig, id, "end_turn", "2026-08-26T00:00:30.000Z");
}

fn ended(rig: &Rig, index: usize) -> bool {
    rig.stand_in.process(index).ended.load(Ordering::SeqCst)
}

fn lines_to(rig: &Rig, index: usize) -> String {
    rig.stand_in.process(index).sent.lock().unwrap().join("\n")
}

fn write_to_main(rig: &Rig) -> String {
    format!(r#"{{"file_path":"{}/src/lib.rs"}}"#, rig.root)
}

/// The first write of a turn with an agent running: refused, and its turn over.
async fn stood_down_for_a_slot(rig: &Rig, id: &SessionId) {
    agent_started(rig, id).await;
    assert!(denied(&rig.gate(id, "Edit", &write_to_main(rig)).await));
    rig.stand_in.finishes(0, "I will stop here");
    idle(rig, id).await;
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().moving.as_ref().is_some_and(|moving| moving.held) }).await;
}

#[tokio::test]
async fn a_process_with_a_subagent_working_outlives_a_quiet_spell_and_ends_once_it_finishes() {
    let (rig, _) = quiet_at_once();
    let id = turn_running(&rig).await;
    agent_started(&rig, &id).await;
    rig.stand_in.finishes(0, "started it");
    idle(&rig, &id).await;

    rig.fleet.sweep_quiet().await;
    assert!(!ended(&rig, 0), "the subagent would be cut off");

    agent_finished(&rig, &id);
    rig.fleet.sweep_quiet().await;
    assert!(ended(&rig, 0), "nothing is working, and it has been quiet");
}

#[tokio::test]
async fn a_process_with_a_background_command_running_outlives_a_quiet_spell_and_ends_once_it_ends() {
    let (rig, _) = quiet_at_once();
    let id = turn_running(&rig).await;
    rig.stand_in.says(
        0,
        vec![
            DroneEvent::Called { tool: "Bash".into(), call: "toolu_1".into(), detail: CallDetail::of("make") },
            DroneEvent::TaskStarted { task: "t1".into(), call: "toolu_1".into(), description: "build".into(), backgrounded: true },
        ],
    );
    rig.stand_in.finishes(0, "started it");
    idle(&rig, &id).await;
    // The turn's end is heard after the task's start, so the row is there.
    rig.fleet.sweep_quiet().await;
    assert!(!ended(&rig, 0), "the command would be cut off");

    rig.stand_in.says(0, vec![DroneEvent::TaskEnded { task: "t1".into(), status: TaskStatus::Completed, output: None, exit: Some(0) }]);
    eventually(|| async {
        rig.fleet.sweep_quiet().await;
        ended(&rig, 0)
    })
    .await;
}

async fn running(rig: &Rig, id: &SessionId) -> Vec<ipc::RunningWork> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session.running
}

#[tokio::test]
async fn the_listing_of_a_job_idle_with_a_subagent_working_names_it_and_drops_it_once_it_finishes() {
    let (rig, _) = quiet_at_once();
    let id = turn_running(&rig).await;
    assert!(running(&rig, &id).await.is_empty(), "nothing is running");
    agent_started(&rig, &id).await;
    rig.stand_in.finishes(0, "started it");
    idle(&rig, &id).await;

    let working = running(&rig, &id).await;
    assert_eq!(working.len(), 1);
    assert_eq!((working[0].kind, working[0].what.as_str()), (ipc::RunningKind::Subagent, "sweep"));
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session;
    let line = ipc::SessionSummary::of(&record);
    assert_eq!(line.turn, Some(ipc::TurnState::Idle));
    assert_eq!(line.running, working);

    agent_finished(&rig, &id);
    assert!(running(&rig, &id).await.is_empty(), "it finished");
}

#[tokio::test]
async fn a_background_command_is_listed_by_its_title() {
    let (rig, _) = quiet_at_once();
    let id = turn_running(&rig).await;
    rig.stand_in.says(
        0,
        vec![
            DroneEvent::Called { tool: "Bash".into(), call: "toolu_1".into(), detail: CallDetail::of("make") },
            DroneEvent::TaskStarted { task: "t1".into(), call: "toolu_1".into(), description: "build".into(), backgrounded: true },
        ],
    );
    rig.stand_in.finishes(0, "started it");
    idle(&rig, &id).await;

    let working = running(&rig, &id).await;
    assert_eq!(working.len(), 1);
    assert_eq!((working[0].kind, working[0].what.as_str()), (ipc::RunningKind::BackgroundTask, "build"));
}

/// A subagent's transcript that never finishes must not hold the process for ever. Each line it writes
/// counts as work, so only a long silence lets go of it.
#[tokio::test]
async fn a_hung_subagent_stops_holding_its_process() {
    let (rig, clock) = quiet_at_once();
    let id = turn_running(&rig).await;
    agent_started(&rig, &id).await;
    rig.stand_in.finishes(0, "started it");
    idle(&rig, &id).await;

    clock.on(50 * 60);
    transcript(&rig, &id, "tool_use", "2026-08-26T00:50:00.000Z");
    clock.on(30 * 60);
    rig.fleet.sweep_quiet().await;
    assert!(!ended(&rig, 0), "it wrote half an hour ago, though it started over an hour ago");

    clock.on(40 * 60);
    rig.fleet.sweep_quiet().await;
    assert!(ended(&rig, 0), "over an hour since its last line");
}

/// One whose transcript never appeared is judged by when it started.
#[tokio::test]
async fn a_subagent_that_never_wrote_stops_holding_its_process_an_hour_after_it_started() {
    let (rig, clock) = quiet_at_once();
    let id = turn_running(&rig).await;
    agent_started(&rig, &id).await;
    rig.stand_in.finishes(0, "started it");
    idle(&rig, &id).await;

    rig.fleet.sweep_quiet().await;
    assert!(!ended(&rig, 0));

    clock.on(61 * 60);
    rig.fleet.sweep_quiet().await;
    assert!(ended(&rig, 0));
}

#[tokio::test]
async fn the_move_into_a_slot_waits_for_a_working_subagent_and_is_made_once_it_finishes() {
    let (rig, _) = quiet_at_once();
    let id = turn_running(&rig).await;
    stood_down_for_a_slot(&rig, &id).await;

    rig.fleet.sweep_quiet().await;
    rig.fleet.held_moves_made().await;
    assert!(!ended(&rig, 0), "the subagent would be cut off");
    assert_eq!(rig.stand_in.starts().len(), 1);
    assert!(denied(&rig.gate(&id, "Edit", &write_to_main(&rig)).await), "the move is still owed");

    agent_finished(&rig, &id);
    rig.fleet.held_moves_made().await;
    assert!(ended(&rig, 0));
    let moved = &rig.stand_in.starts()[1];
    assert!(moved.resuming, "the same session, resumed");
    assert_eq!(moved.directory, adapter_traits::slot_path(&rig.root, 1));
    let told = lines_to(&rig, 1);
    assert!(told.contains("You are now in"), "{told}");
    assert!(!told.contains("They stopped"), "nothing was cut off: {told}");
    assert!(
        !rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text == "I will stop here")),
        "what the agent said standing down is not shown"
    );
}

/// A turn while the move waits is an ordinary one, and the move is made at the end of the first with
/// nothing working.
#[tokio::test]
async fn a_turn_while_the_move_waits_is_shown_and_the_move_is_made_when_a_turn_ends_with_nothing_working() {
    let (rig, _) = quiet_at_once();
    let id = turn_running(&rig).await;
    stood_down_for_a_slot(&rig, &id).await;

    rig.send(&id, "how is it going").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "still sweeping");
    idle(&rig, &id).await;
    eventually(|| async { rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text == "still sweeping")) }).await;
    assert_eq!(rig.stand_in.starts().len(), 1, "the subagent is still working");

    agent_finished(&rig, &id);
    rig.send(&id, "and now").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "it reported back");
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    assert!(lines_to(&rig, 1).contains("You are now in"));
    eventually(|| async { rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text == "it reported back")) }).await;
}

/// The Job is not left in the checkout on a promise, and is told what the wait cost it.
#[tokio::test]
async fn a_hung_subagent_stops_holding_the_move_and_the_job_is_told_it_was_cut_off() {
    let (rig, clock) = quiet_at_once();
    let id = turn_running(&rig).await;
    stood_down_for_a_slot(&rig, &id).await;

    clock.on(61 * 60);
    rig.fleet.held_moves_made().await;
    assert_eq!(rig.stand_in.starts().len(), 2);
    let told = lines_to(&rig, 1);
    assert!(told.contains("You are now in") && told.contains("They stopped") && told.contains(AGENT), "{told}");
}

/// The agents went with the process, so the move waits on nothing and is still owed: the Job was
/// told it would hear.
#[tokio::test]
async fn a_process_that_dies_while_the_move_waits_is_moved_all_the_same() {
    let (rig, _) = quiet_at_once();
    let id = turn_running(&rig).await;
    stood_down_for_a_slot(&rig, &id).await;

    rig.stand_in.hears(0, Heard::Gone);
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().process.is_none() }).await;
    rig.fleet.held_moves_made().await;
    assert_eq!(rig.stand_in.starts()[1].directory, adapter_traits::slot_path(&rig.root, 1));
    assert!(lines_to(&rig, 1).contains("You are now in"));
}

/// Fleet restarts while the move waits on agents. The keeper carries the process across, still in
/// the checkout while its hosting names the slot, so the move is owed all the same and a relative
/// write is held rather than landing in main (`armada-f7s3`).
#[tokio::test]
async fn a_restart_while_the_move_waits_on_agents_still_owes_the_move_and_holds_relative_writes() {
    let (rig, _) = quiet_at_once();
    let id = turn_running(&rig).await;
    stood_down_for_a_slot(&rig, &id).await;

    let rig = rig.restarted_keeping_processes().await;
    assert!(rig.fleet.hosts().of(id.as_str()).state().process.is_some(), "the keeper's process is taken back");
    let relative = r#"{"file_path":"src/lib.rs"}"#;
    assert!(denied(&rig.gate_in(&rig.root, &id, "Edit", relative).await), "the write would land in main");
    assert!(move_waits(&rig, &id).await, "the wait for the move stands");

    agent_finished(&rig, &id);
    rig.fleet.held_moves_made().await;
    assert_eq!(rig.stand_in.starts()[1].directory, adapter_traits::slot_path(&rig.root, 1));
    assert!(lines_to(&rig, 1).contains("You are now in"));
    assert!(!move_waits(&rig, &id).await, "the move ended its wait");
}

/// Whether a wait for the move into the slot stands.
async fn move_waits(rig: &Rig, id: &SessionId) -> bool {
    let waits = rig.fleet.store().lock().await.slot_waits_of(id.as_str()).expect("read");
    waits.iter().any(|wait| wait.phase == "move" && wait.ended_at.is_none())
}

/// A slot granted from the line to an idle Job: the move that was made at once now waits as well.
#[tokio::test]
async fn a_slot_granted_to_an_idle_job_with_a_subagent_working_waits_for_it() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let (waiting, _) = queued_and_idle(&rig, "change a file").await;
    agent_started(&rig, &waiting).await;

    close(&rig, &holder).await;
    eventually(|| async { rig.fleet.hosts().of(waiting.as_str()).state().moving.is_some() }).await;
    rig.fleet.held_moves_made().await;
    assert!(resumes(&rig, &waiting).is_empty(), "the subagent would be cut off");

    agent_finished(&rig, &waiting);
    rig.fleet.held_moves_made().await;
    assert_eq!(resumes(&rig, &waiting).len(), 1);
    keeper.abort();
}
