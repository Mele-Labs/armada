//! The watch on settings.json: a file that is not there yet is watched all the
//! same, its first save and a rename over it are each noticed once, and a
//! dropped watch is quiet. And the defaults only the composition root can supply.

use std::time::Duration;

use tokio::sync::mpsc;

use crate::settings::watch_every;
use crate::tests::TempDir;

const QUICK_POLL: Duration = Duration::from_millis(20);
const QUICK_SETTLE: Duration = Duration::from_millis(100);
const PATIENCE: Duration = Duration::from_secs(10);

#[tokio::test(flavor = "multi_thread")]
async fn a_file_made_after_the_watch_and_a_rename_over_it_are_each_noticed_once() {
    let machine = TempDir::new();
    let file = machine.path().join("settings.json");
    let (told, mut changes) = mpsc::unbounded_channel();
    let watching = watch_every(file.clone(), QUICK_POLL, QUICK_SETTLE, move || {
        let told = told.clone();
        async move {
            let _ = told.send(());
        }
    });

    machine.write("settings.json", "{\"limits.dronesAtOnce\": 3}\n");
    tokio::time::timeout(PATIENCE, changes.recv()).await.expect("noticed").expect("open");

    machine.write("settings.json.new", "{\"limits.dronesAtOnce\": 4}\n");
    std::fs::rename(machine.path().join("settings.json.new"), &file).expect("the rename over it");
    tokio::time::timeout(PATIENCE, changes.recv()).await.expect("noticed").expect("open");
    assert!(
        tokio::time::timeout(QUICK_SETTLE * 4, changes.recv()).await.is_err(),
        "one change is told once"
    );

    drop(watching);
    machine.write("settings.json", "{}\n");
    assert!(
        tokio::time::timeout(QUICK_SETTLE * 4, changes.recv()).await.map_or(true, |got| got.is_none()),
        "a dropped watch says nothing"
    );
}

/// A key whose default only this process knows ships as nothing if the composition root forgets it,
/// and a model of nothing is a step down that is never offered.
#[test]
fn the_composition_root_supplies_every_default_it_alone_can_know() {
    assert_eq!(crate::settings::supplied().missing(), Vec::<&str>::new());
}

/// The owner's own machine: a Session ships on a different model from the step down's target and
/// from the Drone's, so the offer is made and moving it never moves what Drones run on.
#[test]
fn a_machine_that_changed_nothing_has_a_session_model_the_step_down_can_lower() {
    use config::settings as keys;
    let shipped = config::settings::Resolved::new(
        config::settings::Saved::nothing(),
        crate::settings::supplied(),
        config::settings::Env::read(|_| None),
    );
    assert_eq!(shipped.get(keys::MODELS_SESSION), adapters::HeadlessAgent::session_model());
    assert_ne!(shipped.get(keys::MODELS_SESSION), shipped.get(keys::PLAN_DOWNSHIFT_TO));
    assert_ne!(shipped.get(keys::MODELS_SESSION), shipped.get(keys::MODELS_DEFAULT));
    assert!(shipped.models().contains(&shipped.get(keys::MODELS_SESSION)));
}
