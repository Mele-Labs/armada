//! An empty optional field leaves no key, because Bridge reads a key as present.
//!

use crate::{decode, encode, DismissWaiting, Instant, ManifestEdited, SessionId};

/// A removal with no reason leaves no `reason` key, and a Bridge that sends none reads as none.
#[test]
fn a_removal_without_a_reason_carries_no_reason_key_and_reads_without_one() {
    let bare = DismissWaiting { session_id: SessionId::carried("s1"), item_id: "a".into(), reason: None };
    let written = encode(&bare).expect("a removal encodes");
    assert_eq!(written, r#"{"session_id":"s1","item_id":"a"}"#);
    let read: DismissWaiting = decode("a removal", written.as_bytes()).expect("it reads");
    assert_eq!(read, bare);
    let why = DismissWaiting { reason: Some("done already".into()), ..bare };
    let written = encode(&why).expect("a removal encodes");
    assert_eq!(written, r#"{"session_id":"s1","item_id":"a","reason":"done already"}"#);
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
