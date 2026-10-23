//! An empty optional field leaves no key, because Bridge reads a key as present.
//!

use crate::{decode, encode, DismissWaiting, Instant, ManifestEdited, SessionId, WaitingItem};

/// A removal with no reason leaves no `reason` key, and a Bridge that sends none reads as none.
#[test]
fn a_removal_without_a_reason_carries_no_reason_key_and_reads_without_one() {
    let bare = DismissWaiting { session_id: SessionId::carried("s1"), item_id: "a".into(), reason: None, misread: false };
    let written = encode(&bare).expect("a removal encodes");
    assert_eq!(written, r#"{"session_id":"s1","item_id":"a"}"#);
    let read: DismissWaiting = decode("a removal", written.as_bytes()).expect("it reads");
    assert_eq!(read, bare);
    let why = DismissWaiting { reason: Some("done already".into()), ..bare };
    let written = encode(&why).expect("a removal encodes");
    assert_eq!(written, r#"{"session_id":"s1","item_id":"a","reason":"done already"}"#);
}

/// A removal he marked as misread keeps the mark through a round trip, and one he did not sends no key.
#[test]
fn a_removal_marked_misread_keeps_the_mark_and_an_unmarked_one_sends_no_key() {
    let marked = r#"{"session_id":"s1","item_id":"replied:t:0","misread":true}"#;
    let read: DismissWaiting = decode("a removal", marked.as_bytes()).expect("it reads");
    assert_eq!(encode(&read).expect("a removal encodes"), marked);
    let unmarked = r#"{"session_id":"s1","item_id":"replied:t:0","misread":false}"#;
    let read: DismissWaiting = decode("a removal", unmarked.as_bytes()).expect("it reads");
    assert_eq!(encode(&read).expect("a removal encodes"), r#"{"session_id":"s1","item_id":"replied:t:0"}"#);
}

/// An item he confirmed keeps `confirmed` through a round trip, and one he did not sends no key, so a
/// Bridge that reads a key as present never sees `false`.
#[test]
fn a_confirmed_item_keeps_the_mark_and_an_unconfirmed_one_sends_no_key() {
    let item = r#"{"id":"replied:t:0","text":"Which one?","since":"2026-10-22T09:00:00.000Z","source":"replied"#;
    let confirmed = format!(r#"{item}","confirmed":true}}"#);
    let read: WaitingItem = decode("an item", confirmed.as_bytes()).expect("it reads");
    assert!(read.confirmed);
    assert_eq!(encode(&read).expect("an item encodes"), confirmed);
    let bare = format!(r#"{item}"}}"#);
    let read: WaitingItem = decode("an item", bare.as_bytes()).expect("a Fleet that sends no mark reads");
    assert!(!read.confirmed);
    assert_eq!(encode(&read).expect("an item encodes"), bare);
}

/// An edit's answer from a Fleet that sends no `declared` still reads, and one
/// with nothing to declare sends no key.
#[test]
fn an_edit_answer_with_nothing_declared_carries_no_declared_key() {
    let edited = ManifestEdited {
        path: "armada.yml".to_string(),
        at: Instant::carried("2026-09-12T09:00:00.000Z"),
        text: "version: 1\n".to_string(),
        declared: None,
    };
    let written = encode(&edited).expect("an edit's answer encodes");
    assert!(
        !written.contains("declared") && !written.contains("null"),
        "{written}"
    );
    let read: ManifestEdited = decode("an edit's answer", written.as_bytes()).expect("it reads");
    assert_eq!(read, edited);
}
