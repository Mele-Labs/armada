//! What an Autopilot keeps of the boxes checked at start. A record kept before them reads as
//! backlog on, running off and no decisions, so an Autopilot started earlier behaves as it did.

use rusqlite::params;

use crate::tests::{at, open, TempDir};
use crate::KeptAutopilot;

fn kept(manifest: &str) -> KeptAutopilot {
    KeptAutopilot {
        manifest_id: manifest.to_string(),
        run_id: "01RUN".to_string(),
        instruction: "fix what is open".to_string(),
        at_once: 2,
        until: None,
        spend_cap_micros: None,
        session_cap: None,
        state: "running".to_string(),
        started_at: at("2026-11-02T09:00:00.000Z").as_str().to_string(),
        ended_at: None,
        capped_by: None,
        on_backlog: false,
        on_running: true,
        decisions: vec!["plan".to_string(), "land".to_string()],
    }
}

#[test]
fn the_boxes_checked_at_start_are_kept_and_read_back() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.start_autopilot(&kept("one")).expect("started");
    let read = store.autopilot("one").expect("reads").expect("held");
    assert!(!read.on_backlog && read.on_running);
    assert_eq!(read.decisions, ["plan", "land"]);
}

#[test]
fn a_change_of_settings_leaves_the_boxes_where_they_were() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.start_autopilot(&kept("one")).expect("started");
    let changed = KeptAutopilot { at_once: 5, on_backlog: true, on_running: false, decisions: Vec::new(), ..kept("one") };
    assert!(store.update_autopilot(&changed).expect("changed"));
    let read = store.autopilot("one").expect("reads").expect("held");
    assert_eq!(read.at_once, 5);
    assert!(!read.on_backlog && read.on_running);
    assert_eq!(read.decisions, ["plan", "land"]);
}

#[test]
fn a_record_kept_before_the_boxes_reads_as_backlog_on_running_off_and_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .conn
        .execute(
            "INSERT INTO autopilots (manifest_id, run_id, instruction, at_once, state, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["old", "01OLD", "fix", 1, "running", "2026-10-01T09:00:00.000Z"],
        )
        .expect("a row written as it was");
    let read = store.autopilot("old").expect("reads").expect("held");
    assert!(read.on_backlog && !read.on_running);
    assert!(read.decisions.is_empty());
}
