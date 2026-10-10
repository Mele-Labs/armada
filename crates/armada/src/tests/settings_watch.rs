//! The watch on settings.json: a file that is not there yet is watched all the
//! same, its first save and a rename over it are each noticed once, and a
//! dropped watch is quiet.

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
