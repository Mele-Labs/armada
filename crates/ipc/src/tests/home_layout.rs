//! The Cockpit home's layout text: the project board's blocks, kept as written.

use crate::home_layout::{problems, read, text, MOST_BLOCKS};
use crate::{BoardBlock, BoardLayout};

fn block(kind: &str, view: &str) -> BoardBlock {
    BoardBlock { kind: kind.to_string(), x: 0, y: 1, w: 3, h: 2, view: view.to_string() }
}

#[test]
fn a_layout_reads_back_as_it_was_written_with_kinds_this_build_does_not_know() {
    let layout = BoardLayout { blocks: vec![block("sunrise", "constellation"), block("needs", "list")] };
    assert_eq!(problems(&text(&layout)), Vec::<String>::new());
    assert_eq!(read(&text(&layout)), Some(layout));
}

#[test]
fn what_is_not_a_layout_is_named_and_reads_as_none() {
    for bad in ["{", "[]", r#"{"blocks":[{"kind":"needs"}]}"#, ""] {
        assert!(!problems(bad).is_empty(), "{bad}");
        assert_eq!(read(bad), None, "{bad}");
    }
}

#[test]
fn a_blank_word_or_too_many_blocks_is_a_problem() {
    assert_eq!(problems(&text(&BoardLayout { blocks: vec![block(" ", "list")] })).len(), 1);
    assert_eq!(problems(&text(&BoardLayout { blocks: vec![block("needs", "")] })).len(), 1);
    let many = BoardLayout { blocks: (0..=MOST_BLOCKS).map(|_| block("needs", "list")).collect() };
    assert_eq!(problems(&text(&many)).len(), 1);
}
