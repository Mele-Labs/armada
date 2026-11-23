//! A round of margin talk must read back from the thread it is sent in: `ThreadItem` is tagged by `kind`, so the
//! round's own kind travels under another name.

use crate::{decode, encode, Instant, MarginRoundKind, ThreadItem, ThreadMargin};

#[test]
fn a_margin_round_in_a_thread_reads_back_whole() {
    let item = ThreadItem::Margin(ThreadMargin {
        id: String::from("margin:1"),
        replaces: Some(String::from("row-7")),
        at: Instant::carried("2026-11-21T09:00:00.000Z"),
        page: String::from("quiet-hours.html"),
        title: String::from("Quiet hours for notices"),
        kind: MarginRoundKind::Comments,
        summary: String::from("Your comments are with the agent"),
        waits_on_you: true,
        threads: vec![String::from("t-1")],
        notes: Vec::new(),
    });
    let sent = encode(&item).expect("a round encodes");
    assert!(sent.contains("\"round\":\"comments\""), "the round's kind goes as `round`: {sent}");
    let read: ThreadItem = decode("a thread item", sent.as_bytes()).expect("a round reads back");
    assert_eq!(read, item);
}
