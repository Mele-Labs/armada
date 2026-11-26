//! The grid's geometry on a kind of block that is not the Dashboard's, which is what the Cockpit home will be.

use ipc::{BoardBlock, BoardLayout};

use super::{GridKind, Layout, Placed, View, COLUMNS, LEAST_H, LEAST_W, MOST_H};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Bay {
    Console,
    Needs,
    Feed,
}

impl GridKind for Bay {
    fn all() -> &'static [Bay] {
        &[Bay::Console, Bay::Needs, Bay::Feed]
    }

    fn slug(self) -> String {
        match self {
            Bay::Console => "console",
            Bay::Needs => "needs",
            Bay::Feed => "feed",
        }
        .to_string()
    }

    fn from_slug(slug: &str) -> Option<Bay> {
        Bay::all().iter().copied().find(|bay| bay.slug() == slug)
    }

    fn views(self) -> &'static [View] {
        match self {
            Bay::Console => &[View::Graph],
            Bay::Needs => &[View::List, View::Cards],
            Bay::Feed => &[View::Table, View::List],
        }
    }

    fn title(self) -> &'static str {
        match self {
            Bay::Console => "Console",
            Bay::Needs => "Needs you",
            Bay::Feed => "Feed",
        }
    }
}

fn block(kind: &str, x: u8, y: u8, w: u8, h: u8, view: &str) -> BoardBlock {
    BoardBlock { kind: kind.into(), x, y, w, h, view: view.into() }
}

fn two_up() -> Layout<Bay> {
    Layout { blocks: vec![Placed::new(Bay::Needs, 0, 0, 3, 2, View::List), Placed::new(Bay::Feed, 3, 0, 3, 2, View::Table)] }
}

#[test]
fn a_second_kind_goes_to_fleet_and_comes_back_as_it_stood() {
    let kept = BoardLayout { blocks: vec![block("feed", 3, 0, 3, 2, "table"), block("needs", 0, 0, 3, 2, "cards")] };
    let (layout, unknown) = Layout::<Bay>::from_wire(&kept);
    assert!(unknown.is_empty());
    assert_eq!(layout.block(Bay::Needs).map(|one| one.view), Some(View::Cards));
    let sent = layout.to_wire(&unknown);
    assert_eq!(sent.blocks.iter().map(|one| one.kind.as_str()).collect::<Vec<_>>(), ["feed", "needs"], "the blocks in the order they were kept");
}

#[test]
fn a_block_of_a_kind_this_build_has_no_word_for_goes_back_as_it_came() {
    let kept = BoardLayout { blocks: vec![block("needs", 0, 0, 3, 2, "list"), block("timeline", 3, 0, 3, 2, "graph"), block("needs", 3, 2, 3, 2, "list")] };
    let (layout, unknown) = Layout::<Bay>::from_wire(&kept);
    assert_eq!(layout.blocks.len(), 1, "a kind that stands twice keeps its first");
    assert_eq!(unknown, vec![block("timeline", 3, 0, 3, 2, "graph")]);
    let sent = layout.to_wire(&unknown);
    assert_eq!(sent.blocks[1], unknown[0], "kept as it was written");
}

#[test]
fn a_kept_block_outside_the_grid_or_in_a_view_its_kind_does_not_draw_is_brought_inside() {
    let kept = BoardLayout { blocks: vec![block("needs", 5, 0, 4, 9, "graph")] };
    let (layout, _) = Layout::<Bay>::from_wire(&kept);
    assert_eq!(layout.blocks, vec![Placed::new(Bay::Needs, 2, 0, 4, MOST_H, View::List)]);
}

#[test]
fn a_hidden_block_lets_the_rest_rise_and_an_added_one_takes_the_first_free_cells() {
    let mut layout = two_up();
    assert_eq!(layout.missing(), [Bay::Console]);
    layout.add(Bay::Console);
    assert!(layout.missing().is_empty());
    let console = layout.block(Bay::Console).copied().expect("the console is on");
    assert_eq!((console.x, console.y, console.view), (0, 2, View::Graph), "under the two, in the kind's first view");
    layout.hide(Bay::Needs);
    assert_eq!(layout.missing(), [Bay::Needs]);
    assert_eq!(layout.block(Bay::Console).map(|one| one.y), Some(0), "the console rises into the room the block left");
}

#[test]
fn a_move_pins_the_block_and_the_others_settle_round_it() {
    let mut layout = two_up();
    layout.move_to(Bay::Feed, 0, 0);
    let feed = layout.block(Bay::Feed).copied().unwrap();
    let needs = layout.block(Bay::Needs).copied().unwrap();
    assert_eq!((feed.x, feed.y), (0, 0), "the moved block keeps its place");
    assert!(!feed.overlaps(&needs) && needs.y >= feed.y + feed.h, "the other gives way");
    layout.move_to(Bay::Feed, 40, -3);
    let feed = layout.block(Bay::Feed).copied().unwrap();
    assert_eq!((feed.x, feed.y), (COLUMNS - feed.w, 0), "held inside the grid");
}

#[test]
fn a_resize_is_held_between_the_least_and_the_most_and_inside_the_grid() {
    let mut layout = two_up();
    layout.resize_to(Bay::Feed, 9, 99);
    let feed = layout.block(Bay::Feed).copied().unwrap();
    assert_eq!((feed.w, feed.h), (COLUMNS - feed.x, MOST_H));
    layout.grow(Bay::Feed, -9, -9);
    let feed = layout.block(Bay::Feed).copied().unwrap();
    assert_eq!((feed.w, feed.h), (LEAST_W, LEAST_H));
}

#[test]
fn a_nudge_moves_by_whole_cells_and_a_view_its_kind_does_not_draw_is_refused() {
    let mut layout = two_up();
    layout.nudge(Bay::Needs, 0, 1);
    assert_eq!(layout.block(Bay::Needs).map(|one| one.y), Some(1));
    layout.set_view(Bay::Needs, View::Graph);
    assert_eq!(layout.block(Bay::Needs).map(|one| one.view), Some(View::List));
    layout.set_view(Bay::Needs, View::Cards);
    assert_eq!(layout.block(Bay::Needs).map(|one| one.view), Some(View::Cards));
}

#[test]
fn blocks_appended_below_stand_under_everything_side_by_side() {
    let mut layout = two_up();
    layout.append_below(&[Bay::Console, Bay::Needs]);
    assert_eq!(layout.blocks.len(), 3, "a block already on the grid is not added twice");
    assert_eq!(layout.block(Bay::Console).map(|one| (one.x, one.y)), Some((0, 2)));
}
