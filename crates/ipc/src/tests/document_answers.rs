//! `answer_document` on the wire, as Bridge's mirror in `packages/protocol/src/sessions.ts` spells it.

use serde_json::json;

use crate::{AnswerDocument, DocumentHow, WaitingSource};

#[test]
fn an_approval_names_the_revision_he_read_and_writes_no_words() {
    let said: AnswerDocument = crate::decode(
        "an answer to a document",
        br#"{"session_id":"s1","document":"document:/r/.armada/documents/a.html","revision":"2","how":"approved"}"#,
    )
    .unwrap();
    assert_eq!((said.revision.as_str(), said.how, said.words.as_deref()), ("2", DocumentHow::Approved, None));
    assert_eq!(
        serde_json::to_value(&said).unwrap(),
        json!({"session_id":"s1","document":"document:/r/.armada/documents/a.html","revision":"2","how":"approved"})
    );
}

#[test]
fn a_send_back_carries_its_note_and_how_and_the_source_are_spelled_as_bridge_reads_them() {
    let said: AnswerDocument = crate::decode(
        "an answer to a document",
        br#"{"session_id":"s1","document":"document:/r/a.html","revision":"1","how":"sent_back","words":"Shorter."}"#,
    )
    .unwrap();
    assert_eq!((said.how, said.words.as_deref()), (DocumentHow::SentBack, Some("Shorter.")));
    for how in [DocumentHow::Approved, DocumentHow::SentBack] {
        assert_eq!(serde_json::to_value(how).unwrap(), json!(how.word()));
    }
    assert_eq!(serde_json::to_value(WaitingSource::Document).unwrap(), json!("document"));
    let fixed = crate::decode::<AnswerDocument>(
        "an answer to a document",
        br#"{"session_id":"s1","document":"document:/r/a.html","revision":"1","how":"fixed"}"#,
    );
    assert!(fixed.is_err(), "a document is approved or sent back, never fixed");
}
