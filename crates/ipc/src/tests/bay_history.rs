//! A bay's history on the wire: the entry kinds Bridge matches on, and what an absent field looks like.

use crate::{
    decode, encode, BayHistory, BayHistoryAsk, BayHistoryEntry, BayHolder, BayHolderKind, Instant, ManifestId,
};

fn an_agent() -> BayHolder {
    BayHolder { kind: BayHolderKind::Agent, id: "4120".to_string(), title: None }
}

/// Absent, never null: Bridge types these `field?: T` and compares to `undefined`.
#[test]
fn what_an_entry_does_not_carry_is_left_out_not_sent_as_null() {
    let released = BayHistoryEntry::Released { at: Instant::carried("2026-10-15T10:00:00.000Z"), holder: an_agent() };
    let json = encode(&released).expect("plain data");
    assert_eq!(
        json,
        r#"{"kind":"released","at":"2026-10-15T10:00:00.000Z","holder":{"kind":"agent","id":"4120"}}"#
    );

    let detached = BayHistoryEntry::Checkout {
        at: Instant::carried("2026-10-15T10:01:00.000Z"),
        from: None,
        to: "7f3a".to_string(),
        sha: "7f3a".to_string(),
    };
    let json = encode(&detached).expect("plain data");
    assert!(!json.contains("null") && !json.contains("\"from\""), "{json}");

    let whole = BayHistory { manifest_id: ManifestId::carried("armada"), slot: 10, entries: Vec::new(), cut: false };
    assert_eq!(encode(&whole).unwrap(), r#"{"manifest_id":"armada","slot":10,"entries":[]}"#, "`cut` only when cut");
}

/// The three kinds and the three holders are the spellings `packages/protocol/src/bay-history.ts` lists.
#[test]
fn the_kinds_are_the_spellings_the_mirror_lists() {
    let leased = BayHistoryEntry::Leased {
        at: Instant::carried("2026-10-15T10:00:00.000Z"),
        holder: BayHolder { kind: BayHolderKind::Session, id: "s9".to_string(), title: Some("Fix".to_string()) },
        branch: Some("bug-fix".to_string()),
    };
    let json = encode(&leased).expect("plain data");
    assert!(json.starts_with(r#"{"kind":"leased""#), "{json}");
    assert!(json.contains(r#""holder":{"kind":"session","id":"s9","title":"Fix"}"#), "{json}");
    assert_eq!(decode::<BayHistoryEntry>("an entry", json.as_bytes()).expect("round-trips"), leased);
    for (kind, spelling) in [(BayHolderKind::Session, "session"), (BayHolderKind::Agent, "agent")] {
        assert_eq!(encode(&kind).unwrap(), format!("\"{spelling}\""));
    }
    assert!(decode::<BayHolderKind>("a holder kind", br#""person""#).is_err(), "strict");
}

#[test]
fn an_ask_names_the_repository_and_the_slot() {
    let ask = BayHistoryAsk { manifest_id: ManifestId::carried("armada"), slot: 10 };
    assert_eq!(encode(&ask).unwrap(), r#"{"manifest_id":"armada","slot":10}"#);
}
