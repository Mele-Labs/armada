//! A shell block run from a Session's thread, through a real Fleet and a real shell.

use std::time::Duration;

use api::{ObservedSessionRun, SessionRunEvent, Sessions};
use ipc::{SessionFact, SessionId, SessionReport, StartSessionRun, StopSessionRun};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;

use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

async fn a_session_in(fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>, dir: &str) -> SessionId {
    let id = SessionId::carried("s1");
    fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: id.clone(),
            fact: SessionFact::Started {
                cwd: dir.into(),
                title: None,
                origin: ipc::SessionOrigin::Terminal,
                mod_version: None,
            },
        })
        .await
        .expect("kept");
    id
}

/// Everything the run printed and how it ended, read as a socket would.
async fn whole(observed: ObservedSessionRun) -> (Vec<String>, Option<i32>) {
    let mut lines = observed.history;
    let mut live = match observed.live {
        Err(code) => return (lines, code),
        Ok(live) => live,
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match live.recv().await.expect("the run ends with exited") {
                SessionRunEvent::Lines(more) => lines.extend(more),
                SessionRunEvent::Exited(code) => return (lines, code),
            }
        }
    })
    .await
    .expect("the run ended")
}

#[tokio::test]
async fn a_block_runs_in_the_sessions_directory_and_its_output_is_read() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let dir = TempDir::new();
    let session = a_session_in(&fleet, &dir.path().to_string_lossy()).await;

    let underway = fleet
        .start_session_run(StartSessionRun {
            session_id: session,
            command: "echo hi; pwd; echo oops >&2".into(),
        })
        .await
        .expect("underway");
    let (lines, code) = whole(fleet.observe_session_run(underway.run_id.clone()).await.unwrap()).await;

    assert_eq!(lines[0], "hi");
    assert!(lines.iter().any(|line| line == "oops"), "stderr is merged: {lines:?}");
    assert_eq!(code, Some(0));

    let (again, replayed) =
        whole(fleet.observe_session_run(underway.run_id).await.unwrap()).await;
    assert_eq!((again[0].as_str(), replayed), ("hi", Some(0)), "an ended run replays");
}

#[tokio::test]
async fn stop_ends_a_sleeping_block_promptly_with_no_code() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let dir = TempDir::new();
    let session = a_session_in(&fleet, &dir.path().to_string_lossy()).await;
    let underway = fleet
        .start_session_run(StartSessionRun {
            session_id: session,
            command: "sleep 30".into(),
        })
        .await
        .unwrap();
    let observed = fleet.observe_session_run(underway.run_id.clone()).await.unwrap();

    let stopped = fleet
        .stop_session_run(StopSessionRun { run_id: underway.run_id.clone() })
        .await
        .expect("in flight");
    assert_eq!(stopped.run_id, underway.run_id);
    let started = std::time::Instant::now();
    let (_, code) = whole(observed).await;

    assert_eq!(code, None);
    assert!(started.elapsed() < Duration::from_secs(5), "ended promptly");
    assert!(
        fleet.stop_session_run(StopSessionRun { run_id: underway.run_id }).await.is_err(),
        "a run that ended is not in flight"
    );
}

#[tokio::test]
async fn an_ended_or_unknown_session_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let dir = TempDir::new();
    let session = a_session_in(&fleet, &dir.path().to_string_lossy()).await;
    fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: session.clone(),
            fact: SessionFact::Ended { reason: "prompt_input_exit".into() },
        })
        .await
        .unwrap();
    let run = |session_id| StartSessionRun { session_id, command: "echo hi".into() };

    assert!(matches!(
        fleet.start_session_run(run(session)).await,
        Err(api::Refusal::Unacceptable(_))
    ));
    assert!(matches!(
        fleet.start_session_run(run(SessionId::carried("nobody"))).await,
        Err(api::Refusal::Unacceptable(_))
    ));
}
