//! What a hosted Job's agent came up without, read off its process's `init` line: the mark on the row,
//! and the one restart for the pieces Fleet can mend. **A stand-in process, as `session_host` has**; the
//! `init` it says is the reading a real one would make. `docs/concepts/session/a-job-that-came-up-without-its-tools.md`.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{DroneEvent, HarnessReading};
use api::HostedSessions;
use ipc::{CloseSession, SessionId, ToolMissing, ToolsMissing};

use super::session_host::{eventually, rig_placing, whole, Rig};

const VERSION: &str = "2.1.310";
const HARNESS: &str = "the harness";
const UPDATE: &str = "harness update";

fn init_reading(reading: HarnessReading) -> Vec<DroneEvent> {
    vec![DroneEvent::Started { session: String::new(), model: String::from("stand-in"), mcp_servers: 1, harness: reading }]
}

fn reading(mod_loaded: bool, door_connected: bool, task_tools: bool) -> HarnessReading {
    HarnessReading { name: HARNESS.into(), update: UPDATE.into(), version: VERSION.into(), mod_loaded, door_connected, task_tools }
}

async fn missing(rig: &Rig, id: &SessionId) -> Option<ToolsMissing> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session.tools_missing
}

fn marked(pieces: &[ToolMissing]) -> Option<ToolsMissing> {
    Some(ToolsMissing { harness: HARNESS.into(), update: UPDATE.into(), harness_version: VERSION.into(), missing: pieces.to_vec() })
}

/// Fleet's copy of the mod on disk, so a launch passes `--plugin-dir` and a mod that did not load is
/// something a restart can mend.
fn mod_copied(rig: &Rig) {
    let manifest = crate::runtime::mod_dir(&rig.fleet.host().home).join(adapters::MOD_INSTALLED_MANIFEST);
    std::fs::create_dir_all(manifest.parent().expect("a folder")).expect("its folder");
    std::fs::write(manifest, "{}").expect("the manifest");
}

/// Let anything the init set going settle, so a restart that is going to happen has.
async fn settled() {
    tokio::time::sleep(Duration::from_millis(150)).await;
}

#[tokio::test]
async fn a_job_that_came_up_with_everything_carries_no_mark() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;

    rig.stand_in.says(0, init_reading(whole()));
    settled().await;

    assert_eq!(missing(&rig, &id).await, None, "nothing missing is absent, not an empty list");
    assert_eq!(rig.stand_in.starts().len(), 1);
}

#[tokio::test]
async fn a_job_whose_init_lacks_the_door_is_restarted_once_and_its_unfinished_line_goes_to_the_replacement() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "fix the login").await;
    assert!(rig.stand_in.process(0).sent.lock().unwrap()[0].contains("fix the login"));

    rig.stand_in.says(0, init_reading(reading(true, false, true)));

    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst), "the one without the door is ended");
    eventually(|| async { !rig.stand_in.process(1).sent.lock().unwrap().is_empty() }).await;
    let sent = rig.stand_in.process(1).sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 1, "the line, once: {sent:?}");
    assert!(sent[0].contains("fix the login"), "{sent:?}");
    assert_eq!(missing(&rig, &id).await, None, "a process being mended is not marked");

    // The replacement comes up whole: the line is taken, the turn goes on, and nothing is marked.
    rig.stand_in.says(1, init_reading(whole()));
    settled().await;
    assert_eq!(missing(&rig, &id).await, None);
    assert_eq!(rig.stand_in.starts().len(), 2, "mended once");
}

#[tokio::test]
async fn a_replacement_that_lacks_the_door_too_keeps_the_mark_and_is_not_restarted_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    rig.stand_in.says(0, init_reading(reading(true, false, true)));
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;

    rig.stand_in.says(1, init_reading(reading(true, false, true)));

    eventually(|| async { missing(&rig, &id).await.is_some() }).await;
    assert_eq!(missing(&rig, &id).await, marked(&[ToolMissing::Door]));
    settled().await;
    assert_eq!(rig.stand_in.starts().len(), 2, "not a third process");
    assert!(!rig.stand_in.process(1).ended.load(Ordering::SeqCst));
}

#[tokio::test]
async fn a_mark_is_cleared_by_the_next_init_that_lacks_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    rig.stand_in.says(0, init_reading(reading(true, true, false)));
    eventually(|| async { missing(&rig, &id).await.is_some() }).await;

    rig.stand_in.says(0, init_reading(whole()));

    eventually(|| async { missing(&rig, &id).await.is_none() }).await;
}

#[tokio::test]
async fn missing_task_tools_only_mark_the_job_and_nothing_is_restarted() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;

    rig.stand_in.says(0, init_reading(reading(true, true, false)));

    eventually(|| async { missing(&rig, &id).await.is_some() }).await;
    assert_eq!(missing(&rig, &id).await, marked(&[ToolMissing::TaskTools]));
    settled().await;
    assert_eq!(rig.stand_in.starts().len(), 1);
    assert!(!rig.stand_in.process(0).ended.load(Ordering::SeqCst));
}

#[tokio::test]
async fn a_mod_that_did_not_load_is_not_restarted_when_the_launch_never_carried_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;

    rig.stand_in.says(0, init_reading(reading(false, true, true)));

    eventually(|| async { missing(&rig, &id).await.is_some() }).await;
    assert_eq!(missing(&rig, &id).await, marked(&[ToolMissing::Mod]));
    settled().await;
    assert_eq!(rig.stand_in.starts().len(), 1, "a restart would launch the same way");
}

#[tokio::test]
async fn a_mod_that_did_not_load_though_the_launch_carried_it_is_restarted_once() {
    let rig = rig_placing(None);
    mod_copied(&rig);
    let id = rig.start().await;
    rig.send(&id, "hello").await;

    rig.stand_in.says(0, init_reading(reading(false, true, true)));
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    rig.stand_in.says(1, init_reading(reading(false, true, true)));

    eventually(|| async { missing(&rig, &id).await.is_some() }).await;
    assert_eq!(missing(&rig, &id).await, marked(&[ToolMissing::Mod]));
    settled().await;
    assert_eq!(rig.stand_in.starts().len(), 2);
}

/// The wire's order is the mod, the door, the task tools, whatever the init listed.
#[tokio::test]
async fn what_is_missing_is_listed_the_mod_then_the_door_then_the_task_tools() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    rig.stand_in.says(0, init_reading(reading(false, false, false)));
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;

    rig.stand_in.says(1, init_reading(reading(false, false, false)));

    eventually(|| async { missing(&rig, &id).await.is_some() }).await;
    assert_eq!(missing(&rig, &id).await, marked(&[ToolMissing::Mod, ToolMissing::Door, ToolMissing::TaskTools]));
}

#[tokio::test]
async fn a_process_started_later_for_another_reason_is_mended_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "first").await;
    rig.stand_in.says(0, init_reading(reading(true, false, true)));
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    rig.stand_in.says(1, init_reading(whole()));
    rig.stand_in.finishes(1, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;

    // A quiet spell ends the process; the next message starts another, which lacks the door again.
    rig.fleet
        .tune_session(ipc::TuneSession { session_id: id.clone(), model: None, effort: None, mode: ipc::SessionMode::Auto })
        .await
        .expect("tuned");
    rig.send(&id, "second").await;
    eventually(|| async { rig.stand_in.starts().len() == 3 }).await;
    rig.stand_in.says(2, init_reading(reading(true, false, true)));

    eventually(|| async { rig.stand_in.starts().len() == 4 }).await;
    eventually(|| async { !rig.stand_in.process(3).sent.lock().unwrap().is_empty() }).await;
    assert!(rig.stand_in.process(3).sent.lock().unwrap()[0].contains("second"));
}

#[tokio::test]
async fn the_mark_is_not_on_a_job_that_has_ended() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    rig.stand_in.says(0, init_reading(reading(true, true, false)));
    eventually(|| async { missing(&rig, &id).await.is_some() }).await;

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("closed");

    assert_eq!(missing(&rig, &id).await, None);
}
