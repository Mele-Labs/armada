//! How a repository runs: the merge keeps the newest run, a dropped line never comes back, and a
//! changed place stands every fact as not yet written there.

use crate::tests::{open, TempDir};
use crate::KeptRun;

const REPO: &str = "/work/shop";

fn run(command: &str, last_ran: u64, port: Option<u16>) -> KeptRun {
    KeptRun {
        workspace: ".".to_string(),
        kind: "tests".to_string(),
        command: command.to_string(),
        port,
        succeeded: true,
        last_ran,
        source: "jobs".to_string(),
        kept_in: None,
        why: None,
    }
}

fn key(command: &str) -> (&str, &str, &str) {
    (".", "tests", command)
}

#[test]
fn a_repository_with_no_choice_has_no_place_and_no_runs() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert_eq!(store.run_memory_place(REPO).expect("reads"), None);
    assert!(store.run_facts(REPO).expect("reads").is_empty());
}

#[test]
fn the_newer_run_wins_and_keeps_the_older_ones_port() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.merge_run_facts(REPO, &[run("pnpm dev", 100, Some(3000))]).expect("merged");
    store.merge_run_facts(REPO, &[run("pnpm dev", 200, None)]).expect("merged");
    store.merge_run_facts(REPO, &[run("pnpm dev", 50, Some(9))]).expect("merged");

    let held = store.run_facts(REPO).expect("reads");
    assert_eq!(held.len(), 1);
    assert_eq!((held[0].last_ran, held[0].port), (200, Some(3000)));
}

#[test]
fn a_dropped_line_is_not_kept_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.merge_run_facts(REPO, &[run("cargo test", 100, None), run("cargo clippy", 100, None)]).expect("merged");

    store.drop_run_fact(REPO, key("cargo test"), "2026-10-18T10:00:00.000Z").expect("dropped");
    store.merge_run_facts(REPO, &[run("cargo test", 900, None)]).expect("merged");

    let commands: Vec<_> = store.run_facts(REPO).expect("reads").into_iter().map(|one| one.command).collect();
    assert_eq!(commands, vec!["cargo clippy"]);
    assert_eq!(store.dropped_run_facts(REPO).expect("reads"), vec![(".".into(), "tests".into(), "cargo test".into())]);
}

#[test]
fn where_a_run_is_kept_survives_an_unchanged_merge_and_resets_on_a_new_run_or_place() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.merge_run_facts(REPO, &[run("cargo test", 100, None)]).expect("merged");
    store.stand_run_fact(REPO, key("cargo test"), Some("this_mac"), None).expect("stood");

    store.merge_run_facts(REPO, &[run("cargo test", 100, None)]).expect("merged");
    assert_eq!(store.run_facts(REPO).expect("reads")[0].kept_in.as_deref(), Some("this_mac"));

    store.merge_run_facts(REPO, &[run("cargo test", 200, None)]).expect("merged");
    assert_eq!(store.run_facts(REPO).expect("reads")[0].kept_in, None, "a newer run is not written yet");

    store.stand_run_fact(REPO, key("cargo test"), Some("this_mac"), None).expect("stood");
    store.set_run_memory_place(REPO, "synced", Some("/Users/user/Sync"), None).expect("chosen");
    let after = store.run_facts(REPO).expect("reads");
    assert_eq!((after[0].kept_in.clone(), after[0].why.is_some()), (None, true));
    assert_eq!(
        store.run_memory_place(REPO).expect("reads"),
        Some(crate::KeptPlace { place: "synced".to_string(), synced_folder: Some("/Users/user/Sync".to_string()), cloud: None })
    );
}

#[test]
fn a_cloud_place_keeps_its_address_and_a_later_choice_forgets_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.set_run_memory_place(REPO, "cloud", None, Some("s3://shop-runs/armada")).expect("chosen");
    let kept = store.run_memory_place(REPO).expect("reads").expect("a place");
    assert_eq!((kept.place.as_str(), kept.cloud.as_deref()), ("cloud", Some("s3://shop-runs/armada")));

    store.set_run_memory_place(REPO, "this_mac", None, None).expect("chosen");
    assert_eq!(store.run_memory_place(REPO).expect("reads").expect("a place").cloud, None);
}
