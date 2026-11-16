//! What an aside puts on the wire: the state is a snake_case word, an empty field leaves no key, the event carries the
//! aside whole under `session.aside`, and an aside reads back as it was written.

use crate::{decode, encode, Aside, AsideState, AskAside, Event, Instant, SessionId};

fn an_aside() -> Aside {
    Aside {
        aside_id: "a1".into(),
        session_id: SessionId::carried("s1"),
        question: "why a mutex?".into(),
        quote: None,
        answer: String::new(),
        state: AsideState::Asking,
        failed: None,
        asked_at: Instant::carried("2026-11-15T09:00:00.000Z"),
        answered_at: None,
        told_at: None,
    }
}

#[test]
fn an_aside_leaves_no_key_for_what_it_does_not_hold_and_reads_back() {
    let text = encode(&an_aside()).expect("encoded");
    assert!(text.contains(r#""state":"asking""#), "{text}");
    for absent in ["quote", "failed", "answered_at", "told_at"] {
        assert!(!text.contains(absent), "{absent}: {text}");
    }
    assert_eq!(decode::<Aside>("an aside", text.as_bytes()).expect("read"), an_aside());
}

#[test]
fn the_event_is_the_aside_whole_under_its_kind_and_a_question_needs_no_quote() {
    let told = Aside { state: AsideState::Answered, told_at: Some(Instant::carried("2026-11-15T09:05:00.000Z")), ..an_aside() };
    let text = encode(&Event::SessionAside(told.clone())).expect("encoded");
    assert!(text.contains(r#""kind":"session.aside""#) && text.contains(r#""aside_id":"a1""#), "{text}");
    assert_eq!(decode::<Event>("an event", text.as_bytes()).expect("read"), Event::SessionAside(told));
    let ask: AskAside = decode("a question", br#"{"session_id":"s1","question":"why?"}"#).expect("read");
    assert_eq!(ask.quote, None);
}
