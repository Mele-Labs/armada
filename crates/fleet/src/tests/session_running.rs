//! A shell block run from a Session's thread, through a real Fleet, a real shell and a real
//! pseudo-terminal.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, ObservedSessionRun, SessionRunEnd, SessionRunEvent, Sessions};
use ipc::{
    ResizeSessionRun, SessionFact, SessionId, SessionReport, SetWaitingFor, StartSessionRun,
    StopSessionRun, TakeHeld, WaitingAct, WaitingActKind, WaitingInput, WriteSessionRun,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::session_running::{plain, whole_characters};
use crate::tests::daemon::a_fleet;
use crate::tests::session_host::eventually;
use crate::tests::tmp::TempDir;

type Running = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const ID: &str = "s1";

pub(super) async fn a_session_in(fleet: &Running, dir: &str) -> SessionId {
    let id = SessionId::carried(ID);
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

pub(super) struct Rig {
    pub(super) fleet: Arc<Running>,
    pub(super) session: SessionId,
    pub(super) dir: TempDir,
    pub(super) home: TempDir,
}

pub(super) async fn rig() -> Rig {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&[])));
    let dir = TempDir::new();
    let session = a_session_in(&fleet, &dir.path().to_string_lossy()).await;
    Rig { fleet, session, dir, home }
}

impl Rig {
    async fn runs(&self, command: &str) -> String {
        self.runs_at(command, None).await
    }

    async fn runs_at(&self, command: &str, size: Option<(u16, u16)>) -> String {
        Arc::clone(&self.fleet)
            .start_session_run(StartSessionRun {
                session_id: self.session.clone(),
                command: command.into(),
                cols: size.map(|(cols, _)| cols),
                rows: size.map(|(_, rows)| rows),
            })
            .await
            .expect("underway")
            .run_id
    }

    pub(super) async fn observes(&self, run_id: &str) -> ObservedSessionRun {
        self.fleet.observe_session_run(run_id.into()).await.expect("held")
    }

    pub(super) async fn types(&self, run_id: &str, data: &str) -> Result<(), api::Refusal> {
        self.fleet
            .write_session_run(WriteSessionRun { run_id: run_id.into(), data: data.into() })
            .await
            .map(|_| ())
    }

    /// Until the run has printed `needle`.
    pub(super) async fn printed(&self, run_id: &str, needle: &str) {
        eventually(|| async { self.observes(run_id).await.history.contains(needle) }).await;
    }
}

/// Each message the run sent, as a socket would, and how it ended.
async fn chunks(observed: ObservedSessionRun) -> (Vec<String>, SessionRunEnd) {
    let mut seen = vec![observed.history];
    let mut live = match observed.live {
        Err(end) => return (seen, end),
        Ok(live) => live,
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            // A reader that falls behind reads on, as the socket does with `missed`.
            match live.recv().await {
                Ok(SessionRunEvent::Output(more)) => seen.push(more),
                Ok(SessionRunEvent::Exited(end)) => return (seen, end),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(closed) => panic!("the run ends with exited: {closed:?}"),
            }
        }
    })
    .await
    .expect("the run ended")
}

/// Everything the run printed and how it ended.
async fn whole(observed: ObservedSessionRun) -> (String, Option<i32>) {
    let (seen, end) = chunks(observed).await;
    (seen.concat(), end.code)
}

#[tokio::test]
async fn a_block_runs_in_the_sessions_directory_and_its_output_is_read() {
    let rig = rig().await;
    let run = rig.runs("echo hi; pwd; echo oops >&2").await;
    let (text, code) = whole(rig.observes(&run).await).await;

    assert!(text.starts_with("hi\r\n"), "{text:?}");
    assert!(text.contains("oops"), "stderr is the same stream: {text:?}");
    let here = std::fs::canonicalize(rig.dir.path()).unwrap();
    assert!(text.contains(&*here.to_string_lossy()), "ran in the session's directory: {text:?}");
    assert_eq!(code, Some(0));

    let (again, replayed) = whole(rig.observes(&run).await).await;
    assert!(again.starts_with("hi"));
    assert_eq!(replayed, Some(0), "an ended run replays");
}

#[tokio::test]
async fn the_command_has_a_terminal_and_the_exit_code_is_its_own() {
    let rig = rig().await;
    let tty = rig.runs("test -t 0 && test -t 1 && test -t 2 && echo $TERM").await;
    let (text, code) = whole(rig.observes(&tty).await).await;
    assert_eq!((code, text.trim()), (Some(0), "xterm-256color"), "{text:?}");

    let failing = rig.runs("exit 7").await;
    assert_eq!(whole(rig.observes(&failing).await).await.1, Some(7));
}

#[tokio::test]
async fn keys_typed_reach_the_command_and_ctrl_c_reaches_it_as_an_interrupt() {
    let rig = rig().await;
    let reading = rig.runs("read x; echo got $x").await;
    rig.types(&reading, "hello\n").await.expect("in flight");
    let (text, code) = whole(rig.observes(&reading).await).await;
    assert!(text.contains("got hello"), "{text:?}");
    assert_eq!(code, Some(0));

    let waiting = rig.runs("trap 'echo interrupted; exit 3' INT; echo ready; while :; do sleep 0.05; done").await;
    rig.printed(&waiting, "ready").await;
    rig.types(&waiting, "\u{3}").await.expect("in flight");
    let (text, code) = whole(rig.observes(&waiting).await).await;
    assert!(text.contains("interrupted"), "{text:?}");
    assert_eq!(code, Some(3));
}

#[tokio::test]
async fn a_resize_is_seen_by_the_command() {
    let rig = rig().await;
    let run = rig.runs_at("stty size; read x; stty size", Some((100, 30))).await;
    rig.printed(&run, "30 100").await;
    let resized = rig
        .fleet
        .resize_session_run(ResizeSessionRun { run_id: run.clone(), cols: 120, rows: 40 })
        .await
        .expect("in flight");
    assert_eq!(resized.run_id, run);
    rig.types(&run, "\n").await.unwrap();
    let (text, _) = whole(rig.observes(&run).await).await;
    assert!(text.contains("40 120"), "{text:?}");

    let plain = rig.runs("stty size").await;
    assert!(whole(rig.observes(&plain).await).await.0.contains("24 80"), "80 by 24 where none is given");
}

#[tokio::test]
async fn stop_ends_a_sleeping_block_promptly_with_no_code() {
    let rig = rig().await;
    let run = rig.runs("sleep 30").await;
    let observed = rig.observes(&run).await;

    let stopped = rig
        .fleet
        .stop_session_run(StopSessionRun { run_id: run.clone() })
        .await
        .expect("in flight");
    assert_eq!(stopped.run_id, run);
    let started = std::time::Instant::now();
    let (_, end) = chunks(observed).await;

    assert_eq!(end, SessionRunEnd { code: None, stopped: true }, "stopped by Stop, not by a signal");
    assert!(started.elapsed() < Duration::from_secs(5), "ended promptly");
    assert!(
        rig.fleet.stop_session_run(StopSessionRun { run_id: run }).await.is_err(),
        "a run that ended is not in flight"
    );
}

#[tokio::test]
async fn a_signal_that_is_not_stop_ends_a_run_with_no_code_and_not_stopped() {
    let rig = rig().await;
    let run = rig.runs("kill -KILL $$").await;
    let (_, end) = chunks(rig.observes(&run).await).await;
    assert_eq!(end, SessionRunEnd { code: None, stopped: false });
}

#[tokio::test]
async fn typing_and_resizing_a_run_that_ended_or_is_unknown_are_refused_as_a_stop_is() {
    let rig = rig().await;
    let run = rig.runs("true").await;
    whole(rig.observes(&run).await).await;

    for id in [run.as_str(), "session-run-999"] {
        assert!(matches!(rig.types(id, "x").await, Err(api::Refusal::Unacceptable(_))));
        assert!(matches!(
            rig.fleet.resize_session_run(ResizeSessionRun { run_id: id.into(), cols: 80, rows: 24 }).await,
            Err(api::Refusal::Unacceptable(_))
        ));
    }
    let live = rig.runs("sleep 30").await;
    let no_cells = rig.fleet.resize_session_run(ResizeSessionRun { run_id: live.clone(), cols: 0, rows: 24 }).await;
    assert!(matches!(no_cells, Err(api::Refusal::Unacceptable(_))));
    rig.fleet.stop_session_run(StopSessionRun { run_id: live }).await.unwrap();
}

#[tokio::test]
async fn a_viewer_that_opens_late_gets_the_held_tail_and_then_the_rest_once() {
    let rig = rig().await;
    let run = rig.runs("echo first; read x; echo second").await;
    rig.printed(&run, "first").await;

    let late = rig.observes(&run).await;
    assert!(late.history.contains("first"));
    rig.types(&run, "\n").await.unwrap();
    let (seen, end) = chunks(late).await;
    let (text, code) = (seen.concat(), end.code);
    assert_eq!(text.matches("first").count(), 1, "{text:?}");
    assert!(text.contains("second"), "{text:?}");
    assert_eq!(code, Some(0));
}

#[tokio::test]
async fn only_the_last_quarter_megabyte_is_held() {
    let rig = rig().await;
    let run = rig.runs("seq 1 80000").await;
    let (_, code) = whole(rig.observes(&run).await).await;
    assert_eq!(code, Some(0));

    let held = rig.observes(&run).await.history;
    assert!(held.len() <= 256 * 1024, "{} bytes", held.len());
    assert!(held.len() > 200 * 1024, "{} bytes", held.len());
    assert!(held.trim_end().ends_with("80000"));
}

#[tokio::test]
async fn a_character_that_arrives_in_two_reads_is_sent_whole() {
    let rig = rig().await;
    let run = rig.runs("printf '\\342\\202'; sleep 0.5; printf '\\254\\n'").await;
    let (seen, _) = chunks(rig.observes(&run).await).await;

    assert!(seen.iter().all(|one| !one.contains('\u{FFFD}')), "{seen:?}");
    assert!(seen.iter().any(|one| one.contains('\u{20AC}')), "{seen:?}");
}

#[test]
fn text_holds_back_what_is_not_yet_a_character_and_replaces_what_never_will_be() {
    let mut pending = vec![b'a', 0xE2, 0x82];
    assert_eq!(whole_characters(&mut pending), "a");
    assert_eq!(pending, vec![0xE2, 0x82]);
    pending.push(0xAC);
    assert_eq!(whole_characters(&mut pending), "\u{20AC}");
    assert!(pending.is_empty());

    let mut pending = vec![b'a', 0xFF, b'b'];
    assert_eq!(whole_characters(&mut pending), "a\u{FFFD}b");
}

#[test]
fn the_screen_is_what_a_person_would_have_read() {
    let drawn = "\u{1b}[1;32mok\u{1b}[0m\r\n10%\r50%\r100%\r\nbar\u{1b}[2K\rdone\r\n\
                 \u{1b}]0;title\u{7}tab\there\r\nbell\u{8}!\r\n";
    assert_eq!(plain::last_lines(drawn, 40), "ok\n100%\ndone\ntab     here\nbel!");
    assert_eq!(plain::last_lines("a\r\nb\r\nc\r\n", 2), "b\nc");
    assert_eq!(plain::last_lines("", 40), "");
}

/// The mod of a terminal Session asking, as a task, so the test can start a run while it holds.
fn mod_holds(rig: &Rig) -> tokio::task::JoinHandle<Vec<String>> {
    let fleet = Arc::clone(&rig.fleet);
    tokio::spawn(async move {
        fleet
            .take_held_messages(TakeHeld { session_id: ID.into(), wait_ms: Some(20_000) })
            .await
            .unwrap()
            .messages
    })
}

async fn waits_to_run(rig: &Rig, command: &str) -> Vec<ipc::WaitingItem> {
    rig.fleet
        .waiting_for(
            None,
            SetWaitingFor {
                session_id: Some(rig.session.clone()),
                items: vec![WaitingInput {
                    id: "r1".into(),
                    text: "Run it".into(),
                    act: Some(WaitingAct { kind: WaitingActKind::Run, target: command.into() }),
                    options: Vec::new(),
                    steps: Vec::new(),
                }],
            },
        )
        .await
        .expect("kept")
        .waiting_for
}

async fn waiting_now(rig: &Rig) -> Vec<ipc::WaitingItem> {
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed");
    listed.sessions.into_iter().find(|one| one.id.as_str() == ID).expect("the session").waiting_for
}

#[tokio::test]
async fn the_session_is_told_how_a_run_ended_and_a_clean_exit_takes_its_item_off() {
    let rig = rig().await;
    let told = mod_holds(&rig);
    eventually(|| async { rig.fleet.hosts().terminals().listening(ID) }).await;
    assert_eq!(waits_to_run(&rig, "echo built; echo \"it\\`s fine\"").await.len(), 1);

    let run = rig.runs("  echo built; echo \"it\\`s fine\"  ").await;
    whole(rig.observes(&run).await).await;
    let said = tokio::time::timeout(Duration::from_secs(5), told).await.expect("told").unwrap();

    assert_eq!(said.len(), 1, "{said:?}");
    let said = &said[0];
    assert!(said.contains("Exit 0."), "{said}");
    assert!(said.contains("Last lines:\n```\nbuilt\nit`s fine\n```"), "{said}");
    assert!(said.ends_with("This settles the item, so Fleet took it off your waiting list. Do not add it back."), "{said}");
    assert!(waiting_now(&rig).await.is_empty(), "the item is gone");
}

#[tokio::test]
async fn a_clean_run_of_a_step_command_leaves_the_item_with_steps_standing() {
    let rig = rig().await;
    let _told = mod_holds(&rig);
    eventually(|| async { rig.fleet.hosts().terminals().listening(ID) }).await;
    rig.fleet
        .waiting_for(
            None,
            SetWaitingFor {
                session_id: Some(rig.session.clone()),
                items: vec![WaitingInput {
                    id: "r1".into(),
                    text: "Set it up".into(),
                    act: Some(WaitingAct { kind: WaitingActKind::Run, target: "echo built".into() }),
                    options: Vec::new(),
                    steps: vec!["Build it: `echo built`".into(), "Check it".into()],
                }],
            },
        )
        .await
        .expect("kept");

    let run = rig.runs("echo built").await;
    whole(rig.observes(&run).await).await;
    assert_eq!(waiting_now(&rig).await.len(), 1, "the procedure is not finished by its first step");
}

#[tokio::test]
async fn a_failed_or_stopped_run_is_told_and_leaves_the_item_standing() {
    let rig = rig().await;
    let told = mod_holds(&rig);
    eventually(|| async { rig.fleet.hosts().terminals().listening(ID) }).await;
    waits_to_run(&rig, "echo nope; exit 2").await;

    let run = rig.runs("echo nope; exit 2").await;
    whole(rig.observes(&run).await).await;
    let said = tokio::time::timeout(Duration::from_secs(5), told).await.expect("told").unwrap();
    assert!(said[0].starts_with("Fleet: Owner ran `echo nope; exit 2` in Bridge. Exit 2. Last lines:"), "{said:?}");
    assert!(!said[0].contains("waiting list"), "{said:?}");
    assert_eq!(waiting_now(&rig).await.len(), 1);

    let told = mod_holds(&rig);
    eventually(|| async { rig.fleet.hosts().terminals().listening(ID) }).await;
    let sleeping = rig.runs("sleep 30").await;
    rig.fleet.stop_session_run(StopSessionRun { run_id: sleeping.clone() }).await.unwrap();
    whole(rig.observes(&sleeping).await).await;
    let said = tokio::time::timeout(Duration::from_secs(5), told).await.expect("told").unwrap();
    assert!(said[0].contains("`sleep 30` in Bridge. Stopped. No output."), "{said:?}");
}

#[tokio::test]
async fn a_hosted_thread_keeps_a_ran_row_and_the_agent_is_sent_the_words_once() {
    let rig = super::session_host::rig_placing(None);
    let id = rig.start().await;
    let run = Arc::clone(&rig.fleet)
        .start_session_run(StartSessionRun {
            session_id: id.clone(),
            command: "echo built; exit 3".into(),
            cols: None,
            rows: None,
        })
        .await
        .expect("underway")
        .run_id;
    let ended = rig.fleet.observe_session_run(run).await.expect("held");
    whole(ended).await;
    eventually(|| async { rig.stand_in.starts().len() == 1 }).await;
    eventually(|| async { !rig.stand_in.process(0).sent.lock().unwrap().is_empty() }).await;

    let rows = rig.rows(&id).await;
    let [ipc::SessionRow::Ran { command, code, tail, .. }] = &rows[..] else {
        panic!("one ran row and no message from Fleet: {rows:?}");
    };
    assert_eq!(command, "echo built; exit 3");
    assert_eq!(*code, Some(3));
    assert_eq!(tail, "built");

    let sent = rig.stand_in.process(0).sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 1, "told once: {sent:?}");
    assert!(sent[0].contains("Owner ran `echo built; exit 3` in Bridge. Exit 3. Last lines:"), "{sent:?}");
}

#[tokio::test]
async fn an_ended_or_unknown_session_is_refused() {
    let rig = rig().await;
    rig.fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: rig.session.clone(),
            fact: SessionFact::Ended { reason: "prompt_input_exit".into() },
        })
        .await
        .unwrap();
    let run = |session_id| StartSessionRun { session_id, command: "echo hi".into(), cols: None, rows: None };

    assert!(matches!(
        Arc::clone(&rig.fleet).start_session_run(run(rig.session.clone())).await,
        Err(api::Refusal::Unacceptable(_))
    ));
    assert!(matches!(
        Arc::clone(&rig.fleet).start_session_run(run(SessionId::carried("nobody"))).await,
        Err(api::Refusal::Unacceptable(_))
    ));
}
