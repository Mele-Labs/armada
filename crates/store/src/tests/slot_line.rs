//! The line for a slot, kept across a restart in each repository's order.

use crate::tests::{open, TempDir};
use crate::KeptInLine;

fn in_line(session: &str, manifest: &str, place: i64) -> KeptInLine {
    KeptInLine { session_id: session.into(), manifest_id: manifest.into(), place, since: format!("2026-11-04T10:00:0{place}.000Z") }
}

#[test]
fn the_line_reads_back_in_each_repositorys_order_and_without_those_that_left() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for one in [in_line("01LATE", "01REPO", 3), in_line("01FIRST", "01REPO", 1), in_line("01OTHER", "01ELSEWHERE", 1), in_line("01GONE", "01REPO", 2)] {
        store.keep_in_slot_line(&one).expect("kept");
    }
    store.leave_slot_line("01GONE").expect("left");
    store.keep_in_slot_line(&in_line("01LATE", "01REPO", 0)).expect("placed again");

    let lines: Vec<(String, String)> = store.slot_lines().expect("read").into_iter().map(|one| (one.manifest_id, one.session_id)).collect();
    assert_eq!(
        lines,
        [("01ELSEWHERE", "01OTHER"), ("01REPO", "01LATE"), ("01REPO", "01FIRST")].map(|(manifest, session)| (manifest.to_string(), session.to_string()))
    );
}
