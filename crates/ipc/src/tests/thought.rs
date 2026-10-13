//! What the wire keeps true about a thought routed: the reading is told apart by `kind`, an absent
//! field is left out and never `null`, the thread's row is told apart by `message`, and the audit
//! event is spelled as `operations/` declares it. `docs/concepts/helm.md`, *Routing a thought*.

use crate::{
    decode, encode, Event, HelmMessage, HelmSent, HelmThoughtRouted, Instant, ManifestId,
    ReadThought, ThoughtKind, ThoughtReading, ThoughtSent, ThoughtTarget,
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
        kind: ThoughtKind::Job,
        id: "01JOB".into(),
        title: "Pin the clock".into(),
        manifest_id: None,
        slot: None,
    };
    let json = encode(&job).expect("plain data");
    assert_eq!(json, r#"{"kind":"job","id":"01JOB","title":"Pin the clock"}"#);
    assert_eq!(decode::<ThoughtTarget>("a target", json.as_bytes()).expect("round-trips"), job);
}

#[test]
fn a_reading_of_a_kind_nobody_sends_is_refused() {
    assert!(decode::<ThoughtReading>("a reading", br#"{"kind":"banana"}"#).is_err());
    assert!(decode::<ThoughtTarget>("a target", br#"{"kind":"studio","id":"x","title":"t"}"#).is_err());
}

#[test]
fn the_words_and_where_they_went_round_trip() {
    let read = decode::<ReadThought>("words", br#"{"text":"retries should back off"}"#).expect("plain data");
    assert_eq!(read.text, "retries should back off");
    let sent = ThoughtSent {
        text: "retries should back off".into(),
        target: a_session(),
        attachments: Vec::new(),
        mentions: Vec::new(),
    };
    let json = encode(&sent).expect("plain data");
    assert_eq!(decode::<ThoughtSent>("sent", json.as_bytes()).expect("round-trips"), sent);
}

#[test]
fn the_thread_row_is_a_sent_message_and_round_trips() {
    let row = HelmMessage::Sent(HelmSent {
        ts: Instant::carried("2026-10-12T09:00:00.000Z"),
        text: "retries should back off".into(),
        target: a_session(),
        files: Vec::new(),
        tags: Vec::new(),
    });
    let json = encode(&row).expect("plain data");
    assert!(json.starts_with(r#"{"message":"sent","ts":"2026-10-12T09:00:00.000Z","text":"retries should back off","target":{"#), "{json}");
    assert_eq!(decode::<HelmMessage>("a message", json.as_bytes()).expect("round-trips"), row);
}

/// The audit event carries where and never the words.
#[test]
fn the_audit_event_is_spelled_as_declared_and_names_no_words() {
    let routed = Event::HelmThoughtRouted(HelmThoughtRouted {
        manifest_id: ManifestId::carried("armada"),
        to: ThoughtKind::Job,
        id: "01JOB".into(),
        at: Instant::carried("2026-10-12T09:00:00.000Z"),
    });
    assert_eq!(routed.kind(), "helm.thought_routed");
    let json = encode(&routed).expect("plain data");
    assert!(json.contains(r#""kind":"helm.thought_routed""#), "{json}");
    assert!(json.contains(r#""to":"job""#), "{json}");
    assert!(!json.contains("text"), "{json}");
}

#[test]
fn a_sent_row_with_files_and_tags_round_trips_and_leaves_empty_ones_out() {
    let row = HelmMessage::Sent(HelmSent {
        ts: Instant::carried("2026-10-12T09:00:00.000Z"),
        text: "see this".into(),
        target: a_session(),
        files: vec![crate::SentFile { id: "f1".into(), name: "shot.png".into(), media_type: "image/png".into() }],
        tags: vec![crate::SessionTag { kind: crate::TagKind::Issue, id: "214".into(), title: "#214 The title".into(), job: None, url: None }],
    });
    let json = encode(&row).expect("plain data");
    assert!(json.contains(r#""files":[{"id":"f1""#) && json.contains(r#""tags":[{"kind":"issue""#), "{json}");
    assert_eq!(decode::<HelmMessage>("a message", json.as_bytes()).expect("round-trips"), row);
    let bare = encode(&HelmMessage::Sent(HelmSent {
        ts: Instant::carried("2026-10-12T09:00:00.000Z"),
        text: "x".into(),
        target: a_session(),
        files: Vec::new(),
        tags: Vec::new(),
    }))
    .expect("plain data");
    assert!(!bare.contains("files") && !bare.contains("tags") && !bare.contains("null"), "{bare}");
}
