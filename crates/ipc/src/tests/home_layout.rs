//! The Cockpit home's layout text: the project board's blocks, kept as written, with the stamp of the save that made them.

use crate::home_layout::{problems, read, text_of, MOST_BLOCKS};
use crate::{BoardBlock, BoardLayout, HomeLayout, Instant};

fn block(kind: &str, view: &str) -> BoardBlock {
    BoardBlock { kind: kind.to_string(), x: 0, y: 1, w: 3, h: 2, view: view.to_string() }
}

fn bare(blocks: Vec<BoardBlock>) -> HomeLayout {
    HomeLayout { layout: Some(BoardLayout { blocks }), saved_at: None }
}

fn at(when: &str, mut home: HomeLayout) -> HomeLayout {
    home.saved_at = Some(Instant::carried(when));
    home
}

#[test]
fn a_layout_reads_back_as_it_was_written_with_kinds_this_build_does_not_know() {
    let home = bare(vec![block("sunrise", "constellation"), block("needs", "list")]);
    let text = text_of(&home).expect("kept");
    assert_eq!(problems(&text), Vec::<String>::new());
    assert_eq!(read(&text), home);
}

#[test]
fn a_stamp_is_kept_with_the_layout_and_with_a_forgotten_one() {
    let stamped = at("2026-11-24T10:00:00.000Z", bare(vec![block("needs", "list")]));
    assert_eq!(read(&text_of(&stamped).expect("kept")), stamped);
    let forgotten = at("2026-11-24T11:00:00.000Z", HomeLayout::default());
    let text = text_of(&forgotten).expect("a forgetting is kept, so it can beat a layout elsewhere");
    assert_eq!(problems(&text), Vec::<String>::new());
    assert_eq!(read(&text), forgotten);
    assert_eq!(text_of(&HomeLayout::default()), None, "no layout and no stamp keeps nothing");
}

#[test]
fn what_is_not_a_layout_is_named_and_reads_as_none() {
    for bad in ["{", "[]", r#"{"blocks":[{"kind":"needs"}]}"#, ""] {
        assert!(!problems(bad).is_empty(), "{bad}");
        assert_eq!(read(bad), HomeLayout::default(), "{bad}");
    }
}

#[test]
fn a_blank_word_or_too_many_blocks_is_a_problem() {
    let text = |blocks| text_of(&at("2026-11-24T10:00:00.000Z", bare(blocks))).expect("kept");
    assert_eq!(problems(&text(vec![block(" ", "list")])).len(), 1);
    assert_eq!(problems(&text(vec![block("needs", "")])).len(), 1);
    assert_eq!(problems(&text((0..=MOST_BLOCKS).map(|_| block("needs", "list")).collect())).len(), 1);
}

#[test]
fn the_later_save_supersedes_and_an_unstamped_layout_loses_to_any_stamp() {
    let early = at("2026-11-24T10:00:00.000Z", bare(vec![block("needs", "list")]));
    let late = at("2026-11-24T10:00:01.000Z", bare(vec![block("run", "table")]));
    assert!(late.supersedes(&early) && !early.supersedes(&late));
    let by_hand = bare(vec![block("needs", "list")]);
    assert!(early.supersedes(&by_hand) && !by_hand.supersedes(&early));
    assert!(!by_hand.supersedes(&by_hand), "two unstamped ones never trade places");
    let forgotten = at("2026-11-24T10:00:02.000Z", HomeLayout::default());
    assert!(forgotten.supersedes(&late), "a later forgetting beats an earlier layout");
}

#[test]
fn two_saves_of_one_instant_are_settled_the_same_way_on_both_macs() {
    let one = at("2026-11-24T10:00:00.000Z", bare(vec![block("needs", "list")]));
    let two = at("2026-11-24T10:00:00.000Z", bare(vec![block("run", "table")]));
    assert_ne!(one.supersedes(&two), two.supersedes(&one));
    assert!(!one.supersedes(&one));
}
