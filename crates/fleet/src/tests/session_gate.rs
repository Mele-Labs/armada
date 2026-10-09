//! The write gate's reading of a slot the session's own agent leased, and of a
//! shell line that names the main checkout. Since 23.71.
//!
//! A slot lives under the main checkout's path, so a path in it was once
//! refused as if it were the checkout. What decides now is who holds the lease
//! record beside the slot.

use std::process::Command;
use std::sync::Arc;

use super::session_host::{denied, eventually, reason, rig_placing, Rig};
use crate::tests::peer::Placing;

/// A lease record naming `pid` as its holder, as `armada worktree lease` writes
/// it: the pid and its start time together.
fn lease(rig: &Rig, slot: u32, pid: u32, started: &str) -> String {
    let at = adapter_traits::slot_path(&rig.root, slot);
    let directory = std::path::Path::new(&at);
    let _ = std::fs::create_dir_all(directory);
    std::fs::write(
        format!("{at}.lease"),
        format!("branch sessions/next\nholder {pid} {started}\nsince 1\n"),
    )
    .expect("a lease record");
    at
}

fn started(pid: u32) -> String {
    let said = Command::new("ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()
        .expect("ps");
    String::from_utf8_lossy(&said.stdout).trim().to_string()
}

async fn in_its_slot(peers: &Arc<Placing>) -> (Rig, ipc::SessionId) {
    let rig = rig_placing(Some(Arc::clone(peers)));
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    rig.stand_in.init(0);
    let first = rig
        .gate(
            &id,
            "Edit",
            &format!(r#"{{"file_path":"{}/src/lib.rs"}}"#, rig.root),
        )
        .await;
    assert!(denied(&first));
    // The stand-in vcs writes no record; the real pool holds the slot for the
    // session by its id.
    let at = adapter_traits::slot_path(&rig.root, 1);
    std::fs::create_dir_all(&at).expect("a slot");
    std::fs::write(
        format!("{at}.lease"),
        format!("branch sessions/own\nholder job {}\nsince 1\n", id.as_str()),
    )
    .expect("a lease record");
    rig.stand_in.finishes(0, "stopping");
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    rig.stand_in.init(1);
    (rig, id)
}

#[tokio::test]
async fn a_write_in_a_slot_the_sessions_own_tree_leased_is_let_through() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let me = std::process::id();
    let theirs = lease(&rig, 4, me, &started(me));
    let file = format!(r#"{{"file_path":"{theirs}/src/lib.rs"}}"#);

    // Leased, but not by anything this session started.
    let held_by_another = rig.gate(&id, "Edit", &file).await;
    assert!(denied(&held_by_another));
    assert!(reason(&held_by_another).contains("not leased by this session"));

    // Its agent's subagent leased it: the holder is a descendant of the process.
    peers.started(1001, 2002);
    peers.started(2002, me);
    assert!(!denied(&rig.gate(&id, "Edit", &file).await));
    assert!(!denied(&rig.gate(&id, "Write", &file).await));

    // The holder having gone, the lease is nobody's.
    lease(&rig, 4, me, "Mon Jan  1 00:00:00 1990");
    assert!(denied(&rig.gate(&id, "Edit", &file).await));
}

#[tokio::test]
async fn the_main_checkout_is_still_held_and_so_is_a_slot_somebody_else_holds() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let main = rig
        .gate(
            &id,
            "Edit",
            &format!(r#"{{"file_path":"{}/src/lib.rs"}}"#, rig.root),
        )
        .await;
    assert!(denied(&main));
    assert!(reason(&main).contains("main checkout"));
    let sideways = rig
        .gate(
            &id,
            "Edit",
            &format!(
                r#"{{"file_path":"{}/.armada/slots/slot-1/../../../src/lib.rs"}}"#,
                rig.root
            ),
        )
        .await;
    assert!(denied(&sideways), "a `..` out of the slot is the checkout");
    let own = adapter_traits::slot_path(&rig.root, 1);
    assert!(!denied(
        &rig.gate(
            &id,
            "Edit",
            &format!(r#"{{"file_path":"{own}/src/lib.rs"}}"#)
        )
        .await
    ));
    let parent = std::os::unix::process::parent_id();
    let other = lease(&rig, 5, parent, &started(parent));
    assert!(denied(
        &rig.gate(&id, "Edit", &format!(r#"{{"file_path":"{other}/a"}}"#))
            .await
    ));
}

#[tokio::test]
async fn a_shell_line_that_names_the_main_checkout_is_held_like_a_write() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let main = rig.root.clone();
    let own = adapter_traits::slot_path(&main, 1);
    let line = |command: String| {
        let rig = &rig;
        let id = &id;
        async move {
            let encoded = ipc::encode(&Quoted { command: &command }).expect("a line");
            rig.gate(id, "Bash", &encoded).await
        }
    };
    for held in [
        format!("echo hi > {main}/src/lib.rs"),
        format!("echo hi >> {main}/notes.md"),
        format!("echo hi 2>&1 | tee {main}/notes.md"),
        format!("cp a.txt {main}/a.txt"),
        format!("mv {main}/a.txt {main}/b.txt"),
        format!("sed -i 's/a/b/' {main}/src/lib.rs"),
        format!("git -C {main} commit -am x"),
        format!("cargo build && printf x > {main}/out"),
    ] {
        assert!(denied(&line(held.clone()).await), "{held} should be held");
    }
    for fine in [
        format!("echo hi > {own}/src/lib.rs"),
        format!("cp a.txt {own}/a.txt"),
        format!("git -C {main} status"),
        format!("git -C {own} commit -am x"),
        String::from("cargo build 2>/dev/null"),
        String::from("sed -i 's/a/b/' src/lib.rs"),
        format!("cat {main}/src/lib.rs > /tmp/copy"),
    ] {
        assert!(!denied(&line(fine.clone()).await), "{fine} should run");
    }
}

#[derive(serde::Serialize)]
struct Quoted<'a> {
    command: &'a str,
}

async fn standing(rig: &Rig, id: &ipc::SessionId, kind: &str) -> Vec<String> {
    rig.fleet
        .store()
        .lock()
        .await
        .attachments_of(&store::Holder::session(id.as_str()))
        .expect("the ledger")
        .into_iter()
        .filter(|one| one.kind == kind && one.state == store::AttachmentState::Standing)
        .map(|one| one.target)
        .collect()
}

#[tokio::test]
async fn a_slot_the_agent_released_by_the_cli_is_given_back_in_the_ledger() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let own = adapter_traits::slot_path(&rig.root, 1);
    let file = format!(r#"{{"file_path":"{own}/src/lib.rs"}}"#);
    assert!(!denied(&rig.gate(&id, "Edit", &file).await));
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);

    // `armada worktree release` removes the record; the slot is then held by
    // another session's agent.
    std::fs::remove_file(format!("{own}.lease")).expect("released");
    let me = std::process::id();
    lease(&rig, 1, me, &started(me));
    assert!(denied(&rig.gate(&id, "Edit", &file).await));
    assert!(standing(&rig, &id, "slot").await.is_empty());
    assert!(standing(&rig, &id, "branch").await.is_empty());
    let hosting = rig.fleet.store().lock().await.hosting(id.as_str()).unwrap().unwrap();
    assert_eq!(hosting.lease_slot, None);
}

#[tokio::test]
async fn a_slot_the_agent_leased_by_the_cli_is_attached_and_writable() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let own = adapter_traits::slot_path(&rig.root, 1);
    std::fs::remove_file(format!("{own}.lease")).expect("released");

    // The agent (pid 1001, under its keeper) runs the command, whose caller is
    // the agent itself.
    let me = std::process::id();
    let theirs = lease(&rig, 5, me, &started(me));
    peers.started(1001, me);
    let file = format!(r#"{{"file_path":"{theirs}/src/lib.rs"}}"#);
    assert!(!denied(&rig.gate(&id, "Edit", &file).await));
    assert_eq!(standing(&rig, &id, "slot").await, vec!["5"]);
    assert_eq!(standing(&rig, &id, "branch").await, vec!["sessions/next"]);
    let hosting = rig.fleet.store().lock().await.hosting(id.as_str()).unwrap().unwrap();
    assert_eq!(hosting.lease_slot, Some(5));
}

#[test]
fn a_keeper_is_found_by_the_socket_it_serves() {
    use crate::session_host::following::keeper_in;
    let listing = "  41 /bin/zsh -l\n 977 armada session-keep /k/a.sock /k/a.spool 600 -- claude\n 978 armada session-keep /k/b.sock /k/b.spool 600 -- claude\n";
    assert_eq!(keeper_in(listing, "/k/b.sock"), Some(978));
    assert_eq!(keeper_in(listing, "/k/c.sock"), None);
}

#[tokio::test]
async fn a_slot_is_read_through_the_keeper_when_fleet_holds_no_pid_for_the_agent() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let me = std::process::id();
    let theirs = lease(&rig, 4, me, &started(me));
    let file = format!(r#"{{"file_path":"{theirs}/src/lib.rs"}}"#);
    // A keeper process whose argv names this session's socket.
    let socket = std::path::Path::new(&rig.fleet.host().keepers_dir).join(format!("{}.sock", id.as_str()));
    let mut keeper = Command::new("sh")
        .args(["-c", "sleep 30; sleep 1", "session-keep", &socket.to_string_lossy()])
        .spawn()
        .expect("a stand-in keeper");
    peers.started(keeper.id(), me);
    rig.fleet.hosts().of(id.as_str()).state().process = None;
    let allowed = !denied(&rig.gate(&id, "Edit", &file).await);
    let _ = keeper.kill();
    assert!(allowed);
}
