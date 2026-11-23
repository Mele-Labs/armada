//! Margin threads: each kept whole with its messages, read back by page, and rounds numbered in order with their notes.

use crate::tests::{open, TempDir};
use crate::{KeptMarginMessage, KeptMarginNote, KeptMarginRound, KeptMarginSuggestion, KeptMarginThread};

fn thread(id: &str, document: &str, at: &str) -> KeptMarginThread {
    KeptMarginThread {
        id: id.to_string(),
        document: document.to_string(),
        quote: String::from("after ten at night"),
        block: Some(String::from("b3")),
        section: Some(String::from("When notices wait")),
        state: String::from("held"),
        gone: false,
        suggestion: None,
        outcome: None,
        opened_at: at.to_string(),
        messages: vec![KeptMarginMessage { voice: "you".into(), text: "Why ten?".into(), at: at.to_string(), held: true }],
    }
}

#[test]
fn a_thread_is_kept_whole_and_read_back_by_page() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_margin_threads("s1", &[thread("m1", "quiet-hours.html", "2026-11-21T10:00:00.000Z")]).expect("kept");
    store.keep_margin_threads("s1", &[thread("m2", "other.html", "2026-11-21T10:01:00.000Z")]).expect("kept");
    assert_eq!(store.margin_thread_count("s1").expect("counts"), 2);

    let mut one = thread("m1", "quiet-hours.html", "2026-11-21T10:00:00.000Z");
    one.state = String::from("suggested");
    one.gone = true;
    one.messages[0].held = false;
    one.messages.push(KeptMarginMessage { voice: "agent".into(), text: "Most people sleep by then.".into(), at: "2026-11-21T10:02:00.000Z".into(), held: false });
    one.suggestion = Some(KeptMarginSuggestion {
        struck: "ten".into(),
        added: "eleven".into(),
        at: "2026-11-21T10:03:00.000Z".into(),
        decided: Some("accepted".into()),
        decided_at: Some("2026-11-21T10:04:00.000Z".into()),
        decided_revision: Some(3),
    });
    store.keep_margin_threads("s1", &[one.clone()]).expect("kept again");

    assert_eq!(store.margin_threads("s1", Some("quiet-hours.html")).expect("reads"), vec![one.clone()]);
    assert_eq!(store.margin_thread("s1", "m1").expect("reads"), Some(one));
    assert_eq!(store.margin_threads("s1", None).expect("reads").len(), 2);
    assert!(store.margin_threads("s2", None).expect("reads").is_empty(), "another Job's are its own");
}

#[test]
fn rounds_are_numbered_in_order_and_keep_their_notes_and_row() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let round = KeptMarginRound {
        seq: 0,
        at: "2026-11-21T10:00:00.000Z".into(),
        document: "quiet-hours.html".into(),
        title: "Quiet hours for notices".into(),
        kind: "comments".into(),
        row: None,
        threads: vec!["m1".into(), "m2".into()],
        notes: vec![KeptMarginNote { thread: Some("m1".into()), section: None, passage: Some("ten".into()), note: "Why ten?".into() }],
    };
    assert_eq!(store.keep_margin_round("s1", &round).expect("kept"), 1);
    assert_eq!(store.keep_margin_round("s1", &KeptMarginRound { kind: "settled".into(), notes: Vec::new(), ..round.clone() }).expect("kept"), 2);
    store.set_margin_round_row("s1", 1, "row-7").expect("row kept");

    let rounds = store.margin_rounds("s1").expect("reads");
    assert_eq!(rounds.iter().map(|one| (one.seq, one.kind.as_str(), one.row.as_deref())).collect::<Vec<_>>(), [(1, "comments", Some("row-7")), (2, "settled", None)]);
    assert_eq!(rounds[0].threads, ["m1", "m2"]);
    assert_eq!(rounds[0].notes, round.notes);
}
