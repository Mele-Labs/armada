//! The timeline: append-only, windowed by instant, newest kept when it is cut.

use crate::tests::{open, TempDir};
use crate::TimelineEntry;

fn entry(at: &str, kind: &str) -> TimelineEntry {
    TimelineEntry {
        at: at.to_string(),
        kind: kind.to_string(),
        subject_kind: "job".to_string(),
        subject_id: "01JOB".to_string(),
        subject_url: None,
        title: Some("80-debounce".to_string()),
        repo: Some("armada".to_string()),
        tone: "run".to_string(),
        says: "Started".to_string(),
        detail: None,
        items: Vec::new(),
        service: None,
        service_name: None,
        to_id: None,
        live: true,
        over: false,
    }
}

fn kinds(window: &crate::TimelineWindow) -> Vec<&str> {
    window.events.iter().map(|one| one.entry.kind.as_str()).collect()
}

/// An entry comes back as it went in, numbered, and survives a reopen.
#[test]
fn an_entry_is_kept_whole_and_numbered() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut asked = entry("2026-10-12T10:00:00.000Z", "job.asked");
    asked.detail = Some("Plan question".to_string());
    asked.tone = "ask".to_string();
    let first = store.append_timeline(&asked).expect("kept");
    let second = store
        .append_timeline(&entry("2026-10-12T10:01:00.000Z", "job.started"))
        .expect("kept");
    assert!(second > first, "numbered in the order written");
    drop(store);

    let store = open(&dir);
    let window = store.timeline_between(None, None, 10).expect("read");
    assert!(!window.cut);
    assert_eq!(window.events[0].entry, asked);
    assert_eq!(window.events[0].seq, first);
}

/// What a burst counted comes back in the order it went in, and an entry with none comes back with none.
#[test]
fn the_items_a_burst_counted_are_kept_in_order() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut read = entry("2026-10-12T10:00:00.000Z", "session.read");
    read.says = "Read 3 files".to_string();
    read.items = vec!["src/c.rs".to_string(), "src/b.rs".to_string(), "src/a.rs".to_string()];
    read.detail = Some("src/c.rs".to_string());
    store.append_timeline(&read).expect("kept");
    store.append_timeline(&entry("2026-10-12T10:01:00.000Z", "job.started")).expect("kept");
    let window = store.timeline_between(None, None, 10).expect("read");
    assert_eq!(window.events[0].entry, read);
    assert!(window.events[1].entry.items.is_empty());
}

/// `since` is inclusive and `until` is not, so two adjacent windows share no entry.
#[test]
fn a_window_takes_since_and_stops_before_until() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (at, kind) in [
        ("2026-10-12T10:00:00.000Z", "job.created"),
        ("2026-10-12T10:05:00.000Z", "job.started"),
        ("2026-10-12T10:10:00.000Z", "job.landed"),
    ] {
        store.append_timeline(&entry(at, kind)).expect("kept");
    }
    let middle = store
        .timeline_between(Some("2026-10-12T10:05:00.000Z"), Some("2026-10-12T10:10:00.000Z"), 10)
        .expect("read");
    assert_eq!(kinds(&middle), ["job.started"]);
    let from = store
        .timeline_between(Some("2026-10-12T10:05:00.000Z"), None, 10)
        .expect("read");
    assert_eq!(kinds(&from), ["job.started", "job.landed"]);
}

/// Over the limit the newest are kept, oldest first, and the cut is said.
#[test]
fn a_cut_window_keeps_the_newest_and_says_so() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (at, kind) in [
        ("2026-10-12T10:00:00.000Z", "job.created"),
        ("2026-10-12T10:05:00.000Z", "job.started"),
        ("2026-10-12T10:10:00.000Z", "job.landed"),
    ] {
        store.append_timeline(&entry(at, kind)).expect("kept");
    }
    let window = store.timeline_between(None, None, 2).expect("read");
    assert!(window.cut);
    assert_eq!(kinds(&window), ["job.started", "job.landed"]);
    let whole = store.timeline_between(None, None, 3).expect("read");
    assert!(!whole.cut, "exactly the limit is not a cut");
}

/// Ordered by the instant a thing happened, not the order it was written: a
/// Session's start can be written after a later event about it.
#[test]
fn a_late_written_early_event_sorts_by_its_instant() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.append_timeline(&entry("2026-10-12T10:10:00.000Z", "session.asked")).expect("kept");
    store.append_timeline(&entry("2026-10-12T10:00:00.000Z", "session.started")).expect("kept");
    let window = store.timeline_between(None, None, 10).expect("read");
    assert_eq!(kinds(&window), ["session.started", "session.asked"]);
}

/// Thirty days before the entry being written, and not before: the same rule
/// as `check_runs`, so the table stays small without a sweep of its own.
#[test]
fn a_row_older_than_thirty_days_is_pruned_on_the_next_write() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.append_timeline(&entry("2026-09-01T00:00:00.000Z", "job.created")).expect("kept");
    store.append_timeline(&entry("2026-09-12T00:00:01.000Z", "job.started")).expect("kept");
    store.append_timeline(&entry("2026-10-12T00:00:00.000Z", "job.landed")).expect("kept");
    let window = store.timeline_between(None, None, 10).expect("read");
    assert_eq!(kinds(&window), ["job.started", "job.landed"], "30 days before 12 Oct is 12 Sep");
}

/// A number is never given twice, even after the newest row is pruned away.
#[test]
fn a_number_is_not_reused_after_a_prune() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.append_timeline(&entry("2026-10-12T00:00:00.000Z", "job.created")).expect("kept");
    // Written with an instant a year on, so the first is pruned and the second takes the next number.
    let second = store.append_timeline(&entry("2027-10-12T00:00:00.000Z", "job.started")).expect("kept");
    assert!(second > first);
    let window = store.timeline_between(None, None, 10).expect("read");
    assert_eq!(kinds(&window), ["job.started"]);
}

/// The service a call went to is kept with the entry; every other entry has none.
#[test]
fn a_calls_service_is_kept_with_its_entry() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut called = entry("2026-10-12T10:00:00.000Z", "session.called");
    called.detail = Some("gh pr view".to_string());
    called.service = Some("forge".to_string());
    called.service_name = Some("the forge".to_string());
    store.append_timeline(&called).expect("kept");
    store.append_timeline(&entry("2026-10-12T10:01:00.000Z", "session.read")).expect("kept");
    let window = store.timeline_between(None, None, 10).expect("read");
    assert_eq!(window.events[0].entry.service.as_deref(), Some("forge"));
    assert_eq!(window.events[0].entry, called);
    assert_eq!(window.events[1].entry.service, None);
}
