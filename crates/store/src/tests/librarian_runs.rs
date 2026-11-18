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
