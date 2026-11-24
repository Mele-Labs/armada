//! The Cockpit home's layout is kept whole across a restart, forgotten where a save carries none, and keeps a block
//! kind this build does not know.

use api::{Refusal, Settings};
use ipc::{BoardBlock, BoardLayout, HomeLayout, SaveHomeLayout};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

fn block(kind: &str, x: u8, view: &str) -> BoardBlock {
    BoardBlock { kind: kind.to_string(), x, y: 0, w: 3, h: 2, view: view.to_string() }
}

fn layout(blocks: Vec<BoardBlock>) -> SaveHomeLayout {
    SaveHomeLayout { layout: Some(BoardLayout { blocks }) }
}

#[tokio::test]
async fn a_home_layout_saved_is_read_back_whole_and_survives_a_restart() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_home_layout().await.expect("reads"), HomeLayout { layout: None }, "none saved reads none");

    let blocks = vec![block("needs", 0, "list"), block("run", 3, "table")];
    let answered = fleet.save_home_layout(layout(blocks.clone())).await.expect("saved");
    assert_eq!(answered.layout, Some(BoardLayout { blocks: blocks.clone() }));
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_home_layout().await.expect("reads").layout, Some(BoardLayout { blocks }));
}

#[tokio::test]
async fn a_save_with_no_layout_forgets_it() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    fleet.save_home_layout(layout(vec![block("needs", 0, "list")])).await.expect("saved");
    assert!(fleet.get_home_layout().await.expect("reads").layout.is_some());

    let answered = fleet.save_home_layout(SaveHomeLayout { layout: None }).await.expect("forgotten");
    assert_eq!(answered, HomeLayout { layout: None });
    assert_eq!(fleet.get_home_layout().await.expect("reads"), HomeLayout { layout: None });
}

#[tokio::test]
async fn a_block_kind_and_view_this_build_does_not_know_survive_a_save() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let blocks = vec![block("sunrise", 0, "constellation"), block("needs", 3, "list")];
    fleet.save_home_layout(layout(blocks.clone())).await.expect("saved");
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.get_home_layout().await.expect("reads").layout.map(|one| one.blocks), Some(blocks));
}

#[tokio::test]
async fn a_blank_kind_or_view_is_refused_and_keeps_what_was_there() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let kept = vec![block("needs", 0, "list")];
    fleet.save_home_layout(layout(kept.clone())).await.expect("saved");

    for bad in [block("  ", 0, "list"), block("run", 0, "")] {
        match fleet.save_home_layout(layout(vec![bad])).await.expect_err("a blank word") {
            Refusal::Unacceptable(error) => assert_eq!(error.code, "fleet.unacceptable_home_layout"),
            other => panic!("not a 422: {other:?}"),
        }
    }
    assert_eq!(fleet.get_home_layout().await.expect("reads").layout.map(|one| one.blocks), Some(kept));
}

#[tokio::test]
async fn saving_the_home_layout_leaves_the_other_preferences_alone() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let theme = ipc::SavePreference { name: "theme".into(), value: false, text: Some("calm".into()) };
    fleet.save_preferences(theme).await.expect("theme saved");
    fleet.save_home_layout(layout(vec![block("needs", 0, "list")])).await.expect("saved");
    assert_eq!(fleet.get_preferences().await.expect("reads").theme, "calm");
}
