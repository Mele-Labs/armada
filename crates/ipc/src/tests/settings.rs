//! What settings.json's wire must keep true for Bridge's hand-written mirror:
//! a setting's kind is tagged on `kind`, an absent saved value and an unset
//! variable are `null` rather than missing, and a save's `null` is a removal.

use std::collections::BTreeMap;

use crate::{
    decode, encode, Event, SaveSettings, Setting, SettingApplies, SettingKind, SettingValue, SettingsList,
    SettingsRefused,
};

fn a_setting() -> Setting {
    Setting {
        key: "limits.dronesAtOnce".to_string(),
        group: "Fleet".to_string(),
            section: "Limits".to_string(),
        title: "Drones at once".to_string(),
        description: "How many.".to_string(),
        kind: SettingKind::Integer { min: 1, max: 8, unit: Some("Drones".to_string()) },
        default: SettingValue::Integer(2),
        value: SettingValue::Integer(2),
        saved: None,
        applies: SettingApplies::Live,
        pending_restart: false,
        overridden_by_env: None,
    }
}

#[test]
fn a_setting_spells_its_kind_tagged_and_its_absences_as_null() {
    let json = encode(&a_setting()).expect("plain data");
    assert!(json.contains(r#""kind":{"kind":"integer","min":1,"max":8,"unit":"Drones"}"#), "{json}");
    assert!(json.contains(r#""saved":null"#), "{json}");
    assert!(json.contains(r#""overridden_by_env":null"#), "{json}");
    assert!(json.contains(r#""applies":"live""#), "{json}");
    assert_eq!(decode::<Setting>("a setting", json.as_bytes()).expect("round-trips"), a_setting());
    let restart = encode(&SettingApplies::AtRestart).expect("plain data");
    assert_eq!(restart, r#""at_restart""#);
    let list = encode(&SettingKind::TextList).expect("plain data");
    assert_eq!(list, r#"{"kind":"text_list"}"#);
}

#[test]
fn a_save_reads_null_as_a_removal_and_any_json_as_a_value() {
    let body = br#"{"changes":{"limits.dronesAtOnce":3,"bridge.theme":null,"bridge.layout":{"version":1,"rows":["a"]}}}"#;
    let save = decode::<SaveSettings>("settings to save", body).expect("plain data");
    let mut layout = BTreeMap::new();
    layout.insert("rows".to_string(), SettingValue::List(vec![SettingValue::Text("a".into())]));
    layout.insert("version".to_string(), SettingValue::Integer(1));
    assert_eq!(save.changes.get("limits.dronesAtOnce"), Some(&Some(SettingValue::Integer(3))));
    assert_eq!(save.changes.get("bridge.theme"), Some(&None));
    assert_eq!(save.changes.get("bridge.layout"), Some(&Some(SettingValue::Map(layout))));
}

#[test]
fn the_event_carries_the_list_whole_beside_its_kind() {
    let list = SettingsList {
        path: "/machine/settings.json".to_string(),
        refused: Some(SettingsRefused { key: Some("limits.dronesAtOnse".to_string()), reason: "no".to_string() }),
        settings: vec![a_setting()],
    };
    let json = encode(&Event::SettingsChanged(list.clone())).expect("plain data");
    assert!(json.starts_with(r#"{"kind":"settings.changed","path":"/machine/settings.json""#), "{json}");
    assert_eq!(Event::SettingsChanged(list).kind(), "settings.changed");
}
