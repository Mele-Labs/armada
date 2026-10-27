//! `read_terminal` waiting for text or a pattern in what the Job's shell printed since its agent
//! last typed. A real Fleet, a real shell and a real pseudo-terminal.

use std::sync::Arc;
use std::time::{Duration, Instant};

use api::Sessions;
use ipc::ReadTerminal;

use crate::tests::session_running::{rig, Rig};
use crate::tests::session_terminal::{ends, typing};

fn waiting(rig: &Rig, until: &str, regex: bool, wait_seconds: u64) -> ReadTerminal {
    ReadTerminal {
        session_id: Some(rig.session.clone()),
        until: Some(until.into()),
        regex,
        wait_seconds: Some(wait_seconds),
        ..Default::default()
    }
}

async fn types(rig: &Rig, text: &str, wait_seconds: u64) -> String {
    Arc::clone(&rig.fleet).type_in_terminal(None, typing(rig, text, true, Some(wait_seconds))).await.expect("typed").run_id
}

#[tokio::test]
async fn a_read_waits_for_text_printed_after_it_was_asked() {
    let rig = rig().await;
    let shell = types(&rig, "sleep 2; echo ready-$((40 + 2))", 1).await;
    let read = rig.fleet.read_terminal(None, waiting(&rig, "ready-42", false, 20)).await;
    ends(&rig, &shell).await;
    let read = read.expect("read");
    assert_eq!(read.matched, Some(true), "{read:?}");
    assert!(read.screen.contains("ready-42"), "{read:?}");
}

#[tokio::test]
async fn text_printed_between_the_typing_and_the_read_still_counts() {
    let rig = rig().await;
    let shell = types(&rig, "echo fast-$((1 + 1))", 5).await;
    let started = Instant::now();
    let read = rig.fleet.read_terminal(None, waiting(&rig, "fast-[0-9]", true, 20)).await;
    ends(&rig, &shell).await;
    assert_eq!(read.expect("read").matched, Some(true));
    assert!(started.elapsed() < Duration::from_secs(5), "answered at once: {:?}", started.elapsed());
}

#[tokio::test]
async fn text_printed_before_the_last_typing_does_not_count() {
    let rig = rig().await;
    let shell = types(&rig, "echo old-$((2 + 2))", 5).await;
    types(&rig, "echo new", 5).await;
    let read = rig.fleet.read_terminal(None, waiting(&rig, "old-4", false, 1)).await;
    ends(&rig, &shell).await;
    let read = read.expect("read");
    assert_eq!(read.matched, Some(false), "{read:?}");
    assert!(read.screen.contains("old-4"), "the screen is still the whole shell: {read:?}");
}

#[tokio::test]
async fn a_read_with_no_until_answers_no_matched_and_a_broken_pattern_is_refused() {
    let rig = rig().await;
    let shell = types(&rig, "echo hi", 5).await;
    let plain = ReadTerminal { session_id: Some(rig.session.clone()), ..Default::default() };
    let read = rig.fleet.read_terminal(None, plain).await;
    let broken = rig.fleet.read_terminal(None, waiting(&rig, "(", true, 1)).await;
    ends(&rig, &shell).await;
    assert_eq!(read.expect("read").matched, None);
    assert!(matches!(broken, Err(api::Refusal::Unacceptable(_))), "{broken:?}");
}
