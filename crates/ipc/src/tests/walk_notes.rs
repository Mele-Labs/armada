//! Walk notes on the wire (23.16): absent where empty and false, so a 23.15
//! peer reads and sends exactly what it did.

use crate::tests::{detail_of, job};
use crate::{decode, encode, CaptureServed, ChangesRequested, WalkNote};

#[test]
fn a_note_sent_back_without_walk_notes_reads_and_writes_as_before() {
    let old: ChangesRequested = decode("a note", br#"{"note":"fix it"}"#).expect("decodes");
    assert!(!old.with_walk_notes);
    assert_eq!(encode(&old).expect("encodes"), r#"{"note":"fix it"}"#);
    let carrying: ChangesRequested =
        decode("a note", br#"{"note":"","with_walk_notes":true}"#).expect("decodes");
    assert!(carrying.with_walk_notes);
}

#[test]
fn a_detail_with_no_walk_notes_carries_no_key_and_one_with_them_round_trips() {
    let mut detail = detail_of(&job(), &[]);
    assert!(!encode(&detail).expect("encodes").contains("walk_notes"));
    detail.walk_notes.push(WalkNote {
        id: "01NOTE".into(),
        said: "this button is too small".into(),
        at: crate::Instant::carried("2026-10-04T10:00:00.000Z"),
        element: "button “Save”".into(),
        selector: "button.save".into(),
        location: "/settings".into(),
        served: Some(CaptureServed {
            run: "01RUN".into(),
            name: "mock".into(),
            address: "http://127.0.0.1:5173".into(),
        }),
        frame: None,
        sent: false,
    });
    let text = encode(&detail).expect("encodes");
    assert!(text.contains(r#""walk_notes":[{"id":"01NOTE""#), "{text}");
    assert!(
        !text.contains(r#""frame":null"#),
        "an absent frame is no key"
    );
    let back: crate::JobDetail = decode("a detail", text.as_bytes()).expect("decodes");
    assert_eq!(back, detail);
}
