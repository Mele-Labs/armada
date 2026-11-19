//! A librarian run survives a reopen, and the latest of a project's runs is the one read back.

use crate::tests::{open, TempDir};
use crate::{librarian_run_by, KeptLibrarianRun};

fn run(project: &str, at: &str, through: u64, parts: &[&str]) -> KeptLibrarianRun {
    KeptLibrarianRun {
        project: project.to_string(),
        ran_at: at.to_string(),
        by: librarian_run_by::PRESS.to_string(),
        through_pr: through,
        through_at: if through == 0 { String::new() } else { "2026-11-18T08:00:00.000Z".to_string() },
        at_commit: "abc123".to_string(),
        parts: parts.iter().map(|one| one.to_string()).collect(),
        paragraph: String::new(),
        night: "2026-11-18".to_string(),
        model_calls: 0,
        skipped: Vec::new(),
    }
}

#[test]
fn a_run_is_kept_across_a_reopen_and_the_latest_of_a_project_reads_back_with_its_parts_in_order() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.librarian_last_run("p1").unwrap(), None, "a project never run has no record");
    store.keep_librarian_run(&run("p1", "2026-11-18T09:00:00.000Z", 3860, &[])).unwrap();
    store.keep_librarian_run(&run("p1", "2026-11-18T10:00:00.000Z", 3866, &["links", "floor_plans"])).unwrap();
    store.keep_librarian_run(&run("p2", "2026-11-18T11:00:00.000Z", 0, &[])).unwrap();
    drop(store);

    let store = open(&dir);
    let last = store.librarian_last_run("p1").unwrap().expect("kept");
    assert_eq!((last.ran_at.as_str(), last.through_pr, last.at_commit.as_str()), ("2026-11-18T10:00:00.000Z", 3866, "abc123"));
    assert_eq!(last.parts, ["links", "floor_plans"]);
    assert_eq!(store.librarian_run_count("p1").unwrap(), 2);
    assert_eq!(store.librarian_last_run("p2").unwrap().expect("kept").through_pr, 0, "no merge to read through is a zero");
}

#[test]
fn a_nights_calls_the_newest_paragraph_and_the_last_run_of_a_part_read_back_per_project() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut first = run("p1", "2026-11-18T09:00:00.000Z", 3860, &["since_you_looked"]);
    first.paragraph = "Two features changed.".to_string();
    first.model_calls = 1;
    let mut second = run("p1", "2026-11-18T10:00:00.000Z", 3866, &[]);
    second.model_calls = 2;
    second.skipped = vec!["since_you_looked".to_string()];
    let mut next_night = run("p1", "2026-11-19T10:00:00.000Z", 3867, &[]);
    next_night.night = "2026-11-19".to_string();
    next_night.model_calls = 4;
    for kept in [&first, &second, &next_night, &run("p2", "2026-11-18T11:00:00.000Z", 0, &[])] {
        store.keep_librarian_run(kept).unwrap();
    }
    drop(store);

    let store = open(&dir);
    assert_eq!(store.librarian_model_calls("p1", "2026-11-18").unwrap(), 3, "a night sums its runs");
    assert_eq!(store.librarian_model_calls("p1", "2026-11-19").unwrap(), 4);
    assert_eq!(store.librarian_model_calls("p2", "2026-11-18").unwrap(), 0, "another project spends its own");
    assert_eq!(store.librarian_last_paragraph("p1").unwrap(), Some(("2026-11-18T09:00:00.000Z".to_string(), "Two features changed.".to_string())));
    assert_eq!(store.librarian_last_paragraph("p2").unwrap(), None);
    assert_eq!(store.librarian_last_ran_part("p1", "since_you_looked").unwrap().as_deref(), Some("2026-11-18T09:00:00.000Z"));
    assert_eq!(store.librarian_last_ran_part("p1", "links").unwrap(), None);
    assert_eq!(store.librarian_last_run("p1").unwrap().expect("kept").skipped, Vec::<String>::new(), "the newest run skipped none");
}
