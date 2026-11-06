//! A terminal watch, through a real Fleet, the Job's real shell and a real pseudo-terminal: it
//! passes on a line printed after it started and quotes it, a terminal that ends first is a failed
//! reading, and a Job's other terminals are not heard.

use std::sync::Arc;
use std::time::Duration;

use api::{Refusal, Sessions, Watches};
use ipc::{StartWatch, StopSessionRun, WatchReading, WatchStarts, WatchState, WatchSummary};

use crate::session_host::eventually;
use crate::session_running::{rig, Rig};

fn waits_for(rig: &Rig, text: &str, regex: bool) -> StartWatch {
    waits_on(rig, text, regex, None)
}

fn waits_on(rig: &Rig, text: &str, regex: bool, terminal: Option<&str>) -> StartWatch {
    StartWatch {
        session_id: Some(rig.session.clone()),
        name: String::from("Server up"),
        starts: WatchStarts::Terminal {
            text: text.to_string(),
            regex,
            terminal: terminal.map(str::to_string),
            gives_up_after: Some(String::from("10m")),
        },
        follows: None,
        set_aside: Vec::new(),
    }
}

async fn the_watch(rig: &Rig) -> WatchSummary {
    rig.fleet.list_watches(None, Some(rig.session.clone()), true).await.unwrap().watches.remove(0)
}

/// Until the watch has ended, for as long as a login shell under the person's own rc may take.
async fn ended(rig: &Rig) -> WatchSummary {
    for _ in 0..600 {
        let watch = the_watch(rig).await;
        if watch.last.is_some() {
            return watch;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the watch read nothing: {:?}", the_watch(rig).await);
}

async fn shell_ends(rig: &Rig, run_id: &str) {
    let _ = rig.fleet.stop_session_run(StopSessionRun { run_id: run_id.into() }).await;
    eventually(|| async { rig.observes(run_id).await.live.is_err() }).await;
}

#[tokio::test]
async fn it_passes_on_a_line_printed_after_it_started_and_quotes_that_line() {
    let rig = rig().await;
    let closed = rig.fleet.start_watch(None, waits_for(&rig, r"READY-\d+", true)).await.expect_err("no terminal");
    assert_eq!((closed.error().code.as_str(), closed.status()), ("fleet.watch_terminal_closed", 409));
    let bad: Refusal = rig.fleet.start_watch(None, waits_for(&rig, "(", true)).await.expect_err("not a pattern");
    assert_eq!(bad.error().code, "fleet.watch_refused");

    let shell = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.expect("opened");
    let started = rig.fleet.start_watch(None, waits_for(&rig, r"READY-\d+", true)).await.expect("started");
    assert_eq!((started.in_terminal.as_deref(), started.regex, started.command.len()), (Some(r"READY-\d+"), true, 0));
    rig.fleet.watches_run_when_due().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    // The line typed is echoed too, and holds no digits after the dash, so only the output matches.
    rig.types(&shell, "echo READY-$((40+2))\r").await.expect("typed");
    let read = ended(&rig).await;
    shell_ends(&rig, &shell).await;
    assert_eq!((read.state, read.live), (WatchState::Passed, false));
    assert!(matches!(&read.last, Some(WatchReading::Read { says, .. }) if says == "The terminal printed \"READY-42\"."), "{:?}", read.last);
}

#[tokio::test]
async fn a_terminal_that_ends_before_the_text_appears_is_a_failed_reading() {
    let rig = rig().await;
    let shell = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.expect("opened");
    rig.fleet.start_watch(None, waits_for(&rig, "never printed", false)).await.expect("started");
    rig.fleet.watches_run_when_due().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    rig.types(&shell, "exit\r").await.expect("typed");
    let read = ended(&rig).await;
    shell_ends(&rig, &shell).await;
    assert_eq!((read.state, read.live), (WatchState::Failed, false), "nothing is left to hear, so it ends");
    assert!(matches!(&read.last, Some(WatchReading::Read { says, finished: true, .. }) if says.contains("ended before")), "{:?}", read.last);
    // Ended, it is not due again, so it listens no more and keeps its one reading.
    rig.fleet.watches_run_when_due().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let after = the_watch(&rig).await;
    assert_eq!((after.state, after.live, after.readings.len()), (WatchState::Failed, false, 1));
}

async fn opened(rig: &Rig, terminal: Option<&str>, new: bool) -> ipc::SessionRunUnderway {
    let open = ipc::OpenTerminal { session_id: rig.session.clone(), terminal: terminal.map(str::to_string), new, cols: None, rows: None };
    Arc::clone(&rig.fleet).open_terminal(open).await.expect("opened")
}

#[tokio::test]
async fn a_watch_hears_only_the_terminal_it_waits_on() {
    let rig = rig().await;
    let (one, two) = (opened(&rig, None, true).await, opened(&rig, None, true).await);
    let started = rig.fleet.start_watch(None, waits_on(&rig, r"READY-\d+", true, Some("t2"))).await.expect("started");
    assert_eq!(started.terminal.as_deref(), Some("t2"));
    rig.fleet.watches_run_when_due().await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    rig.types(&one.run_id, "echo READY-$((40+2))\r").await.expect("typed");
    rig.printed(&one.run_id, "READY-42").await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let unheard = the_watch(&rig).await;

    rig.types(&two.run_id, "echo READY-$((40+3))\r").await.expect("typed");
    let read = ended(&rig).await;
    shell_ends(&rig, &one.run_id).await;
    shell_ends(&rig, &two.run_id).await;
    assert_eq!((unheard.state, unheard.live, unheard.last.is_none()), (WatchState::Waiting, true, true), "{unheard:?}");
    assert_eq!((read.state, read.live), (WatchState::Passed, false));
    assert!(matches!(&read.last, Some(WatchReading::Read { says, .. }) if says == "The terminal printed \"READY-43\"."), "{:?}", read.last);
}

#[tokio::test]
async fn an_unnamed_watch_is_fixed_to_the_agents_last_used_terminal_else_the_lowest_open_one() {
    let rig = rig().await;
    let (one, two) = (opened(&rig, None, true).await, opened(&rig, None, true).await);
    let lowest = rig.fleet.start_watch(None, waits_for(&rig, "a", false)).await.expect("started");

    let read = ipc::ReadTerminal { session_id: Some(rig.session.clone()), terminal: Some("t2".into()), ..Default::default() };
    rig.fleet.read_terminal(None, read).await.expect("read");
    let last = rig.fleet.start_watch(None, waits_for(&rig, "b", false)).await.expect("started");
    let closed = rig.fleet.start_watch(None, waits_on(&rig, "c", false, Some("t3"))).await.expect_err("not open");
    shell_ends(&rig, &one.run_id).await;
    shell_ends(&rig, &two.run_id).await;

    assert_eq!((lowest.terminal.as_deref(), last.terminal.as_deref()), (Some("t1"), Some("t2")));
    assert_eq!((closed.error().code.as_str(), closed.status()), ("fleet.watch_terminal_closed", 409));
}

#[tokio::test]
async fn a_terminal_watch_keeps_which_terminal_it_waits_on_across_a_restart() {
    let rig = rig().await;
    let (one, two) = (opened(&rig, None, true).await, opened(&rig, None, true).await);
    rig.fleet.start_watch(None, waits_on(&rig, "READY-44", false, Some("t2"))).await.expect("started");
    shell_ends(&rig, &one.run_id).await;
    shell_ends(&rig, &two.run_id).await;

    let restarted = Arc::new(crate::daemon::a_fleet(&rig.home, testkit::FakeWorkProduct::changed(&[])));
    let kept = restarted.list_watches(None, Some(rig.session.clone()), true).await.unwrap().watches.remove(0);
    assert_eq!(kept.terminal.as_deref(), Some("t2"));
}
