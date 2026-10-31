//! A Job's System layer: which processes are the Job's, what ending one is allowed to touch, and the
//! readings that are absent rather than wrong. The processes are real `sleep`s; what is faked is
//! who started them, through the same `PeerOf` seam the kernel answers.

use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::Arc;

use api::{HostedSessions, Refusal};
use ipc::{CloseSession, SessionId, StopProcess};

use crate::session_host::processes::{memory_of, volume_of, Load};
use crate::tests::agent_starts::{rig, Rig};
use crate::tests::peer::Placing;

/// The first process the stand-in starts is the first Job's agent.
const AGENT: u32 = 1000;

fn a_sleep() -> Child {
    Command::new("sleep").arg("60").spawn().expect("a sleep to find")
}

/// A Job whose agent started one `sleep`, and another `sleep` it did not.
async fn a_job_with_two_sleeps() -> (Rig, SessionId, Child, Child) {
    let placing = Placing::nothing();
    let rig = rig(Some(Arc::clone(&placing)));
    let id = rig.an_agent().await;
    let (ours, theirs) = (a_sleep(), a_sleep());
    placing.started(AGENT, ours.id());
    (rig, id, ours, theirs)
}

fn stop(id: &SessionId, pid: Option<u32>, port: Option<u16>) -> StopProcess {
    StopProcess { session_id: Some(id.clone()), pid, port }
}

fn refused(result: Result<ipc::ProcessStopped, Refusal>) -> (String, String) {
    match result.expect_err("refused") {
        Refusal::Unacceptable(error) | Refusal::IllegalMove(error) | Refusal::NotFound(error) => (error.code, error.message),
        other => panic!("not a refusal a caller can read: {other:?}"),
    }
}

#[tokio::test]
async fn only_the_processes_the_job_started_are_listed() {
    let (rig, id, mut ours, mut theirs) = a_job_with_two_sleeps().await;
    let fleet = Arc::clone(&rig.fleet);
    let our_pid = ours.id();
    let listed = tokio::task::spawn_blocking(move || fleet.processes_of(id.as_str(), &[])).await.unwrap();
    let _ = (ours.kill(), theirs.kill(), ours.wait(), theirs.wait());

    let pids: Vec<u32> = listed.iter().map(|one| one.pid).collect();
    assert_eq!(pids, [our_pid], "the sleep nobody planted is not the Job's: {listed:?}");
    assert!(listed[0].command.contains("sleep"), "{listed:?}");
}

#[tokio::test]
async fn a_pid_the_job_did_not_start_is_refused_and_left_running() {
    let (rig, id, mut ours, mut theirs) = a_job_with_two_sleeps().await;
    let (code, message) = refused(Arc::clone(&rig.fleet).process_stopped(None, stop(&id, Some(theirs.id()), None)).await);
    assert_eq!(code, "fleet.process_not_the_jobs");
    assert!(message.contains(&format!("pid {}", theirs.id())), "{message}");

    let (code, message) = refused(Arc::clone(&rig.fleet).process_stopped(None, stop(&id, None, Some(1))).await);
    assert_eq!(code, "fleet.process_not_the_jobs", "a port nothing of the Job's listens on");
    assert!(message.contains("port 1"), "{message}");

    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(theirs.try_wait().unwrap().is_none(), "nothing was signalled");
    let _ = (ours.kill(), theirs.kill(), ours.wait(), theirs.wait());
}

#[tokio::test]
async fn neither_or_both_of_pid_and_port_is_refused_as_unnamed() {
    let (rig, id, mut ours, mut theirs) = a_job_with_two_sleeps().await;
    for (pid, port) in [(None, None), (Some(ours.id()), Some(5501))] {
        let (code, _) = refused(Arc::clone(&rig.fleet).process_stopped(None, stop(&id, pid, port)).await);
        assert_eq!(code, "fleet.process_unnamed", "{pid:?} {port:?}");
    }
    assert!(ours.try_wait().unwrap().is_none(), "naming both ended nothing");
    let _ = (ours.kill(), theirs.kill(), ours.wait(), theirs.wait());
}

#[tokio::test]
async fn a_closed_job_takes_no_stop() {
    let (rig, id, mut ours, mut theirs) = a_job_with_two_sleeps().await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("closed");
    let (code, _) = refused(Arc::clone(&rig.fleet).process_stopped(None, stop(&id, Some(ours.id()), None)).await);
    assert_eq!(code, "fleet.session_closed");
    let _ = (ours.kill(), theirs.kill(), ours.wait(), theirs.wait());
}

#[tokio::test]
async fn a_process_the_job_started_is_ended_and_named_in_the_answer() {
    let (rig, id, ours, mut theirs) = a_job_with_two_sleeps().await;
    let pid = ours.id();
    let stopped = Arc::clone(&rig.fleet).process_stopped(None, stop(&id, Some(pid), None)).await.expect("ended");
    assert_eq!(stopped.pid, pid);
    assert!(stopped.command.contains("sleep"), "{}", stopped.command);

    let mut ours = ours;
    let waited = tokio::task::spawn_blocking(move || ours.wait().expect("reaped")).await.unwrap();
    assert!(!waited.success(), "TERM ended it: {waited:?}");
    assert!(theirs.try_wait().unwrap().is_none(), "the other sleep still runs");
    let _ = (theirs.kill(), theirs.wait());
}

/// The report of 30 Oct: a vite server the Job started before its Claude Code process was
/// replaced, now launchd's child and running `node`, was refused as not the Job's.
#[tokio::test]
async fn a_process_left_by_a_replaced_agent_is_still_the_jobs_to_stop() {
    let placing = Placing::nothing();
    let rig = rig(Some(Arc::clone(&placing)));
    let id = rig.an_agent().await;
    let (orphan, mut theirs) = (a_sleep(), a_sleep());
    placing.carrying(orphan.id(), "ARMADA_SESSION", id.as_str());
    placing.carrying(theirs.id(), "ARMADA_SESSION", "another-job");

    let pid = orphan.id();
    let stopped = Arc::clone(&rig.fleet).process_stopped(None, stop(&id, Some(pid), None)).await.expect("ended");
    assert_eq!(stopped.pid, pid);
    let (code, _) = refused(Arc::clone(&rig.fleet).process_stopped(None, stop(&id, Some(theirs.id()), None)).await);
    assert_eq!(code, "fleet.process_not_the_jobs", "another Job's id is not this Job's");

    let mut orphan = orphan;
    let waited = tokio::task::spawn_blocking(move || orphan.wait().expect("reaped")).await.unwrap();
    assert!(!waited.success(), "TERM ended it: {waited:?}");
    assert!(theirs.try_wait().unwrap().is_none(), "the other Job's process still runs");
    let _ = (theirs.kill(), theirs.wait());
}

/// The keeper serving the Job's socket carries the Job's id as every process under it does, and
/// is never the Job's to list or end.
#[tokio::test]
async fn the_jobs_keeper_is_never_one_of_its_processes() {
    let placing = Placing::nothing();
    let rig = rig(Some(Arc::clone(&placing)));
    let id = rig.an_agent().await;
    let socket = std::path::Path::new(&rig.fleet.host().keepers_dir).join(format!("{}.sock", id.as_str()));
    let mut keeper = Command::new("sh")
        // A second command, so `sh` stays and does not exec into `sleep`, which drops the keeper's words.
        .args(["-c", "sleep 60; exit 0", "session-keep", &socket.to_string_lossy()])
        .process_group(0)
        .spawn()
        .expect("a stand-in keeper");
    placing.carrying(keeper.id(), "ARMADA_SESSION", id.as_str());

    let fleet = Arc::clone(&rig.fleet);
    let session = id.clone();
    let listed = tokio::task::spawn_blocking(move || fleet.processes_of(session.as_str(), &[])).await.unwrap();
    let (code, _) = refused(Arc::clone(&rig.fleet).process_stopped(None, stop(&id, Some(keeper.id()), None)).await);
    // The group, so the shell's `sleep` goes with it.
    let _ = Command::new("kill").args(["-KILL", &format!("-{}", keeper.id())]).status();
    let _ = keeper.wait();

    assert!(listed.is_empty(), "the keeper is not listed: {listed:?}");
    assert_eq!(code, "fleet.process_not_the_jobs");
}

#[test]
fn the_machines_memory_and_volume_are_read_from_what_sysctl_and_df_print() {
    assert_eq!(memory_of("17179869184\n"), Some(17_179_869_184));
    let df = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk3s5 489620264 1000 2000 50% /System/Volumes/Data\n";
    assert_eq!(volume_of(df), Some(489_620_264 * 1024));
}

#[test]
fn a_reading_that_is_not_the_shape_asked_for_is_none_never_zero() {
    for text in ["", "unknown oid\n", "12 GB\n"] {
        assert_eq!(memory_of(text), None, "{text:?}");
    }
    for text in ["", "Filesystem 1024-blocks\n", "Filesystem 1024-blocks\n/dev/x lots\n", "header only"] {
        assert_eq!(volume_of(text), None, "{text:?}");
    }
}

#[tokio::test]
async fn a_load_that_could_not_be_read_is_absent_not_zero() {
    let rig = rig(None);
    let fleet = Arc::clone(&rig.fleet);
    let load = tokio::task::spawn_blocking(move || fleet.load_of(&[], "/no/such/slot/anywhere")).await.unwrap();
    assert_eq!(load, Load { cpu_percent: None, memory_bytes: None, disk_bytes: None });
}

/// A Job with no slot, as every Job is once `finish_up` gave its slot back, ended every process
/// running a file from the repository's root on its close: Bridge from `.armada/preview`, and
/// every other Job's build in its slot (29 Oct). The root is no folder of its own.
#[tokio::test]
async fn a_job_without_a_slot_holds_no_folder_of_the_repository_root_to_end_from() {
    let rig = rig(Some(Placing::nothing()));
    let id = rig.an_agent().await;
    let (_, hosting) = rig.fleet.session_and_hosting(id.as_str()).await.expect("hosted");
    assert_eq!(hosting.lease_slot, None, "the case is a Job without a slot");
    let served = rig.fleet.served_named(Some(&rig.manifest)).expect("served");

    let folders = rig.fleet.process_directories(id.as_str(), &served, &hosting);
    let root = std::path::Path::new(served.root());
    assert!(!folders.iter().any(|one| std::path::Path::new(one) == root), "{folders:?}");
}
