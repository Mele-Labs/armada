//! What a hosted session's agent runs beside its turn is on its ledger as `background` rows, read off
//! the session's own stream. `docs/concepts/session.md`, *The ledger*.

use adapter_traits::{CallDetail, DroneEvent, TaskStatus};
use api::Sessions;
use ipc::{Attachment, AttachmentState, SessionId};

use super::session_host::{eventually, rig_placing, Rig};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::session_host::Heard;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn called(tool: &str, call: &str, command: &str) -> DroneEvent {
    DroneEvent::Called { tool: tool.into(), call: call.into(), detail: CallDetail::of(command) }
}

fn started(task: &str, call: &str, description: &str, backgrounded: bool) -> DroneEvent {
    DroneEvent::TaskStarted { task: task.into(), call: call.into(), description: description.into(), backgrounded }
}

fn ended(task: &str, status: TaskStatus, output: Option<&str>, exit: Option<i32>) -> DroneEvent {
    DroneEvent::TaskEnded { task: task.into(), status, output: output.map(String::from), exit }
}

async fn background(rig: &Rig, id: &SessionId) -> Vec<Attachment> {
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    listed.into_iter().find(|one| one.id == *id).map(|one| one.attachments).unwrap_or_default().into_iter().filter(|one| one.kind == "background").collect()
}

fn said(one: &Attachment, key: &str) -> Option<String> {
    one.detail.get(key).cloned()
}

async fn running(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    id
}

#[tokio::test]
async fn a_backgrounded_command_stands_while_it_runs_and_is_spent_with_how_it_ended() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.says(
        0,
        vec![
            called("Bash", "toolu_1", "sh -c 'sleep 3; exit 3'"),
            started("bvz7c6w5u", "toolu_1", "bad job", true),
        ],
    );
    eventually(|| async { background(&rig, &id).await.len() == 1 }).await;
    let row = background(&rig, &id).await.remove(0);
    assert_eq!(row.target, "bvz7c6w5u");
    assert_eq!(row.state, AttachmentState::Standing);
    assert_eq!(said(&row, "form").as_deref(), Some("command"));
    assert_eq!(said(&row, "task").as_deref(), Some("bad job"));
    assert_eq!(said(&row, "command").as_deref(), Some("sh -c 'sleep 3; exit 3'"));
    assert_eq!(said(&row, "status"), None);

    // The status change settles it; the notice that follows fills in where its output went and the exit code.
    rig.stand_in.says(0, vec![ended("bvz7c6w5u", TaskStatus::Failed, None, None)]);
    eventually(|| async { background(&rig, &id).await[0].state == AttachmentState::Spent }).await;
    assert_eq!(said(&background(&rig, &id).await[0], "status").as_deref(), Some("failed"));

    rig.stand_in.says(0, vec![ended("bvz7c6w5u", TaskStatus::Failed, Some("/tmp/tasks/bvz7c6w5u.output"), Some(3))]);
    eventually(|| async { said(&background(&rig, &id).await[0], "exit").is_some() }).await;
    let row = background(&rig, &id).await.remove(0);
    assert_eq!(row.state, AttachmentState::Spent);
    assert_eq!(said(&row, "status").as_deref(), Some("failed"));
    assert_eq!(said(&row, "exit").as_deref(), Some("3"));
    assert_eq!(said(&row, "output").as_deref(), Some("/tmp/tasks/bvz7c6w5u.output"));
    assert_eq!(said(&row, "command").as_deref(), Some("sh -c 'sleep 3; exit 3'"), "what it started with stays");
}

/// The notice may come first; the row is settled by whichever line is heard first.
#[tokio::test]
async fn a_notice_heard_before_the_status_change_settles_it_just_the_same() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.says(0, vec![called("Bash", "toolu_1", "make"), started("t1", "toolu_1", "build", true)]);
    eventually(|| async { background(&rig, &id).await.len() == 1 }).await;
    rig.stand_in.says(0, vec![ended("t1", TaskStatus::Completed, Some("/tmp/t1.output"), Some(0))]);
    eventually(|| async { background(&rig, &id).await[0].state == AttachmentState::Spent }).await;
    let row = background(&rig, &id).await.remove(0);
    assert_eq!((said(&row, "status").as_deref(), said(&row, "exit").as_deref()), (Some("completed"), Some("0")));
}

#[tokio::test]
async fn a_watch_is_a_monitor_and_several_stand_at_once() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.says(
        0,
        vec![
            called("Bash", "toolu_1", "cargo build"),
            called("Monitor", "toolu_2", "tail -f log"),
            started("t1", "toolu_1", "build", true),
            started("t2", "toolu_2", "watch the log", true),
        ],
    );
    eventually(|| async { background(&rig, &id).await.len() == 2 }).await;
    let rows = background(&rig, &id).await;
    let form = |target: &str| rows.iter().find(|one| one.target == target).and_then(|one| said(one, "form"));
    assert_eq!(form("t1").as_deref(), Some("command"));
    assert_eq!(form("t2").as_deref(), Some("monitor"));
    assert!(rows.iter().all(|one| one.state == AttachmentState::Standing));
}

/// A subagent is a task too, and a command run in the foreground is a task that never stood beside the turn.
#[tokio::test]
async fn a_subagent_or_a_foreground_command_is_not_a_background_row_and_a_stray_end_settles_nothing() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.says(
        0,
        vec![
            called("Task", "toolu_1", "explore"),
            started("a1", "toolu_1", "explore", true),
            called("Bash", "toolu_2", "make"),
            started("t2", "toolu_2", "make", false),
            ended("a1", TaskStatus::Completed, None, None),
            ended("never-seen", TaskStatus::Killed, None, None),
        ],
    );
    // Something heard after them has been taken, so they have been.
    rig.stand_in.says(0, vec![called("Bash", "toolu_3", "ls"), started("t3", "toolu_3", "ls", true)]);
    eventually(|| async { background(&rig, &id).await.len() == 1 }).await;
    assert_eq!(background(&rig, &id).await[0].target, "t3");
}

/// A subagent's command is the session's: its call arrives under the subagent and its task on the stream.
#[tokio::test]
async fn a_subagents_backgrounded_command_is_the_sessions_row() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.hears(0, Heard::Subagent { events: vec![called("Bash", "toolu_7", "pnpm mock")], call: Some("toolu_1".into()) });
    rig.stand_in.says(0, vec![started("t7", "toolu_7", "serve the mock", true)]);
    eventually(|| async { background(&rig, &id).await.len() == 1 }).await;
    assert_eq!(said(&background(&rig, &id).await[0], "command").as_deref(), Some("pnpm mock"));
}

/// The row does not make the session working: its turn is what runs.
#[tokio::test]
async fn a_standing_row_leaves_the_turn_idle_when_it_ends() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.says(0, vec![called("Bash", "toolu_1", "make"), started("t1", "toolu_1", "build", true)]);
    eventually(|| async { background(&rig, &id).await.len() == 1 }).await;
    rig.stand_in.finishes(0, "started it");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    let record = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions.into_iter().find(|one| one.id == id).unwrap();
    assert_eq!(background(&rig, &id).await[0].state, AttachmentState::Standing);
    assert_ne!(record.standing, Some(ipc::SessionStanding::Working));
}

/// The process that ran them is gone with them.
#[tokio::test]
async fn a_process_that_is_replaced_gives_its_standing_tasks_back() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.says(0, vec![called("Bash", "toolu_1", "make"), started("t1", "toolu_1", "build", true)]);
    eventually(|| async { background(&rig, &id).await.len() == 1 }).await;
    rig.stand_in.finishes(0, "started it");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;

    Hosted::let_go(&mut rig.fleet.hosts().of(id.as_str()).state());
    rig.send(&id, "again").await;
    eventually(|| async { background(&rig, &id).await[0].state == AttachmentState::GivenBack }).await;
}
