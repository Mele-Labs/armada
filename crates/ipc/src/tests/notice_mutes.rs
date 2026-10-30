//! A mute's body round-trips, its scopes are spelt as words, and a thread row carries the notice
//! it was sent for only when it was one.

use crate::{decode, encode, Instant, Notice, NoticeScope, SessionRow, SessionVoice, SetNoticeMute};

#[test]
fn a_mute_names_its_notice_and_scope_and_round_trips() {
    for (scope, wire) in [
        (NoticeScope::Job { session_id: "s1".into() }, r#"{"kind":"job","session_id":"s1"}"#),
        (NoticeScope::Repo { manifest_id: "m1".into() }, r#"{"kind":"repo","manifest_id":"m1"}"#),
        (NoticeScope::Hour { session_id: "s1".into() }, r#"{"kind":"hour","session_id":"s1"}"#),
    ] {
        let asked = SetNoticeMute { notice: Notice::BehindBase, scope, muted: true };
        let json = encode(&asked).unwrap();
        assert_eq!(json, format!(r#"{{"notice":"behind_base","scope":{wire},"muted":true}}"#));
        assert_eq!(decode::<SetNoticeMute>("a mute", json.as_bytes()).unwrap(), asked);
    }
    let unknown = r#"{"notice":"nothing","scope":{"kind":"job","session_id":"s1"},"muted":true}"#;
    assert!(decode::<SetNoticeMute>("a mute", unknown.as_bytes()).is_err(), "a notice Fleet does not send is refused");
}

#[test]
fn a_message_row_carries_its_notice_and_an_older_row_reads_without_one() {
    let row = |notice: Option<&str>| SessionRow::Message {
        id: "r1".into(),
        at: Instant::carried("2026-10-21T10:00:00.000Z"),
        from: SessionVoice::You,
        text: "hi".into(),
        files: Vec::new(),
        tags: Vec::new(),
        notice: notice.map(String::from),
        queued: false,
    };
    let plain = encode(&row(None)).unwrap();
    assert!(!plain.contains("notice"), "{plain}");
    assert_eq!(decode::<SessionRow>("a row", plain.as_bytes()).unwrap(), row(None));

    let noticed = encode(&row(Some("behind_base"))).unwrap();
    assert!(noticed.contains(r#""notice":"behind_base""#), "{noticed}");
    assert_eq!(decode::<SessionRow>("a row", noticed.as_bytes()).unwrap(), row(Some("behind_base")));
}

/// **The word a row carries is the word a mute is sent by**: `word` and the wire spelling are written
/// apart, so each is read against the other.
#[test]
fn every_notice_carries_the_word_it_is_spelt_as_on_the_wire() {
    use Notice::*;
    for notice in [
        BehindBase, PrChecksFailed, PrConflicts, PrDropped, PrUnmergeable, PrMerged, PrAutoMerge, PrQueued, PrHanded,
        SlotFree, SlotKept, BuildMoved, WriteClash, SlotStranded, MainRed, RailNudge, RailStep, UsageTrip,
        LimitWake, SignInWake, PlanApproved, AutoLandRefused, RetroNote, ReplyClaim, ContextHandoff, WalkFailed,
        WorkflowChanged, DeliveryAsked, DeliveryChosen, WatchSaw, Taught, DocumentSketch, TaskMarked,
        LifecycleRule,
    ] {
        assert_eq!(encode(&notice).unwrap(), format!("\"{}\"", notice.word()));
    }
}
