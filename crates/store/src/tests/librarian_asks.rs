//! His asks of the librarian count their model calls a project and a night, and the count outlives a reopen.

use crate::tests::{open, TempDir};

#[test]
fn ask_calls_count_per_project_and_night_and_outlive_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.librarian_ask_calls("p1", "2026-11-24").unwrap(), 0);
    assert_eq!(store.spend_librarian_ask_call("p1", "2026-11-24").unwrap(), 1);
    assert_eq!(store.spend_librarian_ask_call("p1", "2026-11-24").unwrap(), 2);
    store.spend_librarian_ask_call("p1", "2026-11-25").unwrap();
    store.spend_librarian_ask_call("p2", "2026-11-24").unwrap();
    drop(store);

    let store = open(&dir);
    assert_eq!(store.librarian_ask_calls("p1", "2026-11-24").unwrap(), 2, "kept across a reopen");
    assert_eq!(store.librarian_ask_calls("p1", "2026-11-25").unwrap(), 1, "another night counts its own");
    assert_eq!(store.librarian_ask_calls("p2", "2026-11-24").unwrap(), 1, "another project counts its own");
    assert_eq!(store.librarian_model_calls("p1", "2026-11-24").unwrap(), 0, "apart from Catch up's calls");
}
