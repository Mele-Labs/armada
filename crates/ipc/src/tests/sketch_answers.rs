//! `answer_sketch` on the wire, as Bridge's mirror in `packages/protocol/src/sessions.ts` spells it.

use serde_json::json;

use crate::{AnswerSketch, SketchHow};

#[test]
fn an_approval_reads_without_words_and_writes_none() {
    let said: AnswerSketch = crate::decode(
        "an answer to a sketch",
        br#"{"session_id":"s1","sketch":"sketch:Flow","drawn":"r7","how":"approved"}"#,
    )
    .unwrap();
    assert_eq!((said.how, said.words.as_deref()), (SketchHow::Approved, None));
    assert_eq!(
        serde_json::to_value(&said).unwrap(),
        json!({"session_id":"s1","sketch":"sketch:Flow","drawn":"r7","how":"approved"})
    );
}

#[test]
fn a_send_back_carries_its_note_and_how_is_spelled_as_detail_settled_holds_it() {
    let said: AnswerSketch = crate::decode(
        "an answer to a sketch",
        br#"{"session_id":"s1","sketch":"sketch:Flow","drawn":"r7","how":"sent_back","words":"Smaller."}"#,
    )
    .unwrap();
    assert_eq!((said.how, said.words.as_deref()), (SketchHow::SentBack, Some("Smaller.")));
    for how in [SketchHow::Approved, SketchHow::SentBack] {
        assert_eq!(serde_json::to_value(how).unwrap(), json!(how.word()));
    }
    let unknown = crate::decode::<AnswerSketch>(
        "an answer to a sketch",
        br#"{"session_id":"s1","sketch":"sketch:Flow","drawn":"r7","how":"fixed"}"#,
    );
    assert!(unknown.is_err(), "a sketch is approved or sent back, never fixed");
}
