//! The Logbook note a Session starts with: habits first, then the repository, then this Mac, cut to
//! a budget by rank, and nothing in it that is not standing. `docs/concepts/logbook.md`, *Recall*.

use store::{LogKind, LogSection, LogState};

use crate::logbook::noting::{note, HEADER_TOKENS, NOTE_TOKENS};
use crate::logbook::serving::{served, Logged, Source};

fn entry(section: LogSection, text: &str, confirmed: usize) -> Logged {
    let mut one = Logged::folded(Source::Learnt, LogKind::Gotcha, text.to_string(), 10);
    one.entry.section = section;
    one.entry.evidence = (0..confirmed)
        .map(|n| store::LogEvidence {
            session_id: format!("s{n}"),
            cites: Vec::new(),
            how: store::LogHow::Confirmed,
            hard: false,
            at: 1,
        })
        .collect();
    one
}

#[test]
fn the_note_is_habits_then_the_repository_then_this_mac_whatever_the_rank() {
    let all = vec![
        entry(LogSection::Machine, "Chrome is not in /Applications.", 9),
        entry(LogSection::Repo, "Run `armada check test`.", 5),
        entry(LogSection::Habits, "Show a walk before landing.", 1),
    ];

    let text = note(&served(all, NOTE_TOKENS)).expect("a note");

    let habits = text.find("Show a walk").expect("habits");
    let repo = text.find("armada check test").expect("repo");
    let machine = text.find("Chrome").expect("machine");
    assert!(habits < repo && repo < machine, "{text}");
    assert!(text.contains("never a Check"), "{text}");
}

#[test]
fn a_candidate_or_a_faded_entry_is_not_in_the_note() {
    let mut candidate = entry(LogSection::Repo, "Only one Job saw this.", 1);
    candidate.entry.state = LogState::Candidate;
    let mut faded = entry(LogSection::Repo, "Nobody has needed this lately.", 3);
    faded.entry.state = LogState::Faded;

    assert_eq!(note(&served(vec![candidate, faded], NOTE_TOKENS)), None);
}

#[test]
fn the_budget_cuts_the_least_borne_out_entry_and_keeps_the_rest_in_order() {
    let long = "x".repeat(3000);
    let all = vec![
        entry(LogSection::Repo, &format!("least {long}"), 1),
        entry(LogSection::Repo, &format!("most {long}"), 4),
        entry(LogSection::Repo, &format!("middle {long}"), 2),
    ];

    let text = note(&served(all, NOTE_TOKENS - HEADER_TOKENS)).expect("a note");

    assert!(text.contains("most "), "{text:.80}");
    assert!(!text.contains("least "), "the cut fell on the entry fewest Jobs bore out");
    assert!(text.len() / 4 <= NOTE_TOKENS, "about 2k tokens: {}", text.len() / 4);
}
