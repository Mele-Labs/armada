//! A Job's several terminals, through a real Fleet, real shells and real pseudo-terminals: each is
//! a shell of its own, an ID names the same terminal for as long as the Job lives, and the agent
//! reaches the one it names.

use std::sync::Arc;

use api::{Refusal, Sessions};
use ipc::{ListTerminals, ListedTerminal, OpenTerminal, ReadTerminal, RenameTerminal, SessionRunUnderway, ShowTerminal, TypeInTerminal};
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::session_running::{rig, Rig};
use crate::tests::session_host::eventually;
use crate::tests::session_terminal::{next_terminal, shortly, typing};

async fn opened(rig: &Rig, terminal: Option<&str>, new: bool) -> Result<SessionRunUnderway, Refusal> {
    let open = OpenTerminal { session_id: rig.session.clone(), terminal: terminal.map(str::to_string), new, cols: None, rows: None };
    Arc::clone(&rig.fleet).open_terminal(open).await
}

fn typing_in(rig: &Rig, terminal: Option<&str>, new: bool, text: &str) -> TypeInTerminal {
    TypeInTerminal { terminal: terminal.map(str::to_string), new, ..typing(rig, text, true, Some(5)) }
}

async fn types(rig: &Rig, terminal: Option<&str>, new: bool, text: &str) -> Result<ipc::TerminalTyped, Refusal> {
    Arc::clone(&rig.fleet).type_in_terminal(None, typing_in(rig, terminal, new, text)).await
}

fn reading(rig: &Rig, terminal: Option<&str>) -> ReadTerminal {
    ReadTerminal { session_id: Some(rig.session.clone()), terminal: terminal.map(str::to_string), ..Default::default() }
}

async fn listed(rig: &Rig) -> Vec<ListedTerminal> {
    let asked = ListTerminals { session_id: Some(rig.session.clone()) };
    rig.fleet.list_terminals(None, asked).await.expect("listed").terminals
}

fn showing(rig: &Rig, terminal: Option<&str>) -> ShowTerminal {
    ShowTerminal { session_id: Some(rig.session.clone()), terminal: terminal.map(str::to_string) }
}

/// The refusal's words, where a call was refused as unacceptable.
fn said<T: std::fmt::Debug>(refused: Result<T, Refusal>) -> String {
    match refused {
        Err(Refusal::Unacceptable(error)) => error.message.clone(),
        other => panic!("refused as unacceptable: {other:?}"),
    }
}

/// Stop a shell and wait for it to go, where it is still there: a runtime that shuts down waits on a
/// shell still open, so a test ends its shells before it asserts.
async fn ends(rig: &Rig, run_id: &str) {
    let _ = rig.fleet.stop_session_run(ipc::StopSessionRun { run_id: run_id.into() }).await;
    eventually(|| async { rig.observes(run_id).await.live.is_err() }).await;
}

fn printed_line(printed: &str, line: &str) -> bool {
    printed.lines().any(|one| one.trim() == line)
}

#[tokio::test]
async fn two_terminals_in_one_job_are_separate_shells_and_the_agent_reaches_the_one_it_names() {
    let rig = rig().await;
    let first = types(&rig, None, true, "WHO=alpha").await.expect("typed");
    let second = types(&rig, None, true, "echo \"<${WHO}>\"").await.expect("typed");
    let named = types(&rig, Some("t1"), false, "echo \"<${WHO}>\"").await.expect("typed");
    let (a, b) = (first.run_id.clone(), second.run_id.clone());
    ends(&rig, &a).await;
    ends(&rig, &b).await;

    assert_eq!((first.terminal.as_str(), second.terminal.as_str(), named.terminal.as_str()), ("t1", "t2", "t1"));
    assert_ne!(first.run_id, second.run_id, "a shell each");
    assert!(printed_line(&second.printed, "<>"), "t2 never heard WHO: {:?}", second.printed);
    assert!(printed_line(&named.printed, "<alpha>"), "t1 holds it: {:?}", named.printed);
    assert_eq!(named.run_id, first.run_id);
}

#[tokio::test]
async fn an_id_is_never_given_twice_in_a_job_and_the_open_ones_are_listed_by_number() {
    let rig = rig().await;
    let one = opened(&rig, None, true).await.expect("opened");
    let two = opened(&rig, None, true).await.expect("opened");
    ends(&rig, &two.run_id).await;
    let three = opened(&rig, None, true).await.expect("opened");
    let open = listed(&rig).await;
    ends(&rig, &one.run_id).await;
    ends(&rig, &three.run_id).await;

    assert_eq!((one.terminal.as_deref(), two.terminal.as_deref()), (Some("t1"), Some("t2")));
    assert_eq!(three.terminal.as_deref(), Some("t3"), "t2 ended and is not given again");
    let ids: Vec<&str> = open.iter().map(|one| one.terminal.as_str()).collect();
    assert_eq!(ids, ["t1", "t3"]);
    assert_eq!(open[1].name, "Terminal 3");
    assert_eq!(open[1].run_id, three.run_id);
}

#[tokio::test]
async fn opening_a_named_terminal_that_ended_starts_a_new_shell_under_the_same_id() {
    let rig = rig().await;
    let one = opened(&rig, None, true).await.expect("opened");
    let two = opened(&rig, None, true).await.expect("opened");
    let running = opened(&rig, Some("t2"), false).await.expect("opened");
    ends(&rig, &two.run_id).await;

    let after = opened(&rig, Some("t2"), false).await.expect("opened");
    let open = listed(&rig).await;
    ends(&rig, &one.run_id).await;
    ends(&rig, &after.run_id).await;
    assert_eq!((running.terminal.as_deref(), running.run_id.as_str()), (Some("t2"), two.run_id.as_str()), "the running shell is handed back");
    assert_eq!(after.terminal.as_deref(), Some("t2"));
    assert_ne!(after.run_id, two.run_id);
    let ids: Vec<&str> = open.iter().map(|one| one.terminal.as_str()).collect();
    assert_eq!(ids, ["t1", "t2"], "and no third was minted");
}

#[tokio::test]
async fn opening_an_id_the_job_never_had_or_both_new_and_an_id_is_refused() {
    let rig = rig().await;
    let one = opened(&rig, None, false).await.expect("opened");
    let (never, odd, both) = (
        opened(&rig, Some("t7"), false).await,
        opened(&rig, Some("t01"), false).await,
        opened(&rig, Some("t1"), true).await,
    );
    ends(&rig, &one.run_id).await;
    let (never, odd, both) = (said(never), said(odd), said(both));
    assert!(never.contains("t7"), "{never}");
    assert!(odd.contains("t01"), "{odd}");
    assert!(both.contains("new") && both.contains("terminal"), "{both}");
}

#[tokio::test]
async fn opening_with_neither_takes_the_lowest_open_terminal_else_reopens_the_lowest_the_job_had() {
    let rig = rig().await;
    let one = opened(&rig, None, true).await.expect("opened");
    let two = opened(&rig, None, true).await.expect("opened");
    ends(&rig, &one.run_id).await;
    let lowest_open = opened(&rig, None, false).await.expect("opened");
    ends(&rig, &two.run_id).await;
    let reopened = opened(&rig, None, false).await.expect("opened");
    ends(&rig, &reopened.run_id).await;
    ends(&rig, &lowest_open.run_id).await;

    assert_eq!((lowest_open.terminal.as_deref(), lowest_open.run_id.as_str()), (Some("t2"), two.run_id.as_str()));
    assert_eq!(reopened.terminal.as_deref(), Some("t1"), "the lowest the Job has had");
}

#[tokio::test]
async fn unnamed_typing_with_no_last_used_opens_a_terminal_of_its_own_and_not_the_persons() {
    let rig = rig().await;
    let persons = opened(&rig, None, false).await.expect("opened");
    let typed = types(&rig, None, false, "echo mine-$((1 + 1))").await.expect("typed");
    let held = rig.observes(&persons.run_id).await.history;
    ends(&rig, &persons.run_id).await;
    ends(&rig, &typed.run_id).await;

    assert_eq!(typed.terminal, "t2");
    assert_ne!(typed.run_id, persons.run_id);
    assert!(printed_line(&typed.printed, "mine-2"), "{typed:?}");
    assert!(!held.contains("mine-2"), "nothing reached the person's: {held:?}");
}

#[tokio::test]
async fn unnamed_typing_goes_to_the_terminal_the_agent_last_used_while_it_is_open() {
    let rig = rig().await;
    let a = types(&rig, None, true, "echo a").await.expect("typed");
    let b = types(&rig, None, true, "echo b").await.expect("typed");
    let last = types(&rig, None, false, "echo c").await.expect("typed");
    let back = types(&rig, Some("t1"), false, "echo d").await.expect("typed");
    let then = types(&rig, None, false, "echo e").await.expect("typed");
    for run in [&a.run_id, &b.run_id] {
        ends(&rig, run).await;
    }
    assert_eq!((last.terminal.as_str(), last.run_id.as_str()), ("t2", b.run_id.as_str()));
    assert_eq!((back.terminal.as_str(), then.terminal.as_str()), ("t1", "t1"), "naming t1 makes it the last used");
}

#[tokio::test]
async fn the_refusal_for_a_replaced_shell_is_per_terminal() {
    let rig = rig().await;
    let t1 = types(&rig, None, true, "echo one").await.expect("typed");
    let t2 = types(&rig, None, true, "echo two").await.expect("typed");
    ends(&rig, &t2.run_id).await;
    let replaced = opened(&rig, Some("t2"), false).await.expect("opened");
    shortly(|| async { !rig.observes(&replaced.run_id).await.history.is_empty() }).await;

    let refused = types(&rig, Some("t2"), false, "echo again").await;
    let other = types(&rig, Some("t1"), false, "echo still-$((1 + 1))").await;
    let read = rig.fleet.read_terminal(None, reading(&rig, Some("t2"))).await;
    let typed = types(&rig, Some("t2"), false, "echo now-$((1 + 1))").await;
    ends(&rig, &t1.run_id).await;
    ends(&rig, &replaced.run_id).await;

    assert_eq!(replaced.terminal.as_deref(), Some("t2"));
    assert_ne!(replaced.run_id, t2.run_id);
    let (refused, read) = (said(refused), read.expect("read"));
    assert!(refused.starts_with("nothing was typed") && refused.contains("t2") && refused.contains(&replaced.run_id), "{refused}");
    assert!(printed_line(&other.expect("t1 was not replaced").printed, "still-2"));
    assert_eq!(read.terminal, "t2");
    assert!(printed_line(&typed.expect("typed once read").printed, "now-2"));
}

#[tokio::test]
async fn naming_a_terminal_that_is_not_open_is_refused_for_typing_reading_and_new_with_an_id() {
    let rig = rig().await;
    let one = opened(&rig, None, false).await.expect("opened");
    let never = types(&rig, Some("t5"), false, "echo hi").await;
    let read_never = rig.fleet.read_terminal(None, reading(&rig, Some("t5"))).await;
    let both = types(&rig, Some("t1"), true, "echo hi").await;
    ends(&rig, &one.run_id).await;
    let ended = types(&rig, Some("t1"), false, "echo hi").await;
    let read_ended = rig.fleet.read_terminal(None, reading(&rig, Some("t1"))).await;
    let (never, read_never, both) = (said(never), said(read_never), said(both));
    let (ended, read_ended) = (said(ended), said(read_ended));

    assert_eq!(never, "terminal t5 is not open. list_terminals names the open ones");
    assert_eq!(ended, "terminal t1 is not open. list_terminals names the open ones");
    assert_eq!(read_never, never);
    assert_eq!(read_ended, ended);
    assert!(both.contains("new") && both.contains("terminal"), "{both}");
}

#[tokio::test]
async fn an_unnamed_read_takes_the_last_used_terminal_else_the_lowest_open_one() {
    let rig = rig().await;
    let none = said(rig.fleet.read_terminal(None, reading(&rig, None)).await);
    let a = types(&rig, None, true, "echo a").await.expect("typed");
    let b = types(&rig, None, true, "echo b").await.expect("typed");
    let last = rig.fleet.read_terminal(None, reading(&rig, None)).await.expect("read");
    types(&rig, Some("t1"), false, "echo c").await.expect("typed");
    let back = rig.fleet.read_terminal(None, reading(&rig, None)).await.expect("read");
    ends(&rig, &a.run_id).await;
    let lowest = rig.fleet.read_terminal(None, reading(&rig, None)).await.expect("read");
    ends(&rig, &b.run_id).await;

    assert!(none.contains("no terminal open"), "{none}");
    assert_eq!((last.terminal.as_str(), back.terminal.as_str(), lowest.terminal.as_str()), ("t2", "t1", "t2"));
}

#[tokio::test]
async fn running_is_true_only_while_a_program_other_than_the_shell_has_the_foreground() {
    let rig = rig().await;
    let shell = types(&rig, None, true, "echo up").await.expect("typed");
    let running = || async { listed(&rig).await[0].running };
    let at_the_prompt = running().await;

    types(&rig, Some("t1"), false, "sleep 30").await.expect("typed");
    let during = shortly(|| async { running().await }).await;
    types(&rig, Some("t1"), false, "\u{3}").await.expect("typed");
    let after = shortly(|| async { !running().await }).await;
    ends(&rig, &shell.run_id).await;

    assert!(!at_the_prompt, "the shell holds the foreground at its prompt");
    assert!(during, "sleep 30 took the foreground");
    assert!(after, "and gave it back");
}

#[tokio::test]
async fn a_rename_is_kept_a_blank_restores_the_default_and_an_unknown_terminal_is_refused() {
    let rig = rig().await;
    let one = opened(&rig, None, false).await.expect("opened");
    let rename = |terminal: &str, name: &str| RenameTerminal { session_id: rig.session.clone(), terminal: terminal.into(), name: name.into() };

    let renamed = rig.fleet.rename_terminal(rename("t1", "  build  ")).await.expect("renamed");
    let named = listed(&rig).await[0].name.clone();
    let unknown = said(rig.fleet.rename_terminal(rename("t9", "x")).await);
    ends(&rig, &one.run_id).await;

    let restarted = Arc::new(a_fleet(&rig.home, FakeWorkProduct::changed(&[])));
    let again = Arc::clone(&restarted)
        .open_terminal(OpenTerminal { session_id: rig.session.clone(), terminal: Some("t1".into()), new: false, cols: None, rows: None })
        .await
        .expect("opened");
    let kept = restarted.list_terminals(None, ListTerminals { session_id: Some(rig.session.clone()) }).await.expect("listed").terminals;
    let blank = restarted.rename_terminal(rename("t1", " ")).await.expect("renamed");
    restarted.stop_session_run(ipc::StopSessionRun { run_id: again.run_id.clone() }).await.expect("in flight");

    assert_eq!((renamed.name.as_str(), named.as_str()), ("build", "build"));
    assert_eq!(renamed.run_id, one.run_id);
    assert!(unknown.contains("t9"), "{unknown}");
    assert_eq!(kept[0].name, "build", "kept in the store, not the shell");
    assert_eq!(blank.name, "Terminal 1");
}

#[tokio::test]
async fn ending_the_job_ends_every_one_of_its_shells() {
    let rig = rig().await;
    let one = opened(&rig, None, true).await.expect("opened");
    let two = opened(&rig, None, true).await.expect("opened");
    rig.fleet
        .report_session(ipc::SessionReport {
            harness: "a_harness".into(),
            session_id: rig.session.clone(),
            fact: ipc::SessionFact::Ended { reason: "prompt_input_exit".into() },
        })
        .await
        .unwrap();

    let mut outlived = Vec::new();
    for run in [&one.run_id, &two.run_id] {
        if !shortly(|| async { rig.observes(run).await.live.is_err() }).await {
            outlived.push(run.clone());
            ends(&rig, run).await;
        }
    }
    assert_ne!(one.run_id, two.run_id, "two shells");
    assert!(outlived.is_empty(), "{outlived:?} outlived its Job");
}

#[tokio::test]
async fn each_terminals_opening_and_ending_is_told_by_its_id() {
    let rig = rig().await;
    let mut watching = rig.fleet.events().subscribe();
    let one = opened(&rig, None, true).await.expect("opened");
    let two = opened(&rig, None, true).await.expect("opened");
    ends(&rig, &two.run_id).await;
    ends(&rig, &one.run_id).await;

    let told = |terminal: &str, run: &SessionRunUnderway| ipc::JobTerminal {
        session_id: rig.session.clone(),
        terminal: terminal.into(),
        run_id: run.run_id.clone(),
    };
    assert_eq!(next_terminal(&mut watching).await, ipc::Event::TerminalOpened(told("t1", &one)));
    assert_eq!(next_terminal(&mut watching).await, ipc::Event::TerminalOpened(told("t2", &two)));
    assert_eq!(next_terminal(&mut watching).await, ipc::Event::TerminalExited(told("t2", &two)));
    assert_eq!(next_terminal(&mut watching).await, ipc::Event::TerminalExited(told("t1", &one)));
}

#[tokio::test]
async fn showing_a_named_open_terminal_answers_it_and_publishes_terminal_shown_once() {
    let rig = rig().await;
    let a = types(&rig, None, true, "echo a").await.expect("typed");
    let b = types(&rig, None, true, "echo b").await.expect("typed");
    let mut watching = rig.fleet.events().subscribe();

    let shown = rig.fleet.show_terminal(None, showing(&rig, Some("t1"))).await.expect("shown");
    ends(&rig, &a.run_id).await;
    ends(&rig, &b.run_id).await;

    let told = ipc::JobTerminal { session_id: rig.session.clone(), terminal: "t1".into(), run_id: a.run_id.clone() };
    assert_eq!(next_terminal(&mut watching).await, ipc::Event::TerminalShown(told.clone()));
    assert!(
        matches!(next_terminal(&mut watching).await, ipc::Event::TerminalExited(_)),
        "shown once, then the next terminal event is a shell ending"
    );
    assert_eq!((shown.terminal.as_str(), shown.run_id.as_str()), ("t1", a.run_id.as_str()));
    assert_eq!(shown.name, "Terminal 1");
}

#[tokio::test]
async fn an_unnamed_show_takes_the_last_used_terminal_else_the_lowest_open_one() {
    let rig = rig().await;
    let a = types(&rig, None, true, "echo a").await.expect("typed");
    let b = types(&rig, None, true, "echo b").await.expect("typed");
    let last = rig.fleet.show_terminal(None, showing(&rig, None)).await.expect("shown");
    ends(&rig, &b.run_id).await;
    let lowest = rig.fleet.show_terminal(None, showing(&rig, None)).await.expect("shown");
    ends(&rig, &a.run_id).await;

    assert_eq!((last.terminal.as_str(), last.run_id.as_str()), ("t2", b.run_id.as_str()), "the one typed into last");
    assert_eq!((lowest.terminal.as_str(), lowest.run_id.as_str()), ("t1", a.run_id.as_str()), "t2 ended");
}

#[tokio::test]
async fn showing_a_terminal_that_is_not_open_is_refused_and_publishes_nothing() {
    let rig = rig().await;
    let one = opened(&rig, None, false).await.expect("opened");
    let mut watching = rig.fleet.events().subscribe();
    let never = rig.fleet.show_terminal(None, showing(&rig, Some("t5"))).await;
    ends(&rig, &one.run_id).await;
    let ended = rig.fleet.show_terminal(None, showing(&rig, Some("t1"))).await;

    assert_eq!(said(never), "terminal t5 is not open. list_terminals names the open ones");
    assert_eq!(said(ended), "terminal t1 is not open. list_terminals names the open ones");
    assert!(
        matches!(next_terminal(&mut watching).await, ipc::Event::TerminalExited(_)),
        "no terminal.shown for a refused show"
    );
}

#[tokio::test]
async fn showing_with_no_terminal_open_is_refused() {
    let rig = rig().await;

    let refused = said(rig.fleet.show_terminal(None, showing(&rig, None)).await);

    assert_eq!(refused, "this job has no terminal open. type_in_terminal opens one");
}

#[tokio::test]
async fn showing_a_terminal_is_not_a_use_so_the_agents_last_used_does_not_move() {
    let rig = rig().await;
    let a = types(&rig, None, true, "echo a").await.expect("typed");
    let b = types(&rig, None, true, "echo b").await.expect("typed");

    rig.fleet.show_terminal(None, showing(&rig, Some("t1"))).await.expect("shown");
    let read = rig.fleet.read_terminal(None, reading(&rig, None)).await.expect("read");
    let typed = types(&rig, None, false, "echo c").await.expect("typed");
    ends(&rig, &a.run_id).await;
    ends(&rig, &b.run_id).await;

    assert_eq!(read.terminal, "t2", "an unnamed read still means the last used");
    assert_eq!(typed.terminal, "t2", "and so does unnamed typing");
}
