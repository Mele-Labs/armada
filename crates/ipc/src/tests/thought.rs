//! What the wire keeps true about a thought routed: the reading is told apart by `kind`, an absent
//! field is left out and never `null`, and the audit event is spelled as `operations/` declares it.
//! The `sent` row is `SessionRow::Sent`, held in `session_wire.rs`.

use crate::{
    decode, encode, Event, WordsSent, Instant, ManifestId, ReadThought, ThoughtKind,
    ThoughtReading, ThoughtTarget,
};

fn a_session() -> ThoughtTarget {
    ThoughtTarget {
        kind: ThoughtKind::Session,
        id: "01LISTVIEWSAAAAAAAAAAAAAAA".into(),
        title: "Sessions list views".into(),
        manifest_id: Some(ManifestId::carried("armada")),
        slot: Some(3),
    }
}

#[test]
fn a_reading_is_told_apart_by_its_kind() {
    assert_eq!(encode(&ThoughtReading::Question).expect("plain data"), r#"{"kind":"question"}"#);
    assert_eq!(
        encode(&ThoughtReading::Thought { target: None }).expect("plain data"),
        r#"{"kind":"thought"}"#,
        "no target is left out, never null"
    );
    let matched = ThoughtReading::Thought {
        target: Some(a_session()),
    };
    let json = encode(&matched).expect("plain data");
    assert_eq!(
        json,
        r#"{"kind":"thought","target":{"kind":"session","id":"01LISTVIEWSAAAAAAAAAAAAAAA","title":"Sessions list views","manifest_id":"armada","slot":3}}"#
    );
    assert_eq!(decode::<ThoughtReading>("a reading", json.as_bytes()).expect("round-trips"), matched);
}

#[test]
fn a_target_with_nothing_optional_leaves_both_out() {
    let job = ThoughtTarget {
        kind: ThoughtKind::Session,
        id: "01SESSION".into(),
        title: "Pin the clock".into(),
        manifest_id: None,
        slot: None,
    };
    let json = encode(&job).expect("plain data");
    assert_eq!(json, r#"{"kind":"session","id":"01SESSION","title":"Pin the clock"}"#);
    assert_eq!(decode::<ThoughtTarget>("a target", json.as_bytes()).expect("round-trips"), job);
}

#[test]
fn a_reading_of_a_kind_nobody_sends_is_refused() {
    assert!(decode::<ThoughtReading>("a reading", br#"{"kind":"banana"}"#).is_err());
    assert!(decode::<ThoughtTarget>("a target", br#"{"kind":"studio","id":"x","title":"t"}"#).is_err());
}

#[test]
fn the_words_read_round_trip() {
    let read = decode::<ReadThought>("words", br#"{"text":"retries should back off"}"#).expect("plain data");
    assert_eq!(read.text, "retries should back off");
    let json = encode(&read).expect("plain data");
    assert_eq!(decode::<ReadThought>("words", json.as_bytes()).expect("round-trips"), read);
}

/// The audit event carries where and never the words.
#[test]
fn the_audit_event_is_spelled_as_declared_and_names_no_words() {
    let routed = Event::WordsSent(WordsSent {
        manifest_id: ManifestId::carried("armada"),
        to: ThoughtKind::Session,
        id: "01SESSION".into(),
        at: Instant::carried("2026-10-12T09:00:00.000Z"),
    });
    assert_eq!(routed.kind(), "operator.words_sent");
    let json = encode(&routed).expect("plain data");
    assert!(json.contains(r#""kind":"operator.words_sent""#), "{json}");
    assert!(json.contains(r#""to":"session""#), "{json}");
    assert!(!json.contains("text"), "{json}");
}
