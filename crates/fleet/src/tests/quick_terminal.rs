//! A quick terminal (`start_terminal_job`), through a real Fleet and a real shell: it opens onto
//! `t1`, its agent starts nothing until the person writes, and each message of his carries what the
//! terminals show. `docs/concepts/session/a-quick-terminal.md`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{SessionId, SessionRecord, StartTerminalJob, StopSessionRun, TerminalJobStarted, WriteSessionRun};

use super::session_host::{denied, eventually, rig_placing, Rig};
use super::session_terminal::shortly;
use crate::tests::tmp::TempDir;

fn quick(manifest_id: Option<ipc::ManifestId>, dir: Option<String>) -> StartTerminalJob {
    StartTerminalJob { manifest_id, dir, title: None, model: None, effort: None, mode: None, project: None, cols: None, rows: None }
}

async fn started(rig: &Rig, start: StartTerminalJob) -> TerminalJobStarted {
    Arc::clone(&rig.fleet).start_terminal_job(start).await.expect("started")
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

/// Until terminal `run_id` has printed `needle`, from what it holds.
async fn printed(rig: &Rig, run_id: &str, needle: &str) -> bool {
    shortly(|| async { rig.fleet.observe_session_run(run_id.into()).await.is_ok_and(|seen| seen.history.contains(needle)) }).await
}

async fn types(rig: &Rig, run_id: &str, text: &str) {
    rig.fleet.write_session_run(WriteSessionRun { run_id: run_id.into(), data: text.into() }).await.expect("in flight");
}

async fn ends(rig: &Rig, run_id: &str) {
    rig.fleet.stop_session_run(StopSessionRun { run_id: run_id.into() }).await.expect("in flight");
    eventually(|| async { rig.fleet.observe_session_run(run_id.into()).await.is_ok_and(|seen| seen.live.is_err()) }).await;
}

#[tokio::test]
async fn a_quick_terminal_opens_t1_and_starts_no_agent_until_the_person_writes() {
    let rig = rig_placing(None);
    let started = started(&rig, quick(Some(rig.manifest.clone()), None)).await;
    assert_eq!(started.terminal, "t1");

    let job = record(&rig, &started.session_id).await;
    assert!(job.quick_terminal, "listed as a quick terminal");
    assert_eq!(job.workflow, None, "no workflow");
    assert_eq!(job.manifest_id.as_ref(), Some(&rig.manifest));
    let hosted = job.hosted.expect("hosted");
    assert!(!hosted.running && hosted.start.is_none(), "not held and nothing running");
    assert!(rig.stand_in.starts().is_empty(), "no agent process while it waits");
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed");
    assert!(listed.sessions.iter().any(|one| one.id == started.session_id && one.quick_terminal));

    types(&rig, &started.run_id, "echo quick-$((6*7))\r").await;
    assert!(printed(&rig, &started.run_id, "quick-42").await, "t1 is a live shell");
    assert!(rig.stand_in.starts().is_empty(), "the person's typing starts no agent");
    ends(&rig, &started.run_id).await;
}

#[tokio::test]
async fn each_message_of_the_persons_carries_what_the_terminals_show_and_a_plain_jobs_does_not() {
    let rig = rig_placing(None);
    let started = started(&rig, quick(Some(rig.manifest.clone()), None)).await;
    types(&rig, &started.run_id, "echo failed-$((6*7))\r").await;
    assert!(printed(&rig, &started.run_id, "failed-42").await);

    rig.send(&started.session_id, "why did that fail?").await;
    let process = || rig.stand_in.process_of(started.session_id.as_str());
    eventually(|| async { process().is_some_and(|one| !one.sent.lock().unwrap().is_empty()) }).await;
    let sent = process().expect("started by the message").sent.lock().unwrap().join("\n");
    assert!(sent.contains("Terminal t1:") && sent.contains("failed-42"), "{sent}");
    assert!(sent.contains("why did that fail?"), "{sent}");
    ends(&rig, &started.run_id).await;

    let plain = rig.start().await;
    let shell = Arc::clone(&rig.fleet)
        .open_terminal(ipc::OpenTerminal { session_id: plain.clone(), terminal: None, new: false, cols: None, rows: None })
        .await
        .expect("opened")
        .run_id;
    types(&rig, &shell, "echo plain-$((6*7))\r").await;
    assert!(printed(&rig, &shell, "plain-42").await);
    rig.send(&plain, "hello").await;
    let process = || rig.stand_in.process_of(plain.as_str());
    eventually(|| async { process().is_some_and(|one| !one.sent.lock().unwrap().is_empty()) }).await;
    let sent = process().expect("started").sent.lock().unwrap().join("\n");
    assert!(!sent.contains("plain-42"), "a plain Job reads its terminal when it asks: {sent}");
    ends(&rig, &shell).await;
}

#[tokio::test]
async fn a_folder_in_a_served_repository_is_its_job_with_the_shell_in_that_folder() {
    let rig = rig_placing(None);
    let folder = std::path::Path::new(&rig.root).join("tools").join("quick");
    std::fs::create_dir_all(&folder).expect("a folder");
    let started = started(&rig, quick(None, Some(folder.to_string_lossy().into_owned()))).await;

    let job = record(&rig, &started.session_id).await;
    assert!(job.quick_terminal);
    assert_eq!(job.manifest_id.as_ref(), Some(&rig.manifest), "the repository's own Job");

    types(&rig, &started.run_id, "pwd\r").await;
    let here = std::fs::canonicalize(&folder).unwrap().to_string_lossy().into_owned();
    assert!(printed(&rig, &started.run_id, &here).await, "pwd is {here}");
    ends(&rig, &started.run_id).await;
}

#[tokio::test]
async fn a_loose_folder_runs_in_place_with_no_manifest_and_is_never_made_a_repository() {
    let rig = rig_placing(None);
    let loose = TempDir::new();
    let served_before = rig.fleet.repository_list().repositories.len();
    let started = started(&rig, quick(None, Some(loose.path().to_string_lossy().into_owned()))).await;

    let job = record(&rig, &started.session_id).await;
    let here = std::fs::canonicalize(loose.path()).unwrap().to_string_lossy().into_owned();
    assert!(job.quick_terminal);
    assert_eq!(job.manifest_id, None, "no Manifest");
    assert_eq!(job.cwd, here, "runs where it was started");
    assert!(!loose.path().join(".git").exists(), "no repository is started in it");
    assert_eq!(rig.fleet.repository_list().repositories.len(), served_before, "nothing is served");

    let file = format!("{here}/notes.txt");
    let answer = rig.gate_in(&here, &started.session_id, "Write", &format!(r#"{{"file_path":"{file}","content":"x"}}"#)).await;
    assert!(!denied(&answer), "a loose folder's Job writes in place: {answer:?}");
    assert!(record(&rig, &started.session_id).await.attachments.iter().all(|one| one.kind != "slot"), "never a slot");
    ends(&rig, &started.run_id).await;
}

#[tokio::test]
async fn both_or_neither_place_a_relative_folder_or_no_cells_are_refused_and_nothing_is_made() {
    let rig = rig_placing(None);
    let loose = TempDir::new();
    let dir = loose.path().to_string_lossy().into_owned();
    let before = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions.len();
    for start in [
        quick(Some(rig.manifest.clone()), Some(dir.clone())),
        quick(None, None),
        quick(None, Some(String::from("relative/folder"))),
        quick(None, Some(format!("{dir}/not-there"))),
        StartTerminalJob { cols: Some(0), ..quick(Some(rig.manifest.clone()), None) },
    ] {
        let answer = Arc::clone(&rig.fleet).start_terminal_job(start.clone()).await;
        assert!(matches!(answer, Err(api::Refusal::Unacceptable(_))), "{start:?}: {answer:?}");
    }
    assert_eq!(rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions.len(), before);
}
