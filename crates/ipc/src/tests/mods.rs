//! What a mod looks like on the wire: a broken one is a row with its reason, an
//! empty field is left out and never `null`, and the event carries the list whole.

use crate::{
    decode, encode, Delivered, Event, Instant, ModChecked, ModKind, ModList, ModSummary, Preferences,
    SavePreference, ScaffoldMod, StreamMessage,
};

fn a_mod(name: &str) -> ModSummary {
    ModSummary {
        name: name.to_string(),
        kind: Some(ModKind::Theme),
        version: Some("0.1.0".to_string()),
        description: Some("Softer greys".to_string()),
        enabled: true,
        valid: true,
        reason: None,
        changed_at: Some(Instant::carried("2026-10-08T21:00:00.000Z")),
    }
}

#[test]
fn a_valid_mod_carries_what_it_read_and_no_reason() {
    let json = encode(&a_mod("calm")).expect("plain data");
    assert_eq!(
        json,
        r#"{"name":"calm","kind":"theme","version":"0.1.0","description":"Softer greys","enabled":true,"valid":true,"changed_at":"2026-10-08T21:00:00.000Z"}"#
    );
    assert_eq!(decode::<ModSummary>("a mod", json.as_bytes()).expect("round-trips"), a_mod("calm"));
}

#[test]
fn a_broken_mod_leaves_out_what_could_not_be_read_rather_than_sending_null() {
    let broken = ModSummary {
        name: "bad".to_string(),
        kind: None,
        version: None,
        description: None,
        enabled: true,
        valid: false,
        reason: Some("mod.toml is missing".to_string()),
        changed_at: None,
    };
    let json = encode(&broken).expect("plain data");
    assert_eq!(
        json,
        r#"{"name":"bad","enabled":true,"valid":false,"reason":"mod.toml is missing"}"#
    );
    assert!(!json.contains("null"));
}

#[test]
fn a_kind_this_build_does_not_have_does_not_decode() {
    let body = br#"{"name":"x","kind":"widget","enabled":true,"valid":true}"#;
    decode::<ModSummary>("a mod", body).expect_err("a peer that sends it does not share the vocabulary");
}

#[test]
fn the_event_is_mods_changed_and_carries_the_list_whole() {
    let message = StreamMessage::Event(Delivered {
        cursor: crate::Cursor::at(3),
        event: Event::ModsChanged(ModList { mods: vec![a_mod("calm")] }),
    });
    let json = encode(&message).expect("plain data");
    assert!(json.contains(r#""kind":"mods.changed""#), "{json}");
    assert_eq!(decode::<StreamMessage>("a stream message", json.as_bytes()).expect("round-trips"), message);
}

#[test]
fn the_theme_is_left_out_while_dark_and_a_save_carries_it_as_text() {
    assert_eq!(encode(&Preferences::default()).expect("plain data"), r#"{"where_things_are_open":false}"#);
    let calm = Preferences { theme: "calm".to_string(), ..Preferences::default() };
    let json = encode(&calm).expect("plain data");
    assert_eq!(json, r#"{"where_things_are_open":false,"theme":"calm"}"#);
    assert_eq!(decode::<Preferences>("preferences", json.as_bytes()).expect("round-trips"), calm);
    let before_it: Preferences = decode("preferences", br#"{"where_things_are_open":true}"#).expect("an older Fleet's");
    assert_eq!(before_it.theme, "dark");

    let save = decode::<SavePreference>("a save", br#"{"name":"theme","value":false,"text":"calm"}"#).expect("plain data");
    assert_eq!(save.text.as_deref(), Some("calm"));
    let without = encode(&SavePreference { name: "x".into(), value: true, text: None }).expect("plain data");
    assert!(!without.contains("text"), "{without}");
}

#[test]
fn a_layout_mod_and_the_owners_choices_are_left_out_when_empty_and_never_null() {
    let layout = ModSummary { kind: Some(ModKind::Layout), ..a_mod("tidy") };
    let json = encode(&layout).expect("plain data");
    assert!(json.contains(r#""kind":"layout""#), "{json}");
    assert_eq!(decode::<ModSummary>("a mod", json.as_bytes()).expect("round-trips"), layout);

    let scaffold = |kind| encode(&ScaffoldMod { name: "tidy".into(), description: None, kind }).expect("plain data");
    assert_eq!(scaffold(None), r#"{"name":"tidy"}"#, "a theme scaffold is the body it always was");
    assert_eq!(scaffold(Some(ModKind::Layout)), r#"{"name":"tidy","kind":"layout"}"#);

    let checked = ModChecked { name: "tidy".into(), valid: true, problems: vec![], css: None, layout: Some("{}".into()) };
    assert_eq!(encode(&checked).expect("plain data"), r#"{"name":"tidy","valid":true,"problems":[],"layout":"{}"}"#);

    assert!(!encode(&Preferences::default()).expect("plain data").contains("layout_choices"));
    let chosen = Preferences { layout_choices: r#"{"version":1}"#.into(), ..Preferences::default() };
    let json = encode(&chosen).expect("plain data");
    assert_eq!(decode::<Preferences>("preferences", json.as_bytes()).expect("round-trips"), chosen);
    let before_it: Preferences = decode("preferences", br#"{"where_things_are_open":true}"#).expect("an older Fleet's");
    assert_eq!(before_it.layout_choices, "");
}
