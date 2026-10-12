//! A read of the whole board folds every Job's log only when the file has
//! changed since the last one.

use rusqlite::Connection;

use crate::tests::{created_at, open, top_level, top_level_numbered, TempDir};

#[test]
fn a_second_read_of_an_unchanged_board_is_not_folded_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01MEMOONE"), &created_at())
        .expect("a job is stored");

    let first = store.load_all_jobs().expect("the board reads");
    let second = store.load_all_jobs().expect("the board reads again");

    assert_eq!(store.folds, 1, "the second read came from the memo");
    assert_eq!(first.jobs, second.jobs, "and it is the same board");
}

#[test]
fn a_write_by_this_connection_is_in_the_next_read() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01MEMOONE"), &created_at())
        .expect("a job is stored");
    assert_eq!(store.load_all_jobs().expect("reads").jobs.len(), 1);

    store
        .insert_job(&top_level_numbered(&store, "01MEMOTWO"), &created_at())
        .expect("a second job is stored");

    assert_eq!(store.load_all_jobs().expect("reads").jobs.len(), 2);
    assert_eq!(store.folds, 2);
}

#[test]
fn a_write_by_another_connection_is_in_the_next_read() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01MEMOONE"), &created_at())
        .expect("a job is stored");
    let before = store.load_all_jobs().expect("reads");
    assert_eq!(before.jobs[0].title().as_str(), "fix the off-by-one in the log reader");

    // `armada clean` is its own process with its own connection to the file.
    Connection::open(dir.db())
        .expect("a second connection")
        .execute("UPDATE jobs SET title = 'renamed elsewhere'", ())
        .expect("a write from outside");

    let after = store.load_all_jobs().expect("reads");
    assert_eq!(after.jobs[0].title().as_str(), "renamed elsewhere");
    assert_eq!(store.folds, 2);
}
