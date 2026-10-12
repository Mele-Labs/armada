//! `settings.json` in and out: what a person can type reads back as typed, a
//! file that is not one object is refused with where it went wrong, and a write
//! is sorted, pretty and whole.

use std::collections::BTreeMap;

use crate::settings_file::{self, SettingValue, SettingsFileError};
use crate::tests::tmp::TempDir;

#[test]
fn no_file_is_nothing_saved_and_not_an_error() {
    let home = TempDir::new();
    assert_eq!(settings_file::read(&home.db().with_file_name("settings.json")), Ok(None));
}

#[test]
fn a_written_document_reads_back_whole_with_its_keys_sorted() {
    let home = TempDir::new();
    let path = home.db().with_file_name("settings.json");
    let mut document = BTreeMap::new();
    document.insert("limits.dronesAtOnce".to_string(), SettingValue::Integer(3));
    document.insert("bridge.theme".to_string(), SettingValue::Text("calm".into()));
    document.insert(
        "models.roster".to_string(),
        SettingValue::List(vec![SettingValue::Text("a".into())]),
    );
    settings_file::write(&path, &document).expect("written");

    let text = std::fs::read_to_string(&path).expect("there");
    assert!(text.ends_with("}\n"), "{text}");
    assert!(text.find("bridge.theme") < text.find("limits.dronesAtOnce"), "{text}");
    assert_eq!(settings_file::read(&path), Ok(Some(document)));
    let leftovers: Vec<_> = std::fs::read_dir(path.parent().expect("a dir"))
        .expect("listed")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "nothing left beside it");
}

#[test]
fn text_that_is_not_json_names_the_line_and_column() {
    let refused = settings_file::parse("{\n  \"a\": 1,\n  \"b\" 2\n}\n").expect_err("not JSON");
    let SettingsFileError::NotJson { line, .. } = &refused else { panic!("{refused:?}") };
    assert_eq!(*line, 3);
    assert!(refused.to_string().starts_with("settings.json is not JSON at line 3"), "{refused}");
}

#[test]
fn json_that_is_not_one_object_is_refused() {
    assert_eq!(settings_file::parse("[1, 2]"), Err(SettingsFileError::NotAnObject));
}

#[test]
fn json_text_round_trips_through_a_value() {
    let value = SettingValue::from_json_text(r#"{"version":1,"tabs":["a"]}"#).expect("JSON");
    assert_eq!(value.to_json_text(), r#"{"tabs":["a"],"version":1}"#);
}
