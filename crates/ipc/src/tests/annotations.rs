//! `mark_annotation_done`'s edit of a note: `status` and `updatedAt` change, nothing else does,
//! and a note sent elsewhere is left alone.

use crate::annotations::{is_note_id, marked_done, NotMarked};

const SESSION: &str = "5e55e55e-0000-0000-0000-000000000000";

/// A note as Bridge's `serializeAnnotation` writes one, with its keys in Bridge's order.
fn note(sent_to: &str) -> String {
    format!(
        r#"{{
  "id": "mfz1-abc",
  "status": "open",
  "sent": {{
    "at": "2026-10-15T10:00:00.000Z",
    "sessionId": "{sent_to}",
    "title": "Fix the rail"
  }},
  "text": "This is too tight",
  "component": null,
  "owners": [
    "Rail",
    "Shell"
  ],
  "element": {{
    "tag": "div",
    "text": "Jobs",
    "label": null
  }},
  "box": {{
    "x": 12.5,
    "y": 40,
    "width": 200,
    "height": 18
  }},
  "createdAt": "2026-10-15T09:00:00.000Z",
  "updatedAt": "2026-10-15T09:00:00.000Z"
}}
"#
    )
}

#[test]
fn a_note_sent_here_is_done_and_keeps_every_other_byte() {
    let at = "2026-10-15T11:00:00.000Z";
    let marked = marked_done(note(SESSION).as_bytes(), SESSION, at).unwrap();
    let expected = note(SESSION)
        .replace(r#""status": "open""#, r#""status": "done""#)
        .replace(
            r#""updatedAt": "2026-10-15T09:00:00.000Z""#,
            r#""updatedAt": "2026-10-15T11:00:00.000Z""#,
        );
    assert_eq!(String::from_utf8(marked).unwrap(), expected);
}

#[test]
fn a_note_sent_elsewhere_or_nowhere_is_refused() {
    let at = "2026-10-15T11:00:00.000Z";
    let elsewhere = note("0b0b0b0b-0000-0000-0000-000000000000");
    assert_eq!(marked_done(elsewhere.as_bytes(), SESSION, at), Err(NotMarked::NotSentHere));
    let nowhere = r#"{"id": "a", "status": "open"}"#;
    assert_eq!(marked_done(nowhere.as_bytes(), SESSION, at), Err(NotMarked::NotSentHere));
    assert!(matches!(marked_done(b"[1]", SESSION, at), Err(NotMarked::Unreadable(_))));
    assert!(matches!(marked_done(b"{", SESSION, at), Err(NotMarked::Unreadable(_))));
}

#[test]
fn a_note_id_names_only_a_file_in_its_directory() {
    assert!(is_note_id("mfz1-abc-0"));
    for refused in ["", "../x", "a/b", "a.json", "a b", ".."] {
        assert!(!is_note_id(refused), "{refused:?}");
    }
}
