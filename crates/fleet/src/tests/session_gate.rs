//! The write gate's reading of a slot the session's own agent leased, and of a
//! shell line that names the main checkout. Since 23.71.
//!
//! A slot lives under the main checkout's path, so a path in it was once
//! refused as if it were the checkout. What decides now is who holds the lease
//! record beside the slot.

use std::process::Command;
use std::sync::Arc;

use api::HostedSessions;

use super::session_host::{denied, eventually, reason, rig_placing, Rig};
use crate::tests::peer::Placing;

/// A lease record naming `pid` as its holder, as `armada worktree lease` writes
/// it: the pid and its start time together.
pub(super) fn lease(rig: &Rig, slot: u32, pid: u32, started: &str) -> String {
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

pub(super) fn started(pid: u32) -> String {
    let said = Command::new("ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()
        .expect("ps");
    String::from_utf8_lossy(&said.stdout).trim().to_string()
}

pub(super) async fn in_its_slot(peers: &Arc<Placing>) -> (Rig, ipc::SessionId) {
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
    assert!(reason(&held_by_another).contains("not leased by this job"));

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

/// A `cd` moves every relative write after it, so a line that works in a slot another session
/// holds, or in the main checkout, is held though no path in it is written to by name.
#[tokio::test]
async fn a_cd_into_a_slot_somebody_else_holds_holds_what_runs_after_it() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let main = rig.root.clone();
    let own = adapter_traits::slot_path(&main, 1);
    let parent = std::os::unix::process::parent_id();
    let theirs = lease(&rig, 5, parent, &started(parent));
    let line = |command: String| {
        let rig = &rig;
        let id = &id;
        async move {
            let encoded = ipc::encode(&Quoted { command: &command }).expect("a line");
            rig.gate(id, "Bash", &encoded).await
        }
    };
    for held in [
        format!("cd {theirs} && git add -A"),
        format!("cd {theirs} && cargo build"),
        format!("(cd {theirs}; touch f)"),
        String::from("cd .armada/slots/slot-5 && touch f"),
        format!("cd {theirs}; cd {theirs}/src && git commit -m x"),
        format!("cd {main} && git checkout main"),
    ] {
        let answer = line(held.clone()).await;
        assert!(denied(&answer), "{held} should be held");
    }
    assert!(reason(&line(format!("cd {theirs} && touch f")).await).contains("not leased by this job"));
    for fine in [
        format!("cd {theirs} && git status"),
        format!("cd {theirs} && ls src | wc -l"),
        format!("cd {own} && cargo build"),
        format!("cd {main} && armada worktree lease x"),
        format!("cd {theirs}; cd {own} && touch f"),
        String::from("cd /tmp && touch f"),
    ] {
        assert!(!denied(&line(fine.clone()).await), "{fine} should run");
    }
}

/// A relative `cd` after another is read from where the first left the line, not from where the
/// call began: `cd src && cd ../docs` ends in the slot's `docs`, never beside the slot.
#[tokio::test]
async fn a_chain_of_cds_is_read_from_where_the_last_one_left_it() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let own = adapter_traits::slot_path(&rig.root, 1);
    let parent = std::os::unix::process::parent_id();
    lease(&rig, 5, parent, &started(parent));
    let line = |command: &str| {
        let rig = &rig;
        let id = &id;
        let own = &own;
        let encoded = ipc::encode(&Quoted { command }).expect("a line");
        async move { rig.gate_in(own, id, "Bash", &encoded).await }
    };
    for fine in [
        "cd src && cd ../docs && touch f",
        "cd src && cp a ../b",
        "cd src && cd .. && cargo build",
    ] {
        assert!(!denied(&line(fine).await), "{fine} should run");
    }
    let held = line("cd src && cd ../../slot-5 && touch f").await;
    assert!(denied(&held));
    assert!(reason(&held).contains("slot 5"), "{}", reason(&held));
}

#[derive(serde::Serialize)]
pub(super) struct Quoted<'a> {
    pub(super) command: &'a str,
}

pub(super) async fn standing(rig: &Rig, id: &ipc::SessionId, kind: &str) -> Vec<String> {
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

/// #2239. A release by the agent's own CLI told nobody: the rows stood until the session's next
/// tool call, and a session with none to make kept them. The sweep reads the pool for it.
#[tokio::test]
async fn a_slot_the_agent_released_is_given_back_on_the_sweep_without_another_tool_call() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let own = adapter_traits::slot_path(&rig.root, 1);
    let file = format!(r#"{{"file_path":"{own}/src/lib.rs"}}"#);
    assert!(!denied(&rig.gate(&id, "Edit", &file).await));
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);

    std::fs::remove_file(format!("{own}.lease")).expect("released");
    let served = rig.fleet.repositories().first().expect("served");
    rig.fleet.slot_rows_followed(&served).await;

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

/// What the pool holds for a session, written as `armada worktree lease` writes it from a hosted
/// session's process: under the session's id, and not under any process.
pub(super) fn leased_for(rig: &Rig, slot: u32, id: &ipc::SessionId) -> String {
    let at = adapter_traits::slot_path(&rig.root, slot);
    let _ = std::fs::create_dir_all(&at);
    std::fs::write(
        format!("{at}.lease"),
        format!("branch sessions/next\nholder job {}\nsince 1\n", id.as_str()),
    )
    .expect("a lease record");
    at
}

#[tokio::test]
async fn a_slot_leased_for_the_session_stays_its_own_when_its_process_is_restarted() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let own = adapter_traits::slot_path(&rig.root, 1);
    std::fs::remove_file(format!("{own}.lease")).expect("released");
    let theirs = leased_for(&rig, 5, &id);
    let file = format!(r#"{{"file_path":"{theirs}/src/lib.rs"}}"#);

    // No process of the session's is in the lease, and none is in the tree it asks about.
    assert!(!denied(&rig.gate(&id, "Edit", &file).await));
    assert_eq!(standing(&rig, &id, "slot").await, vec!["5"]);

    // Fleet ends the process and resumes the session under another: the lease read the same.
    rig.fleet.hosts().of(id.as_str()).state().process = None;
    peers.started(2002, 1);
    assert!(!denied(&rig.gate(&id, "Edit", &file).await));
    assert_eq!(standing(&rig, &id, "slot").await, vec!["5"]);
    let hosting = rig.fleet.store().lock().await.hosting(id.as_str()).unwrap().unwrap();
    assert_eq!(hosting.lease_slot, Some(5));
}

#[tokio::test]
async fn closing_a_session_parks_every_slot_the_pool_holds_for_it() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    // A subagent's slot, which `armada worktree lease` took under the session's id.
    rig.fleet.vcs().hold_slot(&rig.root, 2, id.as_str());

    Arc::clone(&rig.fleet)
        .close_session(ipc::CloseSession { session_id: id.clone() })
        .await
        .expect("closed");

    super::session_host::eventually(|| async { rig.fleet.vcs().parked_slots().len() == 2 }).await;
    let mut parked = rig.fleet.vcs().parked_slots();
    parked.sort();
    assert_eq!(
        parked,
        vec![(1, id.as_str().to_string()), (2, id.as_str().to_string())]
    );
}

#[tokio::test]
async fn releasing_a_slot_the_session_does_not_hold_is_held_and_its_own_is_let_through() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let own = adapter_traits::slot_path(&rig.root, 1);
    let me = std::process::id();
    let theirs = lease(&rig, 5, me, &started(me));
    let line = |command: String| {
        let rig = &rig;
        let id = &id;
        async move {
            let encoded = ipc::encode(&Quoted { command: &command }).expect("a line");
            rig.gate(id, "Bash", &encoded).await
        }
    };

    let refused = line(format!("armada worktree release {theirs}")).await;
    assert!(denied(&refused));
    assert!(reason(&refused).contains("not leased by this job"));
    assert!(denied(&line(format!("armada worktree release {theirs}/")).await));
    assert!(denied(&line(format!("cd {theirs} && armada worktree release")).await));
    assert!(denied(&line(format!("armada worktree release {own}/../slot-5")).await));
    // The one it is refused is still held: nothing was let through to give it back.
    assert!(std::fs::read_to_string(format!("{theirs}.lease")).unwrap().contains("holder"));

    assert!(!denied(&line(format!("armada worktree release {own}")).await));
    // Run from the main checkout, which is where these calls start, a bare one gives that back.
    assert!(denied(&line(String::from("armada worktree release")).await));
    assert!(!denied(&line(format!("cd {own} && armada worktree release")).await));
    assert!(!denied(&line(String::from("armada worktree --status")).await));
}

#[tokio::test]
async fn a_write_in_the_slot_the_session_gave_back_is_not_its_own_though_its_directory_is_there() {
    let peers = Placing::nothing();
    let (rig, id) = in_its_slot(&peers).await;
    let own = adapter_traits::slot_path(&rig.root, 1);
    let file = format!(r#"{{"file_path":"{own}/src/lib.rs"}}"#);
    assert!(!denied(&rig.gate(&id, "Edit", &file).await));

    // It releases its own slot, and the process still stands in it. Another session takes it.
    std::fs::remove_file(format!("{own}.lease")).expect("released");
    std::fs::write(
        format!("{own}.lease"),
        "branch settings/links\nholder job 0b0b0b0b-0000-0000-0000-000000000000\nsince 1\n",
    )
    .expect("another session's lease");
    rig.fleet.vcs().hold_slot(&rig.root, 1, "0b0b0b0b-0000-0000-0000-000000000000");
    let theirs = rig.gate(&id, "Edit", &file).await;
    assert!(denied(&theirs));
    assert!(reason(&theirs).contains("not leased by this job"));
    let hosting = rig.fleet.store().lock().await.hosting(id.as_str()).unwrap().unwrap();
    assert_eq!(hosting.lease_slot, None, "the ledger lets go of it");

    // Its next write goes to lease it a slot, rather than being let into the old one.
    let relative = rig.gate(&id, "Edit", r#"{"file_path":"src/lib.rs"}"#).await;
    assert!(denied(&relative));
    let hosting = rig.fleet.store().lock().await.hosting(id.as_str()).unwrap().unwrap();
    assert_ne!(hosting.lease_slot, Some(1));
}
