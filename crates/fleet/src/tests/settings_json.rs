//! settings.json's claim, end to end over a real Fleet and a real file: a save
//! from the wire is written and reaches the next admission, a hand edit is
//! taken without a restart and said on the stream, a bad one is refused with
//! the last good settings left in force, a `null` puts the default back, and a
//! first start carries what the store saved into the file.
//!
//! **Here and not in `crates/acceptance`**: the claim is a file on disk and an
//! admission turn, and that crate writes no file and cannot turn a Fleet.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use api::{Queries, Settings};
use config::settings::Supplied;
use core_model::JobId;
use ipc::{Event, SaveSettings, SettingValue, SettingsList};
use store::settings_file;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::{Fittings, Fleet};
use crate::settings::{migrated, Reread};
use crate::slots::Concurrency;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

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
        .words(s::MODELS_JUDGE, "a-model")
        .words(s::MODELS_PROPOSER, "a-model")
        .words(s::MODELS_RETRO, "another-model")
        .words(s::MODELS_SECOND_OPINION, "another-model")
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
        gaming: None,
    }]));
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    fittings.concurrency = Concurrency::of(2);
    fittings
}

async fn approved(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("a proposal");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("a person approves it");
    job.id().clone()
}

fn changing(key: &str, value: Option<SettingValue>) -> SaveSettings {
    SaveSettings { changes: BTreeMap::from([(key.to_string(), value)]) }
}

fn one_of<'a>(list: &'a SettingsList, key: &str) -> &'a ipc::Setting {
    list.settings.iter().find(|one| one.key == key).expect(key)
}

fn hand_edit(home: &TempDir, text: &str) {
    std::fs::write(settings_file_in(home), text).expect("a hand edit");
}

async fn bound(fleet: &Fixture) -> u32 {
    fleet.get_capacity().await.expect("capacity reads").bound
}

/// Claim 1: a save from the wire is in the file, and the next turn admits by it.
#[tokio::test]
async fn saving_three_drones_at_once_writes_the_file_and_the_next_turn_admits_three() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    for title in ["the first", "the second", "the third"] {
        approved(&fleet, &home, title).await;
    }
    assert_eq!(fleet.working_on().await.len(), 2, "two ship");

    let answered = fleet
        .save_settings(changing("limits.dronesAtOnce", Some(SettingValue::Integer(3))))
        .await
        .expect("saved");
    assert_eq!(one_of(&answered, "limits.dronesAtOnce").saved, Some(SettingValue::Integer(3)));
    let written = settings_file::read(&settings_file_in(&home)).expect("reads").expect("written");
    assert_eq!(written.get("limits.dronesAtOnce"), Some(&settings_file::SettingValue::Integer(3)));
    assert_eq!(written.len(), 1, "only what a person changed is written");

    fleet.turn().await.expect("the loop turns");
    assert_eq!(fleet.working_on().await.len(), 3);
}

/// Claim 2: a hand edit is read without a restart and said on the stream.
#[tokio::test]
async fn a_hand_edit_changes_what_is_served_without_a_restart_and_says_so() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    let mut events = fleet.events().subscribe();

    hand_edit(&home, "{\n  \"limits.dronesAtOnce\": 4,\n  \"timeouts.checkSeconds\": 600\n}\n");
    assert!(matches!(fleet.settings_reread().await, Reread::Taken), "taken");

    let served = fleet.get_settings().await.expect("reads");
    assert_eq!(one_of(&served, "limits.dronesAtOnce").value, SettingValue::Integer(4));
    assert_eq!(one_of(&served, "timeouts.checkSeconds").saved, Some(SettingValue::Integer(600)));
    assert_eq!(bound(&fleet).await, 4, "in force, not only served");
    let heard = tokio::time::timeout(Duration::from_secs(5), events.next())
        .await
        .expect("an event")
        .expect("open");
    let api::Next::Send(delivered) = heard else { panic!("{heard:?}") };
    let Event::SettingsChanged(list) = delivered.event else { panic!("{:?}", delivered.event) };
    assert_eq!(one_of(&list, "limits.dronesAtOnce").value, SettingValue::Integer(4));

    assert!(matches!(fleet.settings_reread().await, Reread::Quiet), "the same file again is quiet");
}

/// Claim 3: an unknown key or a value out of range refuses the whole file, the
/// refusal is served, and the last good values stay in force.
#[tokio::test]
async fn a_bad_hand_edit_is_refused_whole_and_the_last_good_settings_stay() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    fleet
        .save_settings(changing("limits.dronesAtOnce", Some(SettingValue::Integer(3))))
        .await
        .expect("saved");

    hand_edit(&home, r#"{"limits.dronesAtOnce": 5, "limits.dronesAtOnse": 5}"#);
    let Reread::Refused(refused) = fleet.settings_reread().await else { panic!("refused") };
    assert_eq!(refused.key.as_deref(), Some("limits.dronesAtOnse"));
    let served = fleet.get_settings().await.expect("reads");
    let shown = served.refused.as_ref().expect("the refusal is served");
    assert_eq!(shown.key.as_deref(), Some("limits.dronesAtOnse"));
    assert_eq!(one_of(&served, "limits.dronesAtOnce").value, SettingValue::Integer(3));
    assert_eq!(bound(&fleet).await, 3, "the previous value is still in force");

    hand_edit(&home, r#"{"limits.dronesAtOnce": 12}"#);
    let Reread::Refused(refused) = fleet.settings_reread().await else { panic!("refused") };
    assert_eq!(refused.key.as_deref(), Some("limits.dronesAtOnce"));
    assert!(refused.reason.contains("from 1 to 8"), "{}", refused.reason);

    hand_edit(&home, "{ not json");
    let Reread::Refused(refused) = fleet.settings_reread().await else { panic!("refused") };
    assert_eq!(refused.key, None, "no key to blame in a file that is not JSON");
    assert_eq!(bound(&fleet).await, 3);

    hand_edit(&home, r#"{"limits.dronesAtOnce": 2}"#);
    assert!(matches!(fleet.settings_reread().await, Reread::Taken));
    assert_eq!(fleet.get_settings().await.expect("reads").refused, None, "a good read clears it");
}

/// Claim 4: `null` removes the key from the file, and the default is back.
#[tokio::test]
async fn null_removes_the_key_and_the_shipped_default_is_back_in_force() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    fleet
        .save_settings(changing("limits.dronesAtOnce", Some(SettingValue::Integer(5))))
        .await
        .expect("saved");
    fleet
        .save_settings(changing("bridge.theme", Some(SettingValue::Text("calm".into()))))
        .await
        .expect("saved");
    assert_eq!(bound(&fleet).await, 5);

    let answered = fleet.save_settings(changing("limits.dronesAtOnce", None)).await.expect("saved");
    let setting = one_of(&answered, "limits.dronesAtOnce");
    assert_eq!(setting.saved, None);
    assert_eq!(setting.value, setting.default);
    assert_eq!(bound(&fleet).await, 2, "what ships");
    let written = settings_file::read(&settings_file_in(&home)).expect("reads").expect("written");
    assert!(!written.contains_key("limits.dronesAtOnce"));
    assert!(written.contains_key("bridge.theme"), "a key the save did not name stays");
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

fn written(path: &Path) -> settings_file::SettingsDocument {
    settings_file::read(path).expect("reads").expect("written")
}

/// Claim 5: a first start with saved rows and no file writes them into it, and
/// never again once the file is there.
#[tokio::test]
async fn a_first_start_carries_the_stores_saved_rows_into_settings_json() {
    let home = TempDir::new();
    let path = settings_file_in(&home);
    {
        let mut store = store_in(&home);
        store
            .save_limits(&store::SavedLimits { concurrency: Some(3), disk_floor_gib: Some(20), ..Default::default() })
            .expect("saved");
        store.save_preference("draft_pull_requests", true).expect("saved");
        store.save_theme("calm").expect("saved");
        store.save_layout_choices(r#"{"version":1}"#).expect("saved");
    }

    let moved = migrated(&store_in(&home), &path).expect("carried");
    assert_eq!(
        moved,
        vec![
            "bridge.layout".to_string(),
            "bridge.theme".to_string(),
            "features.draftPullRequests".to_string(),
            "limits.diskFloorGib".to_string(),
            "limits.dronesAtOnce".to_string(),
        ]
    );
    let file = written(&path);
    assert_eq!(file.get("limits.dronesAtOnce"), Some(&settings_file::SettingValue::Integer(3)));
    assert_eq!(file.get("features.draftPullRequests"), Some(&settings_file::SettingValue::Bool(true)));
    assert_eq!(file.get("bridge.theme"), Some(&settings_file::SettingValue::Text("calm".into())));
    assert!(matches!(file.get("bridge.layout"), Some(settings_file::SettingValue::Map(_))));

    let fleet = Fleet::assembled(fitted(&home));
    assert_eq!(bound(&fleet).await, 3, "the carried value is in force");
    fleet.save_settings(changing("limits.dronesAtOnce", None)).await.expect("saved");
    drop(fleet);
    assert_eq!(migrated(&store_in(&home), &path).expect("nothing to do"), Vec::<String>::new());
    assert!(!written(&path).contains_key("limits.dronesAtOnce"), "the store is never read again");
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
