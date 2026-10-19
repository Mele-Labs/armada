//! A preference a person saves survives a restart, and a name outside the
//! closed set is refused by name on the wire code `fleet.unknown_preference`.

use api::{Refusal, Settings};
use ipc::{Preferences, SavePreference};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

fn saving(name: &str, value: bool) -> SavePreference {
    SavePreference {
        name: name.to_string(),
        value,
        text: None,
    }
}

#[tokio::test]
async fn nothing_saved_reads_as_the_shipped_default() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences::default()
    );
}

#[tokio::test]
async fn a_saved_preference_survives_a_restart() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let now = fleet
        .save_preferences(saving("where_things_are_open", true))
        .await
        .expect("saved");
    assert!(now.where_things_are_open);
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences {
            where_things_are_open: true,
            ..Default::default()
        }
    );
}

#[tokio::test]
async fn an_unknown_name_is_refused_by_name_and_saves_nothing() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));

    let refused = fleet
        .save_preferences(saving("where_things_are_purple", true))
        .await
        .expect_err("not a preference this build reads");
    match refused {
        Refusal::Unacceptable(error) => {
            assert_eq!(error.code, "fleet.unknown_preference");
            assert!(error.message.contains("where_things_are_purple"));
        }
        other => panic!("expected Unacceptable, got {other:?}"),
    }
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences::default(),
        "nothing was saved"
    );
}

#[tokio::test]
async fn the_owners_key_bindings_are_a_preference_that_survives_a_restart() {
    // As settings.json gives JSON back: compact, its keys in order.
    const MINE: &str = r#"{"bindings":{"kill":["q"]},"version":1}"#;
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let save = |text: Option<&str>| SavePreference { name: "key_bindings".into(), value: false, text: text.map(str::to_string) };
    assert_eq!(fleet.save_preferences(save(Some(MINE))).await.expect("saved").key_bindings, MINE);
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_preferences().await.expect("reads").key_bindings, MINE);

    let refused = fleet.save_preferences(save(Some(r#"{"version":1,"bindings":{"kill":"q"}}"#))).await.expect_err("not a list");
    assert_eq!((refused.status(), refused.error().code.clone()), (422, "fleet.unacceptable_key_bindings".to_string()));
    assert_eq!(fleet.get_preferences().await.expect("reads").key_bindings, MINE, "nothing moved");

    assert_eq!(fleet.save_preferences(save(Some(""))).await.expect("empty takes them back").key_bindings, "");
}

#[tokio::test]
async fn the_owners_time_format_is_a_preference_that_survives_a_restart() {
    // As settings.json gives JSON back: compact, its keys in order.
    const MINE: &str = r#"{"date":true,"hour12":false,"seconds":true}"#;
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let save = |text: Option<&str>| SavePreference { name: "time_format".into(), value: false, text: text.map(str::to_string) };
    assert_eq!(fleet.save_preferences(save(Some(MINE))).await.expect("saved").time_format, MINE);
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_preferences().await.expect("reads").time_format, MINE);
}

#[tokio::test]
async fn a_time_format_that_is_not_three_switches_is_refused_and_saves_nothing() {
    const MINE: &str = r#"{"date":false,"hour12":true,"seconds":false}"#;
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let save = |text: &str| SavePreference { name: "time_format".into(), value: false, text: Some(text.to_string()) };
    fleet.save_preferences(save(MINE)).await.expect("saved");

    for bad in [
        "hour12",
        r#"["hour12"]"#,
        r#"{"hour12":true,"seconds":true}"#,
        r#"{"hour12":"yes","seconds":true,"date":false}"#,
        r#"{"hour12":true,"seconds":true,"date":false,"zone":true}"#,
    ] {
        let refused = fleet.save_preferences(save(bad)).await.expect_err(bad);
        assert_eq!((refused.status(), refused.error().code.clone()), (422, "fleet.unacceptable_time_format".to_string()), "{bad}");
        assert_eq!(fleet.get_preferences().await.expect("reads").time_format, MINE, "nothing moved by {bad}");
    }
}

#[tokio::test]
async fn an_empty_time_format_takes_the_choice_back() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let save = |text: &str| SavePreference { name: "time_format".into(), value: false, text: Some(text.to_string()) };
    fleet.save_preferences(save(r#"{"date":true,"hour12":true,"seconds":true}"#)).await.expect("saved");
    assert_eq!(fleet.save_preferences(save("")).await.expect("empty takes it back").time_format, "");
    assert_eq!(fleet.get_preferences().await.expect("reads"), Preferences::default());
}
