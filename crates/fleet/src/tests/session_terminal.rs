//! A Job's shell, through a real Fleet, a real shell and a real pseudo-terminal: one per Job, the
//! person's and the agent's alike.

use std::sync::Arc;

use api::Sessions;
use ipc::{OpenTerminal, SessionFact, SessionId, SessionReport, TypeInTerminal};

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

#[tokio::test]
async fn the_person_opens_the_same_shell_and_an_ended_or_unknown_job_is_refused() {
    let rig = rig().await;
    let open = |session_id: &str, cols| OpenTerminal { session_id: SessionId::carried(session_id), cols, rows: None };
    let first = Arc::clone(&rig.fleet).open_terminal(open(rig.session.as_str(), Some(100))).await.expect("opened");
    let again = Arc::clone(&rig.fleet).open_terminal(open(rig.session.as_str(), None)).await.expect("opened");
    assert_eq!(first.run_id, again.run_id);
    assert!(matches!(
        Arc::clone(&rig.fleet).open_terminal(open(rig.session.as_str(), Some(0))).await,
        Err(api::Refusal::Unacceptable(_))
    ));
    ends(&rig, &first.run_id).await;

    rig.fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: rig.session.clone(),
            fact: SessionFact::Ended { reason: "prompt_input_exit".into() },
        })
        .await
        .unwrap();
    for named in [rig.session.as_str(), "nobody"] {
        assert!(matches!(
            Arc::clone(&rig.fleet).open_terminal(open(named, None)).await,
            Err(api::Refusal::Unacceptable(_))
        ));
    }
}

fn typing(rig: &Rig, text: &str, enter: bool, wait_seconds: Option<u64>) -> TypeInTerminal {
    TypeInTerminal { text: text.into(), enter, wait_seconds, session_id: Some(rig.session.clone()) }
}

#[tokio::test]
async fn the_agents_echo_hi_opens_the_shell_and_answers_hi_once_it_goes_quiet() {
    let rig = rig().await;
    let typed = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "echo hi", true, None))
        .await
        .expect("typed");
    assert!(typed.printed.lines().any(|line| line.trim() == "hi"), "{:?}", typed.printed);
    assert!(!typed.still_printing, "{typed:?}");

    let opened = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.unwrap();
    assert_eq!(opened, typed.run_id, "the person's pane opens the shell the agent typed into");
    ends(&rig, &typed.run_id).await;
}

#[tokio::test]
async fn a_server_answers_still_printing_at_the_cap_and_ctrl_c_quiets_it() {
    let rig = rig().await;
    let serving = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "while :; do echo tick; sleep 0.2; done", true, Some(2)))
        .await
        .expect("typed");
    assert!(serving.still_printing, "{serving:?}");
    assert!(serving.printed.contains("tick"), "{serving:?}");

    let stopped = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "\u{3}", false, Some(5)))
        .await
        .expect("typed");
    assert!(!stopped.still_printing, "{stopped:?}");
    ends(&rig, &serving.run_id).await;
}

#[tokio::test]
async fn typing_with_no_job_placed_is_refused() {
    let rig = rig().await;
    let unplaced = TypeInTerminal { session_id: None, ..typing(&rig, "echo hi", true, None) };
    assert!(matches!(
        Arc::clone(&rig.fleet).type_in_terminal(None, unplaced).await,
        Err(api::Refusal::Unacceptable(_))
    ));
}

/// Stop a shell and wait for it to go: a runtime that shuts down waits on a shell still open.
async fn ends(rig: &Rig, run_id: &str) {
    rig.fleet.stop_session_run(ipc::StopSessionRun { run_id: run_id.into() }).await.expect("in flight");
    eventually(|| async { rig.observes(run_id).await.live.is_err() }).await;
}
