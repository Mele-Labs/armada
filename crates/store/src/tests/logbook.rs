//! The Logbook's record: an entry comes back whole, and a forgotten one never comes back, under
//! any of the texts it had.

use crate::tests::{open, TempDir};
use crate::{LogEntry, LogEvidence, LogHistory, LogHow, LogKind, LogRewrite, LogSection, LogState};

const KEY: &str = "acme--shop";

fn entry(text: &str) -> LogEntry {
    LogEntry {
        id: None,
        section: LogSection::Repo,
        repository: KEY.to_string(),
        kind: LogKind::Gotcha,
        text: text.to_string(),
        paths: vec!["crates/**".to_string()],
        evidence: vec![LogEvidence {
            session_id: "s1".to_string(),
            cites: vec!["tool:1".to_string(), "ran:2".to_string()],
            how: LogHow::First,
            hard: true,
            at: 10,
        }],
        first_seen: 10,
        last_confirmed: 10,
        last_served: None,
        state: LogState::Candidate,
        pinned: false,
        edited_by_person: false,
        tripped: 0,
        history: vec![LogHistory { text: "Chrome is in /Applications.".to_string(), replaced_at: 9, why: LogRewrite::Contradicted }],
    }
}

#[test]
fn a_logbook_entry_comes_back_whole() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = store.save_logbook_entry(&entry("Chrome is not in /Applications.")).expect("kept").expect("an id");
    let read = store.logbook_of(KEY).expect("reads");
    assert_eq!(read, vec![LogEntry { id: Some(id), ..entry("Chrome is not in /Applications.") }]);
    assert!(store.logbook_of("other--repo").expect("reads").is_empty());
}

#[test]
fn a_forgotten_logbook_entry_never_comes_back() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = store.save_logbook_entry(&entry("Chrome is not in /Applications.")).expect("kept").expect("an id");
    assert!(store.forget_logbook_entry(id, 20).expect("forgot"));
    assert!(store.logbook_of(KEY).expect("reads").is_empty());
    assert_eq!(store.save_logbook_entry(&entry("chrome is NOT in /applications")).expect("asked"), None);
    assert_eq!(store.save_logbook_entry(&entry("Chrome is in /Applications")).expect("asked"), None, "an earlier text is forgotten too");
    assert!(store.save_logbook_entry(&LogEntry { repository: "other--repo".into(), ..entry("Chrome is not in /Applications.") }).expect("kept").is_some());
}

#[test]
fn a_repositorys_sessions_are_counted_once_each_after_a_time() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.logbook_distilled("s1", KEY, "2026-10-01T00:00:00Z", 100).expect("kept");
    store.logbook_distilled("s1", KEY, "2026-10-02T00:00:00Z", 200).expect("kept");
    store.logbook_distilled("s2", "other--repo", "2026-10-02T00:00:00Z", 200).expect("kept");
    assert_eq!(store.logbook_sessions_since(KEY, 50).expect("counts"), 1);
    assert_eq!(store.logbook_sessions_since("", 50).expect("counts"), 2);
    assert_eq!(store.logbook_sessions_since(KEY, 200).expect("counts"), 0);
    assert_eq!(store.logbook_last_distil("s1").expect("reads").as_deref(), Some("2026-10-02T00:00:00Z"));
}

#[test]
fn a_taught_logbook_entry_stands_at_once_as_the_persons() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = store.save_logbook_entry(&entry("Chrome is not in /Applications.")).expect("kept").expect("an id");
    store.forget_logbook_entry(id, 20).expect("forgot");
    let taught = store
        .teach_logbook_entry(LogSection::Repo, KEY, LogKind::Gotcha, "Chrome is not in /Applications.", &[], 30)
        .expect("taught");
    let read = store.logbook_of(KEY).expect("reads");
    assert_eq!(read.len(), 1);
    let entry = &read[0];
    assert_eq!((entry.id, entry.state, entry.edited_by_person), (Some(taught), LogState::Standing, true));
    assert!(entry.evidence.is_empty());
}
