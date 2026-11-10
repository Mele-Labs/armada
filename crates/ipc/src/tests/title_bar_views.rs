//! The spellings of what Fleet works out for the title bar and the clash marks, which
//! `packages/protocol` mirrors by hand: a band, Compact's offer, the build switcher and the clashes
//! of every repository.

use crate::{
    decode, encode, AllWriteClashes, BuildSwitcher, ClashHolder, ClashSide, CompactHow, CompactOff, CompactOffer, Event,
    Instant, ManifestId, PlanWindow, RepositoryClashes, RestartAct, RingBand, SessionUsage, WriteClash,
};

#[test]
fn a_band_travels_in_snake_case_and_a_window_from_before_it_reads_as_normal() {
    assert_eq!(encode(&RingBand::Warning).unwrap(), r#""warning""#);
    let kept = br#"{"kind":"five_hour","percent_used":90}"#;
    let window = decode::<PlanWindow>("a window", kept).expect("a window kept before the band existed");
    assert_eq!(window.band, RingBand::Normal);
    let banded = PlanWindow { kind: "five_hour".into(), percent_used: 96, resets_at: None, band: RingBand::Danger };
    assert_eq!(encode(&banded).unwrap(), r#"{"kind":"five_hour","percent_used":96,"band":"danger"}"#);
}

#[test]
fn a_context_reading_leaves_no_key_until_there_is_one() {
    assert_eq!(encode(&SessionUsage::default()).unwrap(), "{}");
    let usage = SessionUsage { context_percent: Some(85), context_band: Some(RingBand::Warning), ..SessionUsage::default() };
    assert_eq!(encode(&usage).unwrap(), r#"{"context_percent":85,"context_band":"warning"}"#);
}

#[test]
fn compact_travels_as_a_state_with_its_route_or_its_reason() {
    let offered = CompactOffer::Offered { how: CompactHow::Terminal };
    assert_eq!(encode(&offered).unwrap(), r#"{"state":"offered","how":"terminal"}"#);
    let off = CompactOffer::Off { why: CompactOff::TerminalNotListed };
    assert_eq!(encode(&off).unwrap(), r#"{"state":"off","why":"terminal_not_listed"}"#);
    assert_eq!(decode::<CompactOffer>("compact", encode(&off).unwrap().as_bytes()).unwrap(), off);
}

#[test]
fn the_build_switcher_travels_as_two_flags_and_an_act() {
    let switcher = BuildSwitcher { aligned: true, idle_on_main: false, restart: RestartAct::RefreshPreview };
    assert_eq!(encode(&switcher).unwrap(), r#"{"aligned":true,"idle_on_main":false,"restart":"refresh_preview"}"#);
}

#[test]
fn the_clashes_of_every_repository_travel_under_write_clashes_changed() {
    let holder = |kind, id: &str| ClashHolder { kind, id: id.into(), label: id.into() };
    let all = AllWriteClashes {
        repositories: vec![RepositoryClashes {
            manifest_id: ManifestId::carried("01A"),
            clashes: vec![WriteClash {
                one: holder(ClashSide::Session, "s1"),
                other: holder(ClashSide::Main, "main"),
                paths: vec!["a.rs".into()],
                since: Instant::carried("2026-11-09T10:00:00.000Z"),
            }],
        }],
    };
    let event = Event::WriteClashesChanged(all.clone());
    assert_eq!(event.kind(), "write_clashes.changed");
    let json = encode(&event).unwrap();
    assert!(json.contains(r#""repositories":[{"manifest_id":"01A","clashes":[{"#), "{json}");
    assert_eq!(decode::<Event>("an event", json.as_bytes()).unwrap(), event);
    assert_eq!(encode(&AllWriteClashes::default()).unwrap(), r#"{"repositories":[]}"#);
}
