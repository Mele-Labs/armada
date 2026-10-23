//! Recall: the paths a prompt names, the globs that match them, what a subagent is handed for
//! them, and a search that ranks by words held and then by Sessions that bore an entry out.
//! `docs/concepts/logbook.md`, *Recall*.

use store::{LogEvidence, LogHow, LogKind, LogSection, LogState};

use crate::logbook::noting::{subagent_note, SUBAGENT_HEADER_TOKENS};
use crate::logbook::recalling::{for_subagent, matches, named_paths, searched, SUBAGENT_TOKENS};
use crate::logbook::serving::{Logged, Source};

use super::session_host::rig_placing;

fn entry(section: LogSection, text: &str, paths: &[&str], confirmed: usize) -> Logged {
    let mut one = Logged::folded(Source::Learnt, LogKind::Gotcha, text.to_string(), 10);
    one.entry.section = section;
    one.entry.paths = paths.iter().map(|path| path.to_string()).collect();
    one.entry.evidence = (0..confirmed)
        .map(|n| LogEvidence { session_id: format!("s{n}"), cites: Vec::new(), how: LogHow::Confirmed, hard: false, at: 1 })
        .collect();
    one
}

fn texts(all: &[Logged]) -> Vec<&str> {
    all.iter().map(|one| one.entry.text.as_str()).collect()
}

#[test]
fn a_prompt_names_the_paths_it_has_a_slash_in_cut_of_the_root_and_the_noise_round_them() {
    let prompt = "Read `apps/desktop/src/main.ts:42`, (crates/fleet/src/logbook.rs), see https://example.com/a/b and \
                  /Users/user/armada/.armada/slots/slot-3/docs/concepts/logbook.md. Then /repo/root/crates/api/. Not a path: cargo.";

    assert_eq!(
        named_paths(prompt, "/repo/root"),
        ["apps/desktop/src/main.ts", "crates/fleet/src/logbook.rs", "docs/concepts/logbook.md", "crates/api/"]
    );
}

#[test]
fn a_glob_matches_a_name_a_folder_run_or_the_folder_it_is_under() {
    assert!(matches("apps/desktop/**", "apps/desktop/src/main.ts"));
    assert!(matches("apps/desktop/**", "apps/desktop"));
    assert!(matches("**/*.rs", "crates/fleet/src/lib.rs"));
    assert!(matches("**/*.rs", "lib.rs"));
    assert!(matches("crates/*/Cargo.toml", "crates/fleet/Cargo.toml"));
    assert!(matches("crates/fleet/src/log?ook.rs", "crates/fleet/src/logbook.rs"));
    assert!(!matches("crates/*/Cargo.toml", "crates/fleet/src/Cargo.toml"));
    assert!(!matches("apps/desktop/**", "apps/mobile/src/main.ts"));
    assert!(!matches("*.rs", "crates/fleet/src/lib.rs"));
}

#[test]
fn a_subagent_is_handed_the_habits_and_the_entries_for_the_paths_it_names() {
    let all = vec![
        entry(LogSection::Habits, "No AI-sounding copy.", &[], 2),
        entry(LogSection::Repo, "Bridge tests run with vitest.", &["apps/desktop/**"], 2),
        entry(LogSection::Repo, "Fleet tests run with armada check test.", &["crates/**"], 5),
        entry(LogSection::Repo, "Commit with a message that says why.", &[], 9),
        entry(LogSection::Machine, "Chrome is not in /Applications.", &[], 9),
    ];

    let handed = for_subagent(all, &["apps/desktop/src/main.ts".to_string()], SUBAGENT_TOKENS);

    assert_eq!(texts(&handed), ["No AI-sounding copy.", "Bridge tests run with vitest."]);
    let note = subagent_note(&handed).expect("a note");
    assert!(note.find("How the owner works").expect("habits") < note.find("vitest").expect("repo"), "{note}");
}

#[test]
fn a_subagent_that_names_no_path_is_handed_the_habits_and_the_best_of_the_repository_within_its_budget() {
    let long = "x".repeat(2000);
    let all = vec![
        entry(LogSection::Habits, "Walks before landing.", &[], 1),
        entry(LogSection::Repo, &format!("least {long}"), &["a/**"], 1),
        entry(LogSection::Repo, &format!("most {long}"), &[], 6),
    ];

    let handed = for_subagent(all, &[], SUBAGENT_TOKENS - SUBAGENT_HEADER_TOKENS);

    let said = texts(&handed);
    assert!(said.contains(&"Walks before landing."), "{said:?}");
    assert!(said.iter().any(|text| text.starts_with("most")), "{said:?}");
    assert!(!said.iter().any(|text| text.starts_with("least")), "the budget cut the weakest: {said:?}");
}

#[test]
fn a_candidate_or_a_faded_entry_is_not_handed_to_a_subagent() {
    let mut faded = entry(LogSection::Habits, "Faded.", &[], 3);
    faded.entry.state = LogState::Faded;
    let mut candidate = entry(LogSection::Repo, "Candidate.", &["a/**"], 1);
    candidate.entry.state = LogState::Candidate;

    assert!(for_subagent(vec![faded, candidate], &["a/b.rs".to_string()], SUBAGENT_TOKENS).is_empty());
}

#[test]
fn a_search_ranks_by_words_held_then_by_sessions_that_bore_the_entry_out() {
    let mut faded = entry(LogSection::Repo, "Chrome is gone; install chrome again.", &[], 1);
    faded.entry.state = LogState::Faded;
    let all = vec![
        entry(LogSection::Repo, "chrome only", &[], 9),
        entry(LogSection::Repo, "Install Chrome from the cache", &[], 2),
        entry(LogSection::Machine, "chrome install is blocked here", &[], 1),
        entry(LogSection::Habits, "Walks before landing.", &[], 4),
        faded,
    ];

    let found = searched(all.clone(), "chrome INSTALL", None);

    // Two words held: the 2-Session, the 1-Session machine line and the faded one tie on words,
    // and are ordered by Sessions; the one-word entry comes last for all its nine.
    assert_eq!(
        texts(&found),
        [
            "Install Chrome from the cache",
            "chrome install is blocked here",
            "Chrome is gone; install chrome again.",
            "chrome only",
        ]
    );
    assert_eq!(found[2].entry.state, LogState::Faded, "faded entries are searched");

    let machine = searched(all, "chrome", Some(LogSection::Machine));
    assert_eq!(texts(&machine), ["chrome install is blocked here"]);
}

#[tokio::test]
async fn recall_answers_the_whole_logbook_and_refuses_a_blank_query_or_an_unknown_section() {
    let rig = rig_placing(None);
    let mut habit = entry(LogSection::Habits, "Show a walk before landing.", &[], 2).entry;
    habit.state = LogState::Faded;
    rig.fleet.store().lock().await.save_logbook_entry(&habit).expect("kept");

    let found = rig.fleet.logbook_recalled(None, "WALK", Some("habits")).await.expect("answers");
    assert_eq!((found.entries.len(), found.entries[0].state.as_str(), found.entries[0].section.as_str()), (1, "faded", "habits"));

    assert!(rig.fleet.logbook_recalled(None, "  ", None).await.is_err());
    assert!(rig.fleet.logbook_recalled(None, "walk", Some("repository")).await.is_err());

    let handed = rig.fleet.logbook_for_subagent("no-such-session", "work in apps/desktop/src").await;
    assert_eq!(handed, "", "a faded habit is not handed over");

    rig.fleet.store().lock().await.save_logbook_entry(&entry(LogSection::Habits, "No AI-sounding copy.", &[], 2).entry).expect("kept");
    let handed = rig.fleet.logbook_for_subagent("no-such-session", "work in apps/desktop/src").await;
    assert!(handed.contains("- No AI-sounding copy."), "a Session Fleet does not know is handed the habits: {handed}");
}
