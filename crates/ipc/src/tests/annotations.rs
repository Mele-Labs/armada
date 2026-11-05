//! `mark_annotation_done`'s edit of a note: `status` and `updatedAt` change, nothing else does,
//! and a note sent elsewhere is left alone.

use crate::annotations::{filed_done, is_note_id, marked_done, sent_to, NotMarked};

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

/// A note nobody has sent anywhere, as Bridge writes it before a send.
fn unsent_note() -> String {
    let sent = note(SESSION);
    let start = sent.find("  \"sent\"").unwrap();
    let end = sent.find("  \"text\"").unwrap();
    format!("{}{}", &sent[..start], &sent[end..])
}

#[test]
fn a_note_sent_by_fleet_gains_sent_before_its_words_and_is_then_the_jobs_to_mark_done() {
    let at = "2026-10-15T11:00:00.000Z";
    let sent = sent_to(unsent_note().as_bytes(), SESSION, "Fix the rail", at).unwrap();

    let expected = note(SESSION)
        .replace("2026-10-15T10:00:00.000Z", at)
        .replace(r#""updatedAt": "2026-10-15T09:00:00.000Z""#, r#""updatedAt": "2026-10-15T11:00:00.000Z""#);
    assert_eq!(String::from_utf8(sent.clone()).unwrap(), expected);
    assert!(marked_done(&sent, SESSION, at).is_ok(), "the Job it went to can mark it done");
}

#[test]
fn a_note_that_names_a_job_is_not_sent_again_and_filing_marks_any_note_done() {
    let at = "2026-10-15T11:00:00.000Z";
    assert_eq!(sent_to(note(SESSION).as_bytes(), SESSION, "x", at), Err(NotMarked::SentAlready));
    assert!(matches!(sent_to(b"[1]", SESSION, "x", at), Err(NotMarked::Unreadable(_))));

    let filed = String::from_utf8(filed_done(unsent_note().as_bytes(), at).unwrap()).unwrap();
    assert_eq!(
        filed,
        unsent_note()
            .replace(r#""status": "open""#, r#""status": "done""#)
            .replace(r#""updatedAt": "2026-10-15T09:00:00.000Z""#, r#""updatedAt": "2026-10-15T11:00:00.000Z""#)
    );
}
