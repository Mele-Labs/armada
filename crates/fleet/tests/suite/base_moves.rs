//! `when_base_moves:` runs coalesced: one at a time per repository, and a move during a run
//! queues exactly one more.

use fleet::base_moves::BaseMoves;

#[test]
fn the_first_tip_seen_starts_a_run_and_hands_it_out_once() {
    let moves = BaseMoves::default();
    moves.moved("a", "c1");
    assert_eq!(moves.due(), ["a"]);
    assert!(moves.due().is_empty());
}

#[test]
fn a_tip_already_seen_starts_nothing() {
    let moves = BaseMoves::default();
    moves.moved("a", "c1");
    moves.due();
    assert!(!moves.ended("a"));
    moves.moved("a", "c1");
    assert!(moves.due().is_empty());
    assert!(moves.seen("a", "c1"));
}

#[test]
fn moves_during_a_run_queue_exactly_one_more() {
    let moves = BaseMoves::default();
    moves.moved("a", "c1");
    moves.due();
    moves.moved("a", "c2");
    moves.moved("a", "c3");
    moves.moved("a", "c4");
    assert!(moves.due().is_empty(), "nothing starts beside the run going");
    assert!(moves.ended("a"), "the moves seen are run once more");
    assert!(!moves.ended("a"), "and only once");
}

#[test]
fn a_move_after_a_run_ended_starts_another() {
    let moves = BaseMoves::default();
    moves.moved("a", "c1");
    moves.due();
    assert!(!moves.ended("a"));
    moves.moved("a", "c2");
    assert_eq!(moves.due(), ["a"]);
}

#[test]
fn each_repository_runs_on_its_own() {
    let moves = BaseMoves::default();
    moves.moved("a", "c1");
    moves.due();
    moves.moved("b", "d1");
    assert_eq!(moves.due(), ["b"], "a run in one repository holds no other back");
    moves.moved("a", "c2");
    assert!(moves.ended("a"));
    assert!(!moves.ended("b"));
}
