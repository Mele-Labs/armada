//! A Job's shell, through a real Fleet, a real shell and a real pseudo-terminal: one per Job, the
//! person's and the agent's alike.

use std::sync::Arc;

use api::Sessions;

use crate::tests::session_host::eventually;
use crate::tests::session_running::{rig, Rig};

#[tokio::test]
async fn a_second_open_hands_back_the_same_shell_and_it_stands_in_the_jobs_directory() {
    let rig = rig().await;
    let first = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.expect("opened");
    let second = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.expect("opened");
    assert_eq!(first, second, "one shell a Job");

    rig.types(&first, "pwd\r").await.expect("in flight");
    let here = std::fs::canonicalize(rig.dir.path()).unwrap();
    rig.printed(&first, &here.to_string_lossy()).await;

    rig.types(&first, "exit\r").await.expect("in flight");
    eventually(|| async {
        rig.fleet.stop_session_run(ipc::StopSessionRun { run_id: first.clone() }).await.is_err()
    })
    .await;
    let third = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.expect("opened");
    assert_ne!(third, first, "a shell that exited is replaced");
    ends(&rig, &third).await;
}

/// Stop a shell and wait for it to go: a runtime that shuts down waits on a shell still open.
async fn ends(rig: &Rig, run_id: &str) {
    rig.fleet.stop_session_run(ipc::StopSessionRun { run_id: run_id.into() }).await.expect("in flight");
    eventually(|| async { rig.observes(run_id).await.live.is_err() }).await;
}
