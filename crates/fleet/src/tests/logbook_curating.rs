//! Curate's rules, with no store and no model: an entry stands on two Sessions or one Session's
//! hard evidence, a contradiction rewrites it and keeps what it said, a trip marks it and orders it
//! first, it fades only when both time and Sessions have passed, a lock holds it, and a forgotten
//! text never comes back. `docs/concepts/logbook.md`.

use std::collections::BTreeSet;

use store::{logbook_words, LogEntry, LogEvidence, LogHow, LogKind, LogRewrite, LogSection, LogState};

use crate::logbook::answer::{Claim, Distilled, Learnt, Titled};
use crate::logbook::curating::{curate, Distil, Held, FADES_AFTER_MS, FADES_AFTER_SESSIONS};
use crate::logbook::serving::{served, Logged, Source};

const KEY: &str = "acme--shop";
const DAY: u64 = 24 * 60 * 60 * 1000;

#[derive(Default)]
struct Kept {
    forgotten: Vec<String>,
    sessions: usize,
}

impl Held for Kept {
    fn denied(&self, _: LogSection, _: &str, text: &str) -> bool {
        self.forgotten.iter().any(|one| logbook_words(one) == logbook_words(text))
    }
    fn sessions_since(&self, _: LogSection, _: u64) -> usize {
        self.sessions
    }
}

fn learnt(text: &str, hard: bool) -> Learnt {
    Learnt {
        section: LogSection::Repo,
        kind: LogKind::Gotcha,
        title: None,
        text: text.to_string(),
        paths: Vec::new(),
        cites: vec!["tool:1".to_string()],
        hard,
    }
}

fn claim(id: i64, text: Option<&str>) -> Claim {
    Claim { id, text: text.map(str::to_string), title: None, cites: vec!["tool:2".to_string()], hard: false }
}

fn distil(session: &str, now: u64) -> Distil<'_> {
    Distil { session, repository: KEY, now }
}

fn standing(id: i64, text: &str, at: u64) -> LogEntry {
    LogEntry {
        id: Some(id),
        section: LogSection::Repo,
        repository: KEY.to_string(),
        kind: LogKind::Gotcha,
        title: None,
        text: text.to_string(),
        paths: Vec::new(),
        evidence: ["s1", "s2"]
            .iter()
            .map(|session| LogEvidence { session_id: session.to_string(), cites: vec!["tool:1".into()], how: LogHow::First, hard: false, at })
            .collect(),
        first_seen: at,
        last_confirmed: at,
        last_served: None,
        state: LogState::Standing,
        locked: false,
        edited_by_person: false,
        tripped: 0,
        history: Vec::new(),
    }
}

fn learning(entries: &mut Vec<LogEntry>, answer: Distilled, session: &str, now: u64, held: &Kept) -> BTreeSet<usize> {
    curate(entries, &answer, &distil(session, now), held)
}

#[test]
fn a_logbook_candidate_stands_at_two_sessions_and_not_one() {
    let mut entries = Vec::new();
    let held = Kept::default();
    let said = || Distilled { learnt: vec![learnt("Chrome is not in /Applications.", false)], ..Distilled::default() };
    learning(&mut entries, said(), "s1", 10, &held);
    assert_eq!(entries[0].state, LogState::Candidate);
    learning(&mut entries, said(), "s1", 20, &held);
    assert_eq!((entries.len(), entries[0].state), (1, LogState::Candidate), "the same Session twice is one");
    learning(&mut entries, said(), "s2", 30, &held);
    assert_eq!((entries.len(), entries[0].state), (1, LogState::Standing));
}

#[test]
fn a_logbook_candidate_stands_on_one_sessions_hard_evidence() {
    let mut entries = Vec::new();
    let answer = Distilled { learnt: vec![learnt("Use `pnpm -F desktop test`; `pnpm test` fails at the root.", true)], ..Distilled::default() };
    let changed = learning(&mut entries, answer, "s1", 10, &Kept::default());
    assert_eq!(changed, BTreeSet::from([0]));
    assert_eq!(entries[0].state, LogState::Standing);
}

#[test]
fn a_logbook_contradiction_rewrites_and_keeps_the_history() {
    let mut entries = vec![LogEntry { edited_by_person: true, ..standing(1, "Chrome is in /Applications.", 10) }];
    let answer = Distilled { contradicted: vec![claim(1, Some("Chrome is not in /Applications."))], ..Distilled::default() };
    learning(&mut entries, answer, "s3", 20, &Kept::default());
    let entry = &entries[0];
    assert_eq!(entry.text, "Chrome is not in /Applications.");
    assert_eq!(entry.history.len(), 1);
    assert_eq!((entry.history[0].text.as_str(), entry.history[0].why), ("Chrome is in /Applications.", LogRewrite::Contradicted));
    assert!(!entry.edited_by_person, "a Session contradicted the person's text");
    assert_eq!(entry.confirmations(), 2, "a contradiction bears nothing out");
}

#[test]
fn a_logbook_trip_marks_rewrites_and_orders_first() {
    let mut entries = vec![standing(1, "Lease the slot before writing.", 10), standing(2, "Run `cargo check -p fleet`.", 10)];
    entries[1].evidence.push(LogEvidence { session_id: "s9".into(), cites: Vec::new(), how: LogHow::Confirmed, hard: false, at: 10 });
    let answer = Distilled { tripped: vec![claim(1, Some("Lease a slot with `armada worktree lease` before the first write."))], ..Distilled::default() };
    learning(&mut entries, answer, "s3", 20, &Kept::default());
    assert_eq!(entries[0].tripped, 1);
    assert_eq!(entries[0].text, "Lease a slot with `armada worktree lease` before the first write.");
    assert_eq!(entries[0].history[0].why, LogRewrite::Tripped);
    let all = entries.iter().cloned().map(|entry| Logged { entry, from: Source::Learnt }).collect();
    let order: Vec<Option<i64>> = served(all, usize::MAX).iter().map(|one| one.entry.id).collect();
    assert_eq!(order, vec![Some(1), Some(2)], "tripped orders ahead of more confirmations");
}

#[test]
fn a_logbook_trip_keeps_a_persons_text() {
    let mut entries = vec![LogEntry { edited_by_person: true, ..standing(1, "Lease the slot before writing.", 10) }];
    let answer = Distilled { tripped: vec![claim(1, Some("Something else."))], ..Distilled::default() };
    learning(&mut entries, answer, "s3", 20, &Kept::default());
    assert_eq!((entries[0].text.as_str(), entries[0].tripped), ("Lease the slot before writing.", 1));
}

#[test]
fn a_locked_logbook_entry_is_never_rewritten_and_never_fades() {
    let mut entries = vec![LogEntry { locked: true, ..standing(1, "Chrome is in /Applications.", 0) }];
    let answer = Distilled { contradicted: vec![claim(1, Some("Chrome is not in /Applications."))], ..Distilled::default() };
    let held = Kept { sessions: 100, ..Kept::default() };
    learning(&mut entries, answer, "s3", 400 * DAY, &held);
    assert_eq!((entries[0].text.as_str(), entries[0].state), ("Chrome is in /Applications.", LogState::Standing));
}

#[test]
fn a_logbook_entry_fades_only_after_both_the_days_and_the_sessions() {
    let now = FADES_AFTER_MS + 10;
    for (sessions, served, fades) in [
        (FADES_AFTER_SESSIONS, None, true),
        (FADES_AFTER_SESSIONS - 1, None, false),
        (FADES_AFTER_SESSIONS, Some(now - DAY), false),
    ] {
        let mut entries = vec![LogEntry { last_served: served, ..standing(1, "Run the desktop tests in a worktree.", 10) }];
        learning(&mut entries, Distilled::default(), "s9", now, &Kept { sessions, ..Kept::default() });
        assert_eq!(entries[0].state == LogState::Faded, fades, "{sessions} sessions, served {served:?}");
    }
    let mut early = vec![standing(1, "Run the desktop tests in a worktree.", 10)];
    learning(&mut early, Distilled::default(), "s9", 29 * DAY, &Kept { sessions: 50, ..Kept::default() });
    assert_eq!(early[0].state, LogState::Standing, "fifty Sessions in 29 days is not enough time");
}

#[test]
fn a_forgotten_logbook_text_is_never_learnt_again() {
    let mut entries = Vec::new();
    let held = Kept { forgotten: vec!["Chrome is not in /Applications".to_string()], ..Kept::default() };
    let answer = Distilled { learnt: vec![learnt("chrome is NOT in /applications.", true)], ..Distilled::default() };
    assert!(learning(&mut entries, answer, "s1", 10, &held).is_empty());
    assert!(entries.is_empty());
}

#[test]
fn a_new_logbook_entry_keeps_its_title_and_one_learnt_again_gains_a_missing_one() {
    let mut entries = Vec::new();
    let held = Kept::default();
    let titled = || Learnt { title: Some("Chrome is not in /Applications".into()), ..learnt("Chrome is not in /Applications.", false) };
    learning(&mut entries, Distilled { learnt: vec![learnt("Chrome is not in /Applications.", false)], ..Distilled::default() }, "s1", 10, &held);
    assert_eq!(entries[0].title, None);
    learning(&mut entries, Distilled { learnt: vec![titled()], ..Distilled::default() }, "s2", 20, &held);
    assert_eq!((entries.len(), entries[0].title.as_deref()), (1, Some("Chrome is not in /Applications")), "learnt again with a title");
    learning(&mut entries, Distilled { learnt: vec![Learnt { title: Some("Another name".into()), ..titled() }], ..Distilled::default() }, "s3", 30, &held);
    assert_eq!(entries[0].title.as_deref(), Some("Chrome is not in /Applications"), "a title it has is not replaced");
}

#[test]
fn a_logbook_contradiction_retitles_with_the_text_and_a_locked_entry_keeps_both() {
    let mut entries = vec![
        LogEntry { title: Some("Chrome location".into()), ..standing(1, "Chrome is in /Applications.", 10) },
        LogEntry { title: Some("Chrome location".into()), locked: true, ..standing(2, "Chrome is in /Applications.", 10) },
        LogEntry { title: Some("Chrome location".into()), ..standing(3, "Chrome is in /Applications.", 10) },
    ];
    let said = |id| Claim { title: Some("Chrome is missing".into()), ..claim(id, Some("Chrome is not in /Applications.")) };
    let untitled = Claim { title: None, ..claim(3, Some("Chrome is not in /Applications.")) };
    let answer = Distilled { contradicted: vec![said(1), said(2), untitled], ..Distilled::default() };
    learning(&mut entries, answer, "s3", 20, &Kept::default());
    let titles: Vec<&str> = entries.iter().map(|one| one.title.as_deref().expect("titled")).collect();
    assert_eq!(titles, ["Chrome is missing", "Chrome location", "Chrome location"], "locked keeps its title; no new title keeps the old one");
}

#[test]
fn a_title_is_given_to_an_untitled_logbook_entry_and_never_replaces_one() {
    let mut entries = vec![standing(1, "Lease the slot before writing.", 10), LogEntry { title: Some("First write waits".into()), ..standing(2, "Run `cargo check -p fleet`.", 10) }];
    let answer = Distilled {
        titled: vec![Titled { id: 1, title: "First write waits for the slot".into() }, Titled { id: 2, title: "Replaced".into() }, Titled { id: 7, title: "No such entry".into() }],
        ..Distilled::default()
    };
    let changed = learning(&mut entries, answer, "s3", 20, &Kept::default());
    assert_eq!(changed, BTreeSet::from([0]), "only the untitled entry changed");
    assert_eq!(entries[0].title.as_deref(), Some("First write waits for the slot"));
    assert_eq!(entries[1].title.as_deref(), Some("First write waits"));
}
