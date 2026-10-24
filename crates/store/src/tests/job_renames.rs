//! The Jobs a person renamed: one row a Job that keeps the name Armada gave it, read back this
//! repository's first and newest first, and kept across a reopen.

use crate::tests::{open, TempDir};
use crate::KeptRename;

fn renamed(session: &str, repository: &str, was: &str, title: &str, at: &str) -> KeptRename {
    KeptRename {
        session_id: session.to_string(),
        repository: repository.to_string(),
        request: format!("the request of {session}"),
        was: was.to_string(),
        title: title.to_string(),
        renamed_at: at.to_string(),
    }
}

#[test]
fn a_second_rename_of_one_job_keeps_the_name_armada_gave_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_rename(&renamed("s1", "armada", "Fix slow phone load", "Pocket slow", "2026-10-24T01:00:00Z")).expect("kept");
    store.keep_rename(&renamed("s1", "armada", "Pocket slow", "Armada pocket slow load", "2026-10-24T02:00:00Z")).expect("kept");

    let kept = store.renames_for("armada", 12).expect("reads");
    assert_eq!(kept.len(), 1, "one row a Job");
    assert_eq!(kept[0].was, "Fix slow phone load", "the name Armada gave it, not his own earlier one");
    assert_eq!(kept[0].title, "Armada pocket slow load");
    assert_eq!(kept[0].renamed_at, "2026-10-24T02:00:00Z");
}

#[test]
fn renames_read_this_repositorys_first_then_newest_first_up_to_the_most() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_rename(&renamed("a1", "armada", "w", "A1", "2026-10-24T01:00:00Z")).expect("kept");
    store.keep_rename(&renamed("o1", "other", "w", "O1", "2026-10-24T05:00:00Z")).expect("kept");
    store.keep_rename(&renamed("a2", "armada", "w", "A2", "2026-10-24T03:00:00Z")).expect("kept");
    store.keep_rename(&renamed("o2", "other", "w", "O2", "2026-10-24T04:00:00Z")).expect("kept");

    let titles = |kept: Vec<KeptRename>| kept.into_iter().map(|one| one.title).collect::<Vec<_>>();
    assert_eq!(titles(store.renames_for("armada", 12).expect("reads")), ["A2", "A1", "O1", "O2"]);
    assert_eq!(titles(store.renames_for("other", 3).expect("reads")), ["O1", "O2", "A2"]);
    assert_eq!(titles(store.renames_for("nowhere", 2).expect("reads")), ["O1", "O2"], "a repository with none reads everyone's");
}

#[test]
fn a_rename_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_rename(&renamed("s1", "armada", "w", "Kept", "2026-10-24T01:00:00Z")).expect("kept");
    drop(store);
    let store = open(&dir);
    assert_eq!(store.renames_for("armada", 12).expect("reads").len(), 1);
}
