//! A person restarts a hosted Job's agent from its tools band: `mend_session_tools`. **A stand-in
//! process, as `session_host` has**: the restart is Fleet ending one and starting the next, and what it
//! is sent is what is read here.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{CloseSession, MendSessionTools, SessionFact, SessionId, SessionOrigin, SessionReport};

use super::session_host::{eventually, rig_placing, Rig};

fn mend(id: &SessionId) -> MendSessionTools {
    MendSessionTools { session_id: id.clone() }
}

fn code(refusal: Refusal) -> (u16, String) {
    let status = refusal.status();
    match refusal {
        Refusal::IllegalMove(why) | Refusal::Unacceptable(why) => (status, why.code),
        other => panic!("not a refusal: {other:?}"),
    }
}

/// Fleet's copy of the mod is there already, so a restart has nothing to fill.
fn copy_present(rig: &Rig) {
    let copy = copy_of(rig);
    std::fs::create_dir_all(copy.parent().unwrap()).unwrap();
    std::fs::write(copy, "{}").unwrap();
}

/// Where Fleet's copy of the mod declares itself.
fn copy_of(rig: &Rig) -> std::path::PathBuf {
    crate::runtime::mod_dir(&rig.fleet.host().home).join(adapters::agent_mod::COPY_MARKETPLACE)
}

fn lines_of(rig: &Rig, index: usize) -> Vec<String> {
    rig.stand_in.process(index).sent.lock().unwrap().clone()
}

#[tokio::test]
async fn a_job_with_unfinished_lines_is_restarted_and_the_lines_go_to_the_new_process() {
    let rig = rig_placing(None);
    copy_present(&rig);
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    // No `init` yet: the line is written to the process and its turn has not begun.
    assert_eq!(lines_of(&rig, 0).len(), 1);

    let record = Arc::clone(&rig.fleet).mend_session_tools(mend(&id)).await.expect("mended");
    assert_eq!(record.id, id);
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst), "the old process was let go");
    assert_eq!(rig.stand_in.starts().len(), 2);
    let sent = lines_of(&rig, 1);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(sent[0].contains("change a file"), "the unfinished line was sent again: {}", sent[0]);

    // A person's second press restarts again: the once-only guard is the automatic restart's.
    Arc::clone(&rig.fleet).mend_session_tools(mend(&id)).await.expect("mended again");
    assert_eq!(rig.stand_in.starts().len(), 3);
    assert!(lines_of(&rig, 2)[0].contains("change a file"));
}

#[tokio::test]
async fn an_idle_job_is_told_by_fleet_so_the_new_process_has_a_turn_to_come_up_in() {
    let rig = rig_placing(None);
    copy_present(&rig);
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "done");
    eventually(|| async { matches!(rig.turn(&id).await, ipc::SessionTurn::Idle) }).await;

    Arc::clone(&rig.fleet).mend_session_tools(mend(&id)).await.expect("mended");
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst));
    assert_eq!(rig.stand_in.starts().len(), 2);
    let sent = lines_of(&rig, 1);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(sent[0].contains("Fleet restarted your process with the armada mod"), "{}", sent[0]);
    let rows = rig.rows(&id).await;
    assert!(
        rows.iter().any(|row| matches!(row, ipc::SessionRow::Message { text, notice: None, .. } if text.contains("restarted your process"))),
        "the thread draws Fleet's word: {rows:?}"
    );
}

#[tokio::test]
async fn the_mods_copy_is_filled_before_the_restart_and_a_copy_that_cannot_be_filled_restarts_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;

    let refused = Arc::clone(&rig.fleet).mend_session_tools(mend(&id)).await.expect_err("nothing to fill it from");
    let (status, why) = code(refused);
    assert_eq!((status, why.as_str()), (422, "fleet.session_mod_not_filled"));
    assert!(!rig.stand_in.process(0).ended.load(Ordering::SeqCst), "the old process was left running");
    assert_eq!(rig.stand_in.starts().len(), 1);

    // The script that fills the copy, in a tree Fleet serves.
    let script = std::path::Path::new(&rig.root).join("scripts/sync-mod");
    std::fs::create_dir_all(script.parent().unwrap()).unwrap();
    let copy = copy_of(&rig);
    let body = format!("#!/bin/sh\nmkdir -p '{}'\necho '{{}}' > '{}'\n", copy.parent().unwrap().display(), copy.display());
    std::fs::write(&script, body).unwrap();
    std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let marketplace = std::path::Path::new(&rig.root).join(adapters::agent_mod::SOURCE_MARKETPLACE);
    std::fs::create_dir_all(marketplace.parent().unwrap()).unwrap();
    std::fs::write(&marketplace, "{}").unwrap();

    Arc::clone(&rig.fleet).mend_session_tools(mend(&id)).await.expect("filled, then restarted");
    assert!(copy_of(&rig).is_file(), "the copy was filled");
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst));
    assert_eq!(rig.stand_in.starts().len(), 2);
}

#[tokio::test]
async fn a_terminal_job_a_closed_job_and_a_job_nobody_hosts_are_refused_in_words() {
    let rig = rig_placing(None);
    copy_present(&rig);
    const TERMINAL: &str = "bbbb-2222";
    rig.fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried(TERMINAL),
            fact: SessionFact::Started { cwd: rig.root.clone(), title: None, origin: SessionOrigin::Terminal, mod_version: None },
        })
        .await
        .unwrap();
    let refused = Arc::clone(&rig.fleet).mend_session_tools(mend(&SessionId::carried(TERMINAL))).await.expect_err("a terminal job");
    assert_eq!(code(refused), (422, "fleet.session_tools_unmendable".to_string()));

    let id = rig.start().await;
    rig.send(&id, "hello").await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("closed");
    let refused = Arc::clone(&rig.fleet).mend_session_tools(mend(&id)).await.expect_err("a closed job");
    assert_eq!(code(refused), (409, "fleet.session_closed".to_string()));

    let refused = Arc::clone(&rig.fleet).mend_session_tools(mend(&SessionId::carried("zzzz"))).await.expect_err("nobody");
    assert_eq!(code(refused), (422, "fleet.no_such_session".to_string()));
}
