//! settings.json's claim, end to end over a real Fleet and a real file: a save
//! from the wire is written and reaches the next admission, a hand edit is
//! taken without a restart and said on the stream, a bad one is refused with
//! the last good settings left in force, a `null` puts the default back, and a
//! first start carries what the store saved into the file.
//!
//! **Here and not in `crates/acceptance`**: the claim is a file on disk and an
//! admission turn, and that crate writes no file and cannot turn a Fleet.

use std::collections::BTreeMap;
use std::sync::Arc;

use api::Settings;
use config::settings::Supplied;
use ipc::{SaveSettings, SettingValue, SettingsList};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::{Fittings, Fleet};
use crate::settings::migrated;
use crate::slots::Concurrency;
use crate::tests::daemon::fitted_with;
use crate::tests::daemon::one;
use crate::tests::tmp::TempDir;

/// What the composition root supplies, as a fixture would: the fittings'
/// own models and harness, four Checks, and the prompts Fleet ships.
pub(crate) fn supplied() -> Supplied {
    use config::settings as s;
    crate::prompts::supplied(Supplied::new())
        .integer(s::CHECKS_AT_ONCE, 4)
        .words(s::HARNESS_AGENT, "a-harness")
        .words(s::HARNESS_BINARY_PATH, "the-agent")
        .list(s::MODELS_ROSTER, &["a-model", "another-model"])
        .words(s::MODELS_DEFAULT, "a-model")
        .words(s::MODELS_SESSION, "a-model")
        .words(s::MODELS_REVIEW, "a-model")
        .words(s::MODELS_JUDGE, "a-model")
        .words(s::MODELS_PROPOSER, "a-model")
        .words(s::MODELS_RETRO, "another-model")
        .words(s::MODELS_SECOND_OPINION, "another-model")
        .words(s::MODELS_ACCOUNT, "a-model")
        .words(s::PLAN_DOWNSHIFT_TO, "another-model")
        .harnesses(&["a-harness"])
}

/// The file a fixture's Fleet reads, beside its store.
pub(crate) fn settings_file_in(home: &TempDir) -> std::path::PathBuf {
    home.path().join("settings.json")
}

/// Fittings whose Drones keep working, shipping the bound settings.json ships.
fn fitted(home: &TempDir) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]),
    );
    fittings.starting().workflows = one(testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
    }]));
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    fittings.concurrency = Concurrency::of(2);
    fittings
}

fn changing(key: &str, value: Option<SettingValue>) -> SaveSettings {
    SaveSettings { changes: BTreeMap::from([(key.to_string(), value)]) }
}

fn one_of<'a>(list: &'a SettingsList, key: &str) -> &'a ipc::Setting {
    list.settings.iter().find(|one| one.key == key).expect(key)
}

/// A save the table refuses writes nothing and names the key.
#[tokio::test]
async fn a_save_out_of_range_writes_nothing_and_names_the_key() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    let refused = fleet
        .save_settings(changing("limits.dronesAtOnce", Some(SettingValue::Integer(9))))
        .await
        .expect_err("out of range");
    let api::Refusal::Unacceptable(error) = refused else { panic!("{refused:?}") };
    assert_eq!(error.code, "fleet.unacceptable_settings");
    assert!(!settings_file_in(&home).exists(), "nothing was written");
}

/// Main and a preview take turns on one file. A key the other build wrote costs this one nothing,
/// a save writes it back, and a save may not add one of its own.
#[tokio::test]
async fn a_key_another_build_wrote_comes_through_a_save() {
    let home = TempDir::new();
    let file = settings_file_in(&home);
    std::fs::write(&file, r#"{"a.key.of.another.build":{"kept":true},"limits.checksAtOnce":3}"#).expect("written");
    let fleet = Fleet::assembled(fitted(&home));
    assert_eq!(one_of(&fleet.get_settings().await.expect("served"), "limits.checksAtOnce").value, SettingValue::Integer(3));

    fleet.save_settings(changing("limits.dronesAtOnce", Some(SettingValue::Integer(2)))).await.expect("saved");
    let written = std::fs::read_to_string(&file).expect("read");
    for kept in ["a.key.of.another.build", "limits.checksAtOnce", "limits.dronesAtOnce"] {
        assert!(written.contains(kept), "{kept} is gone: {written}");
    }

    let refused = fleet.save_settings(changing("a.key.of.another.build", Some(SettingValue::Integer(1)))).await.expect_err("not this build's");
    let api::Refusal::Unacceptable(error) = refused else { panic!("{refused:?}") };
    assert!(error.message.contains("a.key.of.another.build"), "{}", error.message);
    assert_eq!(std::fs::read_to_string(&file).expect("read"), written, "nothing was written");
}

/// A file refused at start is the last good copy and not the shipped settings, and a save while it
/// is refused writes nothing, so what the file holds is still there to be corrected.
#[tokio::test]
async fn a_file_refused_at_start_is_the_last_good_copy_and_a_save_leaves_it_alone() {
    let home = TempDir::new();
    let file = settings_file_in(&home);
    std::fs::write(&file, r#"{"limits.checksAtOnce":3}"#).expect("written");
    drop(Fleet::assembled(fitted(&home)));

    let bad = r#"{"limits.checksAtOnce":3,"limits.dronesAtOnce":99}"#;
    std::fs::write(&file, bad).expect("written");
    let fleet = Fleet::assembled(fitted(&home));
    let list = fleet.get_settings().await.expect("served");
    assert!(list.refused.is_some(), "the refusal is served");
    assert_eq!(one_of(&list, "limits.checksAtOnce").saved, Some(SettingValue::Integer(3)), "the last good copy, not what ships");

    let refused = fleet.save_settings(changing("limits.checksAtOnce", Some(SettingValue::Integer(2)))).await.expect_err("the file is refused");
    let api::Refusal::Unacceptable(error) = refused else { panic!("{refused:?}") };
    assert_eq!(error.code, "fleet.unacceptable_settings");
    assert_eq!(std::fs::read_to_string(&file).expect("read"), bad, "nothing was written");
}

/// A value read once at start is saved and waits for a restart, and says so.
#[tokio::test]
async fn an_at_restart_setting_is_saved_and_waits_for_the_next_start() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    let answered = fleet
        .save_settings(changing("timeouts.turnIntervalMs", Some(SettingValue::Integer(500))))
        .await
        .expect("saved");
    let setting = one_of(&answered, "timeouts.turnIntervalMs");
    assert_eq!(setting.saved, Some(SettingValue::Integer(500)));
    assert_eq!(setting.value, SettingValue::Integer(250), "what Fleet started with");
    assert!(setting.pending_restart);
    drop(fleet);

    let fleet = Fleet::assembled(fitted(&home));
    let setting = fleet.get_settings().await.expect("reads");
    let setting = one_of(&setting, "timeouts.turnIntervalMs");
    assert_eq!(setting.value, SettingValue::Integer(500));
    assert!(!setting.pending_restart);
}

fn store_in(home: &TempDir) -> store::Store {
    store::Store::open(&home.path().join("armada.db")).expect("a store")
}

#[tokio::test]
async fn a_first_start_with_nothing_saved_writes_no_file() {
    let home = TempDir::new();
    let path = settings_file_in(&home);
    assert_eq!(migrated(&store_in(&home), &path).expect("nothing"), Vec::<String>::new());
    assert!(!path.exists(), "Fleet does not create the file until something is saved");
}

/// `limits.checksAtOnce` is live all the way down: a save moves Fleet's own
/// places and the machine's Check slots it shares with `armada check`.
#[tokio::test]
async fn saving_checks_at_once_resizes_the_machine_check_slots_without_a_restart() {
    let home = TempDir::new();
    let slots = checks_runner::CheckSlots::at(home.path().join("check-slots"), 4);
    let mut fittings = fitted(&home);
    fittings.check_slots = Some(slots.clone());
    let fleet = Fleet::assembled(fittings);

    fleet
        .save_settings(changing("limits.checksAtOnce", Some(SettingValue::Integer(2))))
        .await
        .expect("saved");
    assert_eq!(slots.count(), 2);
    assert_eq!(fleet.checks_at_once().get(), 2);
}
