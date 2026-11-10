//! `edited_at` on a message row, and the body of `edit_session_message`.

use serde_json::json;

use crate::{EditSessionMessage, SessionRow};

#[test]
fn a_message_row_kept_before_edits_existed_decodes_with_no_mark_and_writes_none() {
    let old = json!({"kind": "message", "id": "r1", "at": "2026-11-01T10:00:00.000Z", "from": {"kind": "you"}, "text": "an old one"});
    let read: SessionRow = serde_json::from_value(old.clone()).unwrap();
    assert!(matches!(&read, SessionRow::Message { edited_at: None, .. }));
    assert_eq!(serde_json::to_value(&read).unwrap(), old, "an unedited row is written as it always was");
}

#[test]
fn an_edited_message_row_carries_when_it_was_edited() {
    let edited = json!({
        "kind": "message", "id": "r1", "at": "2026-11-01T10:00:00.000Z", "from": {"kind": "you"}, "text": "new",
        "edited_at": "2026-11-01T10:05:00.000Z"
    });
    let read: SessionRow = serde_json::from_value(edited.clone()).unwrap();
    match &read {
        SessionRow::Message { edited_at: Some(at), .. } => assert_eq!(at.as_str(), "2026-11-01T10:05:00.000Z"),
        other => panic!("not an edited message: {other:?}"),
    }
    assert_eq!(serde_json::to_value(&read).unwrap(), edited);
}

#[test]
fn an_edit_is_a_job_a_row_and_the_new_words() {
    let body = json!({"session_id": "s1", "row_id": "r1", "text": "new words"});
    let read: EditSessionMessage = crate::decode("an edit", body.to_string().as_bytes()).unwrap();
    assert_eq!((read.session_id.as_str(), read.row_id.as_str(), read.text.as_str()), ("s1", "r1", "new words"));
    assert_eq!(serde_json::to_value(&read).unwrap(), body);
}
