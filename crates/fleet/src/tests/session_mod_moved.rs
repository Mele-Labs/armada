//! A hosted Job's process that runs an older armada mod than Fleet's copy, after a restart of Fleet
//! refreshed the copy: `processes_moved_onto_mod`. **A stand-in process, as `session_host` has**, and the
//! sweep is called by the test, which would not wait for its timer.

use std::sync::atomic::Ordering;

use api::Sessions;
use ipc::{AttachmentReport, SessionFact, SessionId, SessionOrigin, SessionReport};

use super::session_host::{eventually, rig_placing, Rig};

/// Fleet's copy of the mod, declaring `version`.
fn copy_declares(rig: &Rig, version: &str) {
    let folder = crate::runtime::mod_dir(&rig.fleet.host().home);
    let manifest = folder.join(adapters::MOD_INSTALLED_MANIFEST);
    std::fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    std::fs::write(manifest, format!(r#"{{"name":"armada","version":"{version}"}}"#)).unwrap();
    let marketplace = folder.join(adapters::agent_mod::COPY_MARKETPLACE);
    std::fs::create_dir_all(marketplace.parent().unwrap()).unwrap();
    std::fs::write(marketplace, "{}").unwrap();
}

/// The mod in the Job's process says what it is, as it does when the process starts.
async fn mod_says(rig: &Rig, id: &SessionId, version: &str) {
    reports(rig, id, SessionFact::Started { cwd: rig.root.clone(), title: None, origin: SessionOrigin::Terminal, mod_version: Some(version.into()) }).await;
}

async fn reports(rig: &Rig, id: &SessionId, fact: SessionFact) {
    let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact };
    rig.fleet.report_session(report).await.expect("kept");
}

/// A Job whose turn ended, in process `index`, after the mod in it said `version`.
async fn idle_on(rig: &Rig, version: &str) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    mod_says(rig, &id, version).await;
    let index = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(index);
    rig.stand_in.finishes(index, "done");
    eventually(|| async { matches!(rig.turn(&id).await, ipc::SessionTurn::Idle) }).await;
    id
}

fn lines_of(rig: &Rig, index: usize) -> Vec<String> {
    rig.stand_in.process(index).sent.lock().unwrap().clone()
}

#[tokio::test]
async fn an_idle_job_on_an_older_mod_is_restarted_and_told_what_changed() {
    let rig = rig_placing(None);
    let id = idle_on(&rig, "0.3.50").await;

    // No copy declares a version yet: nothing to be older than.
    rig.fleet.processes_moved_onto_mod().await;
    assert_eq!(rig.stand_in.starts().len(), 1);

    copy_declares(&rig, "0.3.52");
    rig.fleet.processes_moved_onto_mod().await;

    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst), "the old process was let go");
    assert_eq!(rig.stand_in.starts().len(), 2);
    let sent = lines_of(&rig, 1);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(
        sent[0].contains("Fleet restarted your process onto armada mod 0.3.52 from 0.3.50. Nothing is asked of you."),
        "{}",
        sent[0]
    );
    let rows = rig.rows(&id).await;
    assert!(
        rows.iter().any(|row| matches!(row, ipc::SessionRow::Message { text, notice: None, .. } if text.contains("onto armada mod 0.3.52"))),
        "the thread draws Fleet's word: {rows:?}"
    );
}

#[tokio::test]
async fn a_job_with_a_turn_running_is_left_until_the_turn_ends() {
    let rig = rig_placing(None);
    copy_declares(&rig, "0.3.52");
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    mod_says(&rig, &id, "0.3.50").await;
    rig.stand_in.init(0);

    rig.fleet.processes_moved_onto_mod().await;
    assert!(!rig.stand_in.process(0).ended.load(Ordering::SeqCst), "a turn is never cut");
    assert_eq!(rig.stand_in.starts().len(), 1);

    rig.stand_in.finishes(0, "done");
    eventually(|| async { matches!(rig.turn(&id).await, ipc::SessionTurn::Idle) }).await;
    rig.fleet.processes_moved_onto_mod().await;
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst));
    assert_eq!(rig.stand_in.starts().len(), 2);
}

#[tokio::test]
async fn a_replacement_that_still_reports_the_older_mod_is_restarted_once_for_each_copy() {
    let rig = rig_placing(None);
    copy_declares(&rig, "0.3.52");
    let id = idle_on(&rig, "0.3.50").await;
    rig.fleet.processes_moved_onto_mod().await;
    assert_eq!(rig.stand_in.starts().len(), 2);

    // The replacement says the same old version and finishes its turn on Fleet's word.
    mod_says(&rig, &id, "0.3.50").await;
    rig.stand_in.init(1);
    rig.stand_in.finishes(1, "noted");
    eventually(|| async { matches!(rig.turn(&id).await, ipc::SessionTurn::Idle) }).await;
    rig.fleet.processes_moved_onto_mod().await;
    assert_eq!(rig.stand_in.starts().len(), 2, "the same copy does not loop");

    copy_declares(&rig, "0.3.53");
    rig.fleet.processes_moved_onto_mod().await;
    assert_eq!(rig.stand_in.starts().len(), 3, "a newer copy is a new reason");
    assert!(lines_of(&rig, 2)[0].contains("onto armada mod 0.3.53 from 0.3.50"));
}

#[tokio::test]
async fn a_job_on_the_current_mod_or_that_reported_none_is_left_as_it_is() {
    let rig = rig_placing(None);
    copy_declares(&rig, "0.3.52");
    let current = idle_on(&rig, "0.3.52").await;
    let newer = idle_on(&rig, "0.3.60").await;
    let silent = rig.start().await;
    rig.send(&silent, "hello").await;
    rig.stand_in.init(2);
    rig.stand_in.finishes(2, "done");
    eventually(|| async { matches!(rig.turn(&silent).await, ipc::SessionTurn::Idle) }).await;

    rig.fleet.processes_moved_onto_mod().await;

    assert_eq!(rig.stand_in.starts().len(), 3, "{current:?} {newer:?} {silent:?}");
    assert!((0..3).all(|index| !rig.stand_in.process(index).ended.load(Ordering::SeqCst)));
}

#[tokio::test]
async fn a_job_with_a_subagent_still_working_is_left_and_a_terminal_job_is_not_restarted() {
    let rig = rig_placing(None);
    copy_declares(&rig, "0.3.52");
    let id = idle_on(&rig, "0.3.50").await;
    reports(
        &rig,
        &id,
        SessionFact::Attached {
            attachment: AttachmentReport {
                kind: "subagent".into(),
                target: "agent-1".into(),
                detail: [("type".to_string(), "mechanic".to_string()), ("description".to_string(), "rename".to_string())].into_iter().collect(),
            },
        },
    )
    .await;
    // A person's own terminal reports an old mod too, and Fleet hosts no process of it.
    let terminal = SessionId::carried("bbbb-2222");
    mod_says(&rig, &terminal, "0.3.40").await;

    rig.fleet.processes_moved_onto_mod().await;

    assert!(!rig.stand_in.process(0).ended.load(Ordering::SeqCst), "the subagent would be cut off");
    assert_eq!(rig.stand_in.starts().len(), 1);
}
