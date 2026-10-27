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
    let here = here.to_string_lossy();
    if !shortly(|| async { rig.observes(&first).await.history.contains(&*here) }).await {
        ends(&rig, &first).await;
        panic!("pwd printed {here}: {:?}", rig.observes(&first).await.history);
    }

    rig.types(&first, "exit\r").await.expect("in flight");
    assert!(shortly(|| async { rig.observes(&first).await.live.is_err() }).await, "exit ends it");
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

pub(super) fn typing(rig: &Rig, text: &str, enter: bool, wait_seconds: Option<u64>) -> TypeInTerminal {
    TypeInTerminal { text: text.into(), enter, clear: true, wait_seconds, session_id: Some(rig.session.clone()) }
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

#[tokio::test]
async fn the_agent_reads_what_the_person_typed_and_a_job_with_no_shell_is_refused() {
    let rig = rig().await;
    let read = |lines| {
        let asked = ipc::ReadTerminal { session_id: Some(rig.session.clone()), lines, ..Default::default() };
        rig.fleet.read_terminal(None, asked)
    };
    assert!(matches!(read(None).await, Err(api::Refusal::Unacceptable(_))), "no shell open");

    let shell = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.unwrap();
    rig.types(&shell, "echo from-the-person\r").await.expect("in flight");
    let ran = |screen: &str| screen.lines().any(|line| line.trim() == "from-the-person");
    shortly(|| async { read(None).await.is_ok_and(|read| ran(&read.screen)) }).await;
    let screen = read(None).await.expect("read");
    let one = read(Some(1)).await.expect("read").screen;
    let held = rig.observes(&shell).await.history;
    ends(&rig, &shell).await;

    assert!(ran(&screen.screen), "{:?}\nheld: {held:?}", screen.screen);
    assert_eq!(screen.run_id, shell);
    assert_eq!(one.lines().count(), 1, "{one:?}");
}

/// The next `terminal.*` event, by its kind.
async fn next_terminal(watching: &mut api::Subscription) -> ipc::Event {
    loop {
        match tokio::time::timeout(std::time::Duration::from_secs(20), watching.next()).await.expect("an event") {
            Some(api::Next::Send(delivered)) if delivered.event.kind().starts_with("terminal.") => {
                return delivered.event;
            }
            Some(_) => {}
            None => panic!("the stream ended"),
        }
    }
}

#[tokio::test]
async fn bridge_is_told_when_the_agents_first_command_opens_the_shell_and_when_it_ends() {
    let rig = rig().await;
    let mut watching = rig.fleet.events().subscribe();
    let typed = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "echo hi", true, Some(2)))
        .await
        .expect("typed");
    Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.unwrap();
    ends(&rig, &typed.run_id).await;

    let told = ipc::JobTerminal { session_id: rig.session.clone(), run_id: typed.run_id.clone() };
    assert_eq!(next_terminal(&mut watching).await, ipc::Event::TerminalOpened(told.clone()));
    assert_eq!(
        next_terminal(&mut watching).await,
        ipc::Event::TerminalExited(told),
        "an open handed the shell already open publishes nothing"
    );
}

#[tokio::test]
async fn ending_the_job_ends_its_shell() {
    let rig = rig().await;
    let shell = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.unwrap();
    rig.fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: rig.session.clone(),
            fact: SessionFact::Ended { reason: "prompt_input_exit".into() },
        })
        .await
        .unwrap();

    if !shortly(|| async { rig.observes(&shell).await.live.is_err() }).await {
        ends(&rig, &shell).await;
        panic!("the shell outlived its Job");
    }
}

#[tokio::test]
async fn a_stopped_shell_leaves_well_inside_the_grace_it_is_given() {
    let rig = rig().await;
    let shell = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.unwrap();
    // Until it answers it has not set up to ignore SIGTERM, as an interactive shell does.
    rig.types(&shell, "echo up\r").await.expect("in flight");
    assert!(shortly(|| async { rig.observes(&shell).await.history.contains("up") }).await, "the shell never answered");

    let stopped = std::time::Instant::now();
    rig.fleet.stop_session_run(ipc::StopSessionRun { run_id: shell.clone() }).await.expect("in flight");
    let left = shortly(|| async { rig.observes(&shell).await.live.is_err() }).await;
    let took = stopped.elapsed();
    assert!(left && took < std::time::Duration::from_secs(2), "left: {left}, took {took:?}; GRACE is 3 s");
}

/// Until `check` holds, for as long as an interactive login shell may take to start under the
/// person's own rc files, which is longer than [`eventually`] waits.
pub(super) async fn shortly<F: std::future::Future<Output = bool>>(mut check: impl FnMut() -> F) -> bool {
    for _ in 0..150 {
        if check().await {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    false
}

/// Stop a shell and wait for it to go: a runtime that shuts down waits on a shell still open.
pub(super) async fn ends(rig: &Rig, run_id: &str) {
    rig.fleet.stop_session_run(ipc::StopSessionRun { run_id: run_id.into() }).await.expect("in flight");
    eventually(|| async { rig.observes(run_id).await.live.is_err() }).await;
}
