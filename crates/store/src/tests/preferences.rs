//! A person's saved preferences: absent until saved, whole once saved, still
//! there after a reopen, and a name outside the closed set is refused rather
//! than stored.

use crate::tests::{open, TempDir};
use crate::{Preferences, WriteError};

#[test]
fn a_store_nobody_saved_into_reads_the_shipped_default() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert_eq!(store.preferences().expect("reads"), Preferences::default());
}

#[test]
fn a_save_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .save_preference("where_things_are_open", true)
        .expect("saved");
    drop(store);

    let store = open(&dir);
    assert_eq!(
        store.preferences().expect("reads"),
        Preferences {
            where_things_are_open: true,
            ..Default::default()
        }
    );
}

/// A second save is the one row replaced, not a second row beside it.
#[test]
fn a_second_save_replaces_the_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .save_preference("where_things_are_open", true)
        .expect("first");
    store
        .save_preference("where_things_are_open", false)
        .expect("second");

    assert_eq!(
        store.preferences().expect("reads"),
        Preferences {
            where_things_are_open: false,
            ..Default::default()
        }
    );
}

#[test]
fn a_name_outside_the_closed_set_is_refused_by_name_and_nothing_is_stored() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let refused = store
        .save_preference("where_things_are_purple", true)
        .expect_err("not a preference this build reads");
    assert!(
        matches!(refused, WriteError::UnknownPreference { name } if name == "where_things_are_purple")
    );
    assert_eq!(store.preferences().expect("reads"), Preferences::default());
}

/// The draft default is a preference of its own, kept across a reopen, and
/// saving it moves nothing else.
#[test]
fn the_draft_default_is_saved_beside_the_other_preference() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .save_preference("where_things_are_open", true)
        .expect("first");
    store
        .save_preference("draft_pull_requests", true)
        .expect("second");
    drop(store);

    let store = open(&dir);
    assert_eq!(
        store.preferences().expect("reads"),
        Preferences {
            where_things_are_open: true,
            draft_pull_requests: true,
            ..Default::default()
        }
    );
}

/// The theme is a word in a table of its own: `dark` until saved, kept across a
/// reopen, and saving it moves no switch.
#[test]
fn the_theme_is_dark_until_saved_and_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.preferences().expect("reads").theme, "dark");
    let saved = store.save_theme("calm").expect("saved");
    assert_eq!(saved.theme, "calm");
    assert!(!saved.where_things_are_open);
    drop(store);

    let mut store = open(&dir);
    assert_eq!(store.preferences().expect("reads").theme, "calm");
    store.save_theme("light").expect("replaced");
    assert_eq!(store.preferences().expect("reads").theme, "light");
}

/// A mod nobody switched has no row, and a switch is one row replaced.
#[test]
fn a_mod_switch_is_a_row_and_a_second_one_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.mod_switches().expect("reads").is_empty());
    store.switch_mod("calm", false).expect("off");
    store.switch_mod("loud", true).expect("on");
    store.switch_mod("calm", true).expect("on again");
    drop(store);

    let switches = open(&dir).mod_switches().expect("reads");
    assert_eq!(switches.len(), 2);
    assert_eq!(switches["calm"], true);
    assert_eq!(switches["loud"], true);
}

/// What `settings.json` carries over: a saved row, and nothing for a row nobody
/// wrote — not the default that `preferences()` answers in its place.
#[test]
fn the_saved_rows_are_told_apart_from_the_defaults() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.saved_preferences().expect("reads"), crate::SavedPreferences::default());
    store.save_preference("draft_pull_requests", false).expect("saved");
    store.save_theme("calm").expect("saved");
    store.save_layout_choices(r#"{"version":1}"#).expect("saved");
    let saved = store.saved_preferences().expect("reads");
    assert_eq!(saved.draft_pull_requests, Some(false), "a saved false is still saved");
    assert_eq!(saved.theme.as_deref(), Some("calm"));
    assert_eq!(saved.layout_choices.as_deref(), Some(r#"{"version":1}"#));
    assert_eq!(saved.where_things_are_open, None);
    assert_eq!(saved.key_bindings, None);
}
