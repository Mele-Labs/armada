//! The models the harness offers, through a real Fleet: what it says is served with its
//! facts and a switch each, a roster someone saved switches the rest off, and a harness
//! that cannot answer leaves the built-in names with the reason said.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use api::{Queries, Refusal, Settings};
use ipc::{ModelsRead, OfferedModel, SaveSettings, SettingValue};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::models_read::ModelReader;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

/// A reader that says what it was last told to.
struct Planted(Mutex<Result<Vec<OfferedModel>, String>>);

impl Planted {
    fn saying(answer: Result<Vec<OfferedModel>, String>) -> Arc<Planted> {
        Arc::new(Planted(Mutex::new(answer)))
    }
    fn now_saying(&self, answer: Result<Vec<OfferedModel>, String>) {
        *self.0.lock().expect("not poisoned") = answer;
    }
}

impl ModelReader for Planted {
    fn read(&self) -> Result<Vec<OfferedModel>, String> {
        self.0.lock().expect("not poisoned").clone()
    }
}

fn offered(name: &str, display: &str) -> OfferedModel {
    OfferedModel {
        name: name.to_string(),
        display: display.to_string(),
        version: format!("{name}-9"),
        description: format!("{display}, for a test"),
        efforts: vec!["low".to_string(), "high".to_string()],
        on: true,
    }
}

fn fleet_in(home: &TempDir) -> Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> {
    Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[])))
}

fn saving(changes: &[(&str, SettingValue)]) -> SaveSettings {
    SaveSettings {
        changes: BTreeMap::from_iter(changes.iter().map(|(key, value)| (key.to_string(), Some(value.clone())))),
    }
}

fn names(of: &[&str]) -> SettingValue {
    SettingValue::List(of.iter().map(|one| SettingValue::Text(one.to_string())).collect())
}

fn word(of: &str) -> SettingValue {
    SettingValue::Text(of.to_string())
}

/// Every model the harness lists is served, with its own facts, and with nothing saved they are all on.
#[tokio::test]
async fn the_models_the_harness_lists_are_served_with_their_facts_all_on_when_no_roster_is_saved() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    fleet.reading_models_with(Planted::saying(Ok(vec![
        offered("a-model", "A model 9"),
        offered("another-model", "Another model 9"),
        offered("brand-new", "Brand new 1"),
    ])));

    let served = fleet.list_models(true).await.expect("served");

    assert_eq!(served.read, ModelsRead::Read);
    let rows: Vec<(&str, bool)> = served.offered.iter().map(|row| (row.name.as_str(), row.on)).collect();
    assert_eq!(rows, [("a-model", true), ("another-model", true), ("brand-new", true)]);
    assert_eq!(served.offered[0].display, "A model 9");
    assert_eq!(served.offered[0].version, "a-model-9");
    assert_eq!(served.offered[0].efforts, ["low", "high"]);
    assert_eq!(served.models, ["a-model", "another-model", "brand-new"], "a model nobody wrote down is on");
}

/// A save that leaves a setting naming a model it switches off is refused naming that setting; one
/// that moves them together is taken, and the switched-off model is off for everything that asks.
#[tokio::test]
async fn a_model_switched_off_is_served_off_and_a_setting_naming_it_refuses_the_save() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    fleet.reading_models_with(Planted::saying(Ok(vec![offered("a-model", "A"), offered("another-model", "B")])));
    fleet.list_models(true).await.expect("served");

    let refused = fleet
        .save_settings(saving(&[("models.roster", names(&["another-model"]))]))
        .await
        .expect_err("models.default still names a-model");
    let Refusal::Unacceptable(error) = refused else { panic!("{refused:?}") };
    assert_eq!(error.code, "fleet.unacceptable_settings");
    assert!(error.message.contains("a-model") && error.message.contains("models.roster"), "{}", error.message);
    assert_eq!(fleet.list_models(false).await.expect("served").models, ["a-model", "another-model"], "nothing moved");

    let moved: Vec<(&str, SettingValue)> = ["models.default", "models.session", "models.review", "models.judge", "models.proposer", "models.account"]
        .into_iter()
        .map(|key| (key, word("another-model")))
        .chain([("models.roster", names(&["another-model"]))])
        .collect();
    fleet.save_settings(saving(&moved)).await.expect("moved together");

    let served = fleet.list_models(false).await.expect("served");
    let rows: Vec<(&str, bool)> = served.offered.iter().map(|row| (row.name.as_str(), row.on)).collect();
    assert_eq!(rows, [("a-model", false), ("another-model", true)]);
    assert_eq!(served.models, ["another-model"]);
    let roster = config::Roster::of(fleet.models_on());
    let off = core_model::ModelName::new("a-model").expect("a name");
    assert!(!roster.offers(&off), "a definition naming it is refused as the roster refuses");
}

/// With no answer the built-in names are served bare and the reason is said, then a fresh ask that
/// is answered replaces them.
#[tokio::test]
async fn no_answer_leaves_the_built_in_names_with_the_reason_until_a_fresh_ask_is_answered() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);

    let before = fleet.list_models(false).await.expect("served");
    assert!(matches!(before.read, ModelsRead::NotRead { .. }), "nothing has asked");

    let cannot = Planted::saying(Err("`the-agent` did not answer within 15 seconds".to_string()));
    fleet.reading_models_with(Arc::clone(&cannot) as Arc<dyn ModelReader>);
    let served = fleet.list_models(true).await.expect("served");
    assert_eq!(served.read, ModelsRead::NotRead { why: "`the-agent` did not answer within 15 seconds".to_string() });
    let rows: Vec<(&str, bool, &str)> = served.offered.iter().map(|row| (row.name.as_str(), row.on, row.display.as_str())).collect();
    assert_eq!(rows, [("a-model", true, ""), ("another-model", true, "")], "bare, and on");
    assert_eq!(served.models, ["a-model", "another-model"]);

    cannot.now_saying(Ok(vec![offered("a-model", "A")]));
    assert!(matches!(fleet.list_models(false).await.expect("served").read, ModelsRead::NotRead { .. }), "only a fresh ask reads again");
    let served = fleet.list_models(true).await.expect("served");
    assert_eq!(served.read, ModelsRead::Read);
    assert_eq!(served.models, ["a-model"]);
}
