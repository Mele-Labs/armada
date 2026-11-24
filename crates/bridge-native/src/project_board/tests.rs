//! The Project place's board and tabs, on the mock Fleet: what is read back from it, what is sent to it, and the controls that
//! change the board.

use gpui::{div, prelude::*, AnyView, Bounds, Context, Entity, Modifiers, Pixels, Role, TestAppContext, VisualTestContext, Window};
use ipc::{BoardBlock, BoardLayout, Instant, ManifestConfig, MetricAnswer, MetricPoint, MetricReading, MetricRow, MetricState, MetricValue};

use super::board::{Board, SAVE_AFTER};
use super::layout::{BlockKind, Layout, Placed, View};
use super::metric::{self, Plot};
use super::model::{self, Press};
use super::presets;
use super::{ProjectScreen, Tab};
use crate::features::ask_box::EXAMPLE_ASKS;

pub(super) const PROJECT: &str = crate::features::mock::PROJECT;

pub(super) fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
    });
}

/// The window's outermost element, which clears the book of what was drawn each frame, as the app's own root does.
struct Rooted(AnyView);

impl Render for Rooted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::labels::label_root(div().size_full().child(self.0.clone()))
    }
}

pub(super) fn redraw(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
}

pub(super) fn find(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<Bounds<Pixels>> {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|one| one.bounds))
}

pub(super) fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    find(cx, role, name).is_some()
}

pub(super) fn press(cx: &mut VisualTestContext, role: Role, name: &str) {
    let at = find(cx, role, name).unwrap_or_else(|| panic!("{name} is drawn")).center();
    cx.simulate_click(at, Modifiers::default());
    redraw(cx);
}

/// A board on the mock Fleet, which keeps `kept` for the project, read and drawn.
pub(super) fn board_keeping(cx: &mut TestAppContext, kept: Option<BoardLayout>) -> (Entity<Board>, &mut VisualTestContext) {
    ready(cx);
    let mut held = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let board = cx.new(|cx| {
            let mut board = Board::mock(None, window, cx);
            if let Some(kept) = kept {
                board.kept.insert(PROJECT.into(), kept);
            }
            board.fill_mock(cx);
            board
        });
        held = Some(board.clone());
        Rooted(board.into())
    });
    // Tall enough that the blocks the mock's recipes put at the bottom are on screen to be pressed.
    cx.simulate_resize(gpui::size(gpui::px(1280.), gpui::px(2600.)));
    redraw(cx);
    (held.expect("the board was made"), cx)
}

pub(super) fn layout_of(board: &Entity<Board>, cx: &mut VisualTestContext) -> Layout {
    board.update(cx, |board, _| board.layout(PROJECT).clone())
}

/// The layout without the metric blocks the mock's recipes put at the bottom, for what is asked of the fixed kinds alone.
fn fixed(layout: Layout) -> Layout {
    Layout { blocks: layout.blocks.into_iter().filter(|one| one.kind.metric_id().is_none()).collect() }
}

fn kept_of(board: &Entity<Board>, cx: &mut VisualTestContext) -> Option<BoardLayout> {
    board.read_with(cx, |board, _| board.kept.get(PROJECT).cloned())
}

fn block(kind: &str, x: u8, y: u8, w: u8, h: u8, view: &str) -> BoardBlock {
    BoardBlock { kind: kind.into(), x, y, w, h, view: view.into() }
}

pub(super) fn screen(cx: &mut TestAppContext) -> (Entity<ProjectScreen>, &mut VisualTestContext) {
    ready(cx);
    let mut held = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let screen = cx.new(|cx| ProjectScreen::mock(None, window, cx));
        held = Some(screen.clone());
        Rooted(screen.into())
    });
    let screen = held.expect("the screen was made");
    // The input tells of its focus only while its window is the active one.
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(gpui::size(gpui::px(1280.), gpui::px(2600.)));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    redraw(cx);
    (screen, cx)
}

// ---- which project --------------------------------------------------------------------------------------------------------

#[gpui::test]
fn the_project_follows_the_title_bars_pick_and_the_header_draws_no_project_chips(cx: &mut TestAppContext) {
    ready(cx);
    let bar = {
        let (bar, _) = cx.add_window_view(|window, cx| crate::titlebar::TitleBar::new(window, cx));
        bar
    };
    let named = |id: &str, name: &str| crate::titlebar::Project { id: id.to_string().into(), name: name.to_string().into(), members: Vec::new(), tracker: None, kept: None, look: None };
    bar.update(cx, |bar, cx| {
        bar.face.projects = vec![named(PROJECT, "Armada"), named("pocket", "Pocket")];
        bar.face.scoped = Some(1);
        cx.notify();
    });
    let (screen, cx) = screen(cx);
    let shown = |screen: &Entity<ProjectScreen>, cx: &mut VisualTestContext| {
        screen.read_with(cx, |screen, cx| (screen.board.read(cx).project(cx).map(|one| one.id), screen.features.read(cx).project_shown(cx)))
    };
    assert_eq!(shown(&screen, cx), (Some("pocket".to_string()), Some("pocket".to_string())), "the bar is on the second project: the board and the Features tabs show it");
    for name in ["Armada", "Pocket", "Store"] {
        assert!(!drawn(cx, Role::Button, name), "the header draws no chip for {name}: the title bar picks the project");
    }
    bar.update(cx, |bar, cx| {
        bar.face.scoped = Some(0);
        cx.notify();
    });
    redraw(cx);
    assert_eq!(shown(&screen, cx), (Some(PROJECT.to_string()), Some(PROJECT.to_string())), "a pick in the bar moves both halves");
    bar.update(cx, |bar, cx| {
        bar.face.scoped = None;
        cx.notify();
    });
    redraw(cx);
    assert_eq!(shown(&screen, cx), (Some(PROJECT.to_string()), Some(PROJECT.to_string())), "on All the project he chose last stays");
}

// ---- the layout Fleet keeps ----------------------------------------------------------------------------------------------

#[gpui::test]
fn the_layout_fleet_kept_is_what_the_board_opens_on(cx: &mut TestAppContext) {
    let kept = BoardLayout { blocks: vec![block("checks", 0, 0, 6, 2, "table"), block("needs-you", 0, 2, 3, 2, "cards")] };
    let (board, cx) = board_keeping(cx, Some(kept));
    let layout = fixed(layout_of(&board, cx));
    assert_eq!(layout.blocks, vec![Placed::new(BlockKind::Checks, 0, 0, 6, 2, View::Table), Placed::new(BlockKind::Needs, 0, 2, 3, 2, View::Cards)], "Fleet's blocks, kinds spelled as it keeps them");
    assert!(drawn(cx, Role::Region, "Checks and commands") && drawn(cx, Role::Region, "Needs you"));
    assert!(!drawn(cx, Role::Region, "Run"), "a block the layout does not hold is not drawn");
}

#[gpui::test]
fn a_project_that_kept_no_layout_opens_on_the_preset(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    let kind = board.read_with(cx, |board, _| board.kind());
    assert_eq!(fixed(layout_of(&board, cx)), presets::preset(kind));
    assert!(drawn(cx, Role::Region, "Run") && drawn(cx, Role::Region, "Issues"));
}

#[test]
fn a_block_of_a_kind_this_build_does_not_know_goes_back_to_fleet_as_it_came() {
    let kept = BoardLayout { blocks: vec![block("run", 0, 0, 3, 2, "list"), block("timeline", 3, 0, 3, 2, "graph"), block("run", 3, 2, 3, 2, "list")] };
    let (layout, unknown) = Layout::from_wire(&kept);
    assert_eq!(layout.blocks.len(), 1, "a kind that stands twice keeps its first");
    assert_eq!(unknown, vec![block("timeline", 3, 0, 3, 2, "graph")]);
    let sent = layout.to_wire(&unknown);
    assert_eq!(sent.blocks.iter().map(|one| one.kind.as_str()).collect::<Vec<_>>(), ["run", "timeline"]);
    assert_eq!(sent.blocks[1], unknown[0], "kept as it was written");
}

#[test]
fn a_kept_block_outside_the_grid_or_of_a_view_its_kind_does_not_draw_is_brought_inside() {
    let kept = BoardLayout { blocks: vec![block("run", 5, 0, 4, 9, "graph")] };
    let (layout, _) = Layout::from_wire(&kept);
    assert_eq!(layout.blocks, vec![Placed::new(BlockKind::Run, 2, 0, 4, 6, View::List)]);
}

// ---- what the board sends ------------------------------------------------------------------------------------------------

#[gpui::test]
fn a_move_is_kept_in_fleet_once_the_board_has_stood_still(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    assert_eq!(kept_of(&board, cx), None, "nothing is kept before he changes the board");
    for _ in 0..3 {
        board.update(cx, |board, cx| board.edit_layout(PROJECT, |layout| layout.nudge(BlockKind::Features, 1, 0), cx));
        cx.executor().advance_clock(SAVE_AFTER / 3);
    }
    cx.run_until_parked();
    assert_eq!(kept_of(&board, cx), None, "a change still being made is not sent");
    cx.executor().advance_clock(SAVE_AFTER);
    cx.run_until_parked();
    let moved = layout_of(&board, cx);
    assert_eq!(kept_of(&board, cx), Some(moved.to_wire(&[])), "the board as it stands, once");
    assert_eq!(moved.block(BlockKind::Features).map(|one| one.x), Some(3), "three steps of the arrow, and the grid holds it where it fits");
}

#[gpui::test]
fn a_view_switch_is_kept_in_fleet(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    press(cx, Role::Button, "Table view of Reviews");
    cx.executor().advance_clock(SAVE_AFTER);
    cx.run_until_parked();
    let kept = kept_of(&board, cx).expect("the switch is kept");
    let reviews = kept.blocks.iter().find(|one| one.kind == "reviews").expect("Reviews is on the board");
    assert_eq!(reviews.view, "table");
}

#[gpui::test]
fn resetting_the_board_has_fleet_forget_the_layout_it_kept(cx: &mut TestAppContext) {
    let kept = BoardLayout { blocks: vec![block("checks", 0, 0, 6, 2, "table")] };
    let (board, cx) = board_keeping(cx, Some(kept));
    board.update(cx, |board, cx| board.reset_layout(cx));
    cx.executor().advance_clock(SAVE_AFTER);
    cx.run_until_parked();
    assert_eq!(kept_of(&board, cx), None, "the preset is not a layout to keep");
    let kind = board.read_with(cx, |board, _| board.kind());
    assert_eq!(fixed(layout_of(&board, cx)), presets::preset(kind));
}

// ---- Start's checkouts ---------------------------------------------------------------------------------------------------

#[gpui::test]
fn the_start_menu_lists_main_and_the_branch_of_each_job_that_holds_a_worktree(cx: &mut TestAppContext) {
    let (_screen, cx) = screen(cx);
    press(cx, Role::Button, "Checkout, Main");
    assert!(drawn(cx, Role::Menu, "Checkouts"));
    assert!(drawn(cx, Role::SearchInput, "Find a branch"), "the menu opens with a search field");
    for name in ["Main", "armada/slot-gc-fix", "armada/pocket-offline"] {
        assert!(drawn(cx, Role::MenuItemCheckBox, name), "{name} is a checkout Start can run on");
    }
    press(cx, Role::MenuItemCheckBox, "armada/slot-gc-fix");
    assert!(drawn(cx, Role::Button, "Checkout, armada/slot-gc-fix"), "the pick is the checkout Start runs on");
    assert!(!drawn(cx, Role::Menu, "Checkouts"));
}

#[gpui::test]
fn typing_in_the_checkout_menu_narrows_it_to_matching_branches_and_enter_picks_the_first(cx: &mut TestAppContext) {
    let (_screen, cx) = screen(cx);
    press(cx, Role::Button, "Checkout, Main");
    cx.simulate_keystrokes("p o c k e t");
    redraw(cx);
    assert!(drawn(cx, Role::MenuItemCheckBox, "armada/pocket-offline"), "the branch that matches stays");
    for name in ["Main", "armada/slot-gc-fix"] {
        assert!(!drawn(cx, Role::MenuItemCheckBox, name), "{name} does not match what was typed");
    }
    cx.simulate_keystrokes("enter");
    redraw(cx);
    assert!(drawn(cx, Role::Button, "Checkout, armada/pocket-offline"), "Enter takes the first match");
    assert!(!drawn(cx, Role::Menu, "Checkouts"));
}

// ---- Commands that ask ---------------------------------------------------------------------------------------------------

fn command(name: &str) -> Press {
    let item = super::mock::checks().into_iter().find(|item| item.title == name).unwrap_or_else(|| panic!("the mock has {name}"));
    item.buttons[0].clone().expect("a Run button").press
}

#[gpui::test]
fn a_command_flagged_confirm_asks_before_it_runs_and_runs_on_run(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    board.update(cx, |board, cx| board.press(command("restart"), cx));
    redraw(cx);
    assert!(drawn(cx, Role::Dialog, "Run restart?"), "the question is on screen");
    assert!(board.read_with(cx, |board, _| board.ran.is_empty()), "nothing has run");
    press(cx, Role::Button, "Run");
    assert_eq!(board.read_with(cx, |board, _| board.ran.clone()), [("armada-main".to_string(), "restart".to_string())]);
    assert!(!drawn(cx, Role::Dialog, "Run restart?"));
}

#[gpui::test]
fn cancel_and_escape_leave_a_flagged_command_unrun(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    board.update(cx, |board, cx| board.press(command("restart"), cx));
    redraw(cx);
    press(cx, Role::Button, "Cancel");
    assert!(board.read_with(cx, |board, _| board.ran.is_empty() && board.asking.is_none()));
    board.update(cx, |board, cx| board.press(command("restart"), cx));
    redraw(cx);
    cx.simulate_keystrokes("escape");
    redraw(cx);
    assert!(board.read_with(cx, |board, _| board.ran.is_empty() && board.asking.is_none()), "Esc is Cancel");
}

#[gpui::test]
fn a_command_that_is_not_flagged_runs_on_the_press(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    board.update(cx, |board, cx| board.press(command("preview"), cx));
    redraw(cx);
    assert_eq!(board.read_with(cx, |board, _| board.ran.clone()), [("armada-main".to_string(), "preview".to_string())]);
    assert!(!drawn(cx, Role::Dialog, "Run preview?"));
}

#[test]
fn the_manifests_confirm_list_marks_the_commands_that_ask() {
    let config: ManifestConfig = ipc::decode(
        "ManifestConfig",
        br#"{"id":"armada-main","summary":{"id":"armada-main","repository":"armada","path":"/r/armada.yml","records_root":"/rec","version":1,"checks":["test"]},"checks_as_written":["test"],"proved_after_a_merge":[],"commands":["preview","restart"],"confirm_commands":["restart"],"servers":[],"ports":[],"exclude_paths":[]}"#,
    )
    .expect("decodes");
    let project = model::Project { id: PROJECT.into(), name: "Armada".into(), repositories: vec![model::Repository { id: "armada-main".into(), name: "armada".into() }] };
    let items = model::checks(&project, &[config]);
    let ask = |name: &str| match items.iter().find(|item| item.title == name).and_then(|item| item.buttons[0].clone()).map(|button| button.press) {
        Some(Press::Command { confirm, .. }) => confirm,
        other => panic!("{name} has no Run: {other:?}"),
    };
    assert!(ask("restart") && !ask("preview") && !ask("test"));
}

// ---- hiding blocks and adding them back ------------------------------------------------------------------------------------

#[test]
fn a_hidden_block_lets_the_rest_rise_and_an_added_one_takes_the_first_free_cells() {
    let kind = presets::Kind::Web;
    let mut layout = presets::preset(kind);
    layout.hide(BlockKind::Run);
    assert_eq!(layout.missing(), [BlockKind::Run]);
    assert_eq!(layout.block(BlockKind::Reviews).map(|one| (one.x, one.y)), Some((0, 0)), "Reviews rises into the room Run left");
    layout.add(BlockKind::Run);
    let run = layout.block(BlockKind::Run).copied().expect("Run is back");
    assert_eq!((run.x, run.y, run.w, run.h, run.view), (0, 4, 3, 2, View::List), "the first free cells, reading along each row from the top");
    assert!(layout.blocks.iter().all(|one| one.kind == BlockKind::Run || !one.overlaps(&run)));
}

#[gpui::test]
fn edit_mode_draws_a_hide_control_on_each_block_and_it_takes_the_block_off(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    assert!(!drawn(cx, Role::Button, "Hide Checks and commands"), "no hide control until the board is edited");
    board.update(cx, |board, cx| board.toggle_edit(cx));
    redraw(cx);
    for kind in BlockKind::ALL {
        assert!(drawn(cx, Role::Button, &format!("Hide {}", kind.title())), "{} has a hide control", kind.title());
    }
    press(cx, Role::Button, "Hide Checks and commands");
    assert!(layout_of(&board, cx).block(BlockKind::Checks).is_none());
    assert!(!drawn(cx, Role::Region, "Checks and commands"));
    cx.executor().advance_clock(SAVE_AFTER);
    cx.run_until_parked();
    assert!(kept_of(&board, cx).is_some_and(|kept| kept.blocks.iter().all(|one| one.kind != "checks")), "Fleet keeps the board without it");
}

#[gpui::test]
fn add_block_lists_the_kinds_not_on_the_board_and_a_pick_goes_on_it_by_keys(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    let board = screen.read_with(cx, |screen, _| screen.board.clone());
    assert!(!drawn(cx, Role::Button, "Add block"), "Add block is for editing the board");
    board.update(cx, |board, cx| board.toggle_edit(cx));
    redraw(cx);
    press(cx, Role::Button, "Hide Run");
    press(cx, Role::Button, "Hide Issues");
    press(cx, Role::Button, "Add block");
    assert!(drawn(cx, Role::Menu, "Blocks to add"));
    assert!(drawn(cx, Role::MenuItem, "Run") && drawn(cx, Role::MenuItem, "Issues"), "the kinds that are off the board");
    assert!(!drawn(cx, Role::MenuItem, "Reviews"), "a kind on the board is not offered");
    cx.simulate_keystrokes("down enter");
    redraw(cx);
    let layout = layout_of(&board, cx);
    let issues = layout.block(BlockKind::Issues).copied().expect("the second of the two is on the board");
    assert!(layout.block(BlockKind::Run).is_none(), "the first of the two is not");
    assert!(layout.blocks.iter().all(|one| one.kind == BlockKind::Issues || !one.overlaps(&issues)), "in cells that were free");
    assert!(!drawn(cx, Role::Menu, "Blocks to add"));
}

// ---- the librarian box ---------------------------------------------------------------------------------------------------

fn asked(screen: &Entity<ProjectScreen>, cx: &mut VisualTestContext) -> String {
    screen.read_with(cx, |screen, cx| screen.ask_box.read(cx).input.read(cx).value().to_string())
}

#[gpui::test]
fn the_librarian_box_offers_example_asks_on_focus_and_a_press_puts_one_in_it(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    assert!(!drawn(cx, Role::Menu, "Example asks"));
    cx.update(|window, cx| screen.read(cx).ask_box.read(cx).input.clone().update(cx, |state, cx| state.focus(window, cx)));
    redraw(cx);
    assert!(drawn(cx, Role::Menu, "Example asks"));
    for text in EXAMPLE_ASKS {
        assert!(drawn(cx, Role::MenuItem, text), "{text} is offered");
    }
    press(cx, Role::MenuItem, "What runs this project?");
    assert_eq!(asked(&screen, cx), "What runs this project?");
    assert!(!drawn(cx, Role::Menu, "Example asks"), "the list goes once one is in the box");
}

#[gpui::test]
fn the_arrows_and_enter_take_an_example_and_escape_puts_the_list_away(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    cx.update(|window, cx| screen.read(cx).ask_box.read(cx).input.clone().update(cx, |state, cx| state.focus(window, cx)));
    redraw(cx);
    cx.simulate_keystrokes("down down enter");
    redraw(cx);
    assert_eq!(asked(&screen, cx), EXAMPLE_ASKS[1]);
    assert!(!drawn(cx, Role::Menu, "Example asks"), "Enter takes the example and puts the list away");
    cx.simulate_keystrokes("down");
    redraw(cx);
    assert!(drawn(cx, Role::Menu, "Example asks"), "Down offers them again");
    cx.simulate_keystrokes("escape");
    redraw(cx);
    assert!(!drawn(cx, Role::Menu, "Example asks"), "Esc puts them away");
}

// ---- the issues ---------------------------------------------------------------------------------------------------

#[test]
fn a_ranked_issue_is_one_chip_and_a_column_that_names_the_features_it_matched() {
    let read = super::mock::issues();
    let items = model::issues(&read.mine, &read.ranked.might_pick_up);
    let matched = items.iter().find(|item| item.key == "issue-armada-9efp").expect("a ranked row");
    assert_eq!(matched.tag.as_ref().map(|tag| tag.0.as_str()), Some("Might pick up"));
    assert_eq!(matched.meta.as_deref(), Some("Feature library, Project board"), "the column names the features, in Fleet's order");
    let bare = items.iter().find(|item| item.key == "issue-armada-71cc").expect("a ranked row that matched none");
    assert_eq!(bare.meta, None, "an issue that matched no feature leaves the column empty");
    let yours = items.iter().find(|item| item.key == "issue-armada-a3f2").expect("his own");
    assert_eq!((yours.tag.as_ref().map(|tag| tag.0.as_str()), yours.meta.as_deref()), (Some("Yours"), None));
    assert_eq!(items.iter().map(|item| item.key.as_str()).collect::<Vec<_>>(), ["issue-armada-a3f2", "issue-armada-9efp", "issue-armada-71cc"], "his own first, then Fleet's ranking");
}

#[gpui::test]
fn the_issues_table_draws_each_ranked_row_named_by_its_features(cx: &mut TestAppContext) {
    let (_board, cx) = board_keeping(cx, None);
    assert!(drawn(cx, Role::Button, "armada-9efp Library sync, Feature library, Project board"));
    assert!(drawn(cx, Role::Button, "armada-71cc Pocket offline"));
}

#[test]
fn the_pull_requests_of_others_are_a_second_section_headed_once_and_left_out_where_a_list_holds_them() {
    let read = super::mock::issues();
    let own = model::Item::new("pull-4021", "git-pull-request", crate::look::token::SKETCH_AMBER, "Requested", "#4021 Rank issues by feature");
    assert!(model::interested(&read.ranked.might_be_interested, &[own]).is_empty(), "one Reviews lists already is not listed twice");
    let items = model::interested(&read.ranked.might_be_interested, &[]);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].section.as_deref(), Some("Might be interested"));
    assert_eq!(items[0].meta.as_deref(), Some("Feature library"));
    assert!(model::interested(&[], &[]).is_empty(), "no pull request, no section");
}

// ---- the tabs ------------------------------------------------------------------------------------------------------

#[test]
fn every_tab_draws_a_glyph_the_icon_set_has() {
    use gpui::AssetSource as _;
    for tab in Tab::ALL {
        let found = crate::assets::Assets.load(&crate::assets::icon_path(tab.glyph())).ok().flatten().is_some();
        assert!(found, "{} has no glyph in the set: {}", tab.title(), tab.glyph());
    }
    assert_eq!(Tab::ALL.map(Tab::title), ["Dashboard", "Features", "Since you looked", "Who's on what", "Settings"]);
}

#[test]
fn a_feature_waiting_on_him_reads_to_confirm_on_the_board_and_on_since_you_looked() {
    assert_eq!(crate::features::tag_mark(ipc::LookedTag::NeedsYou).0, "To confirm");
    let since = crate::features::mock::since(chrono::Utc::now() - chrono::Duration::days(3));
    let shown = model::features(Some(&since), None);
    let waiting = shown.items.iter().find(|item| item.tag.as_ref().is_some_and(|tag| tag.0 == "To confirm")).expect("a card that waits on him");
    assert!(matches!(waiting.press, Some(Press::Feature(_))), "a press opens that feature on the Features tab, where its Confirm is");
}

#[gpui::test]
fn the_header_settings_and_members_open_the_settings_tab_and_members_has_the_keys_in_the_list(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    assert!(drawn(cx, Role::Tab, "Settings"), "Settings is the fifth tab");
    press(cx, Role::Button, "Settings");
    assert_eq!(screen.read_with(cx, |screen, _| screen.tab), Tab::Settings);
    assert!(drawn(cx, Role::Form, "Project settings"), "the project's members and values are a page, not a layer");
    assert!(drawn(cx, Role::Button, "Reset board"), "the board's option is on the page");
    press(cx, Role::Tab, "Dashboard");
    press(cx, Role::Button, "Members");
    assert_eq!(screen.read_with(cx, |screen, _| screen.tab), Tab::Settings);
    let list = screen.read_with(cx, |screen, cx| screen.settings.read(cx).members_handle()).expect("the sheet is open as a page");
    assert!(cx.update(|window, _| list.is_focused(window)), "Members has the keys in the list of members");
}

// ---- live metrics ----------------------------------------------------------------------------------------------------------

fn instant(hour: u32, minute: u32) -> Instant {
    Instant::carried(format!("2026-11-24T{hour:02}:{minute:02}:00Z"))
}

fn number(label: &str, value: f64, at: Option<Instant>) -> MetricRow {
    MetricRow { label: label.into(), value: MetricValue::Number(value), unit: None, at, link: None }
}

fn reading(rows: Vec<MetricRow>, picture: Option<&str>, kept: Vec<MetricPoint>) -> MetricReading {
    MetricReading {
        id: "r".into(),
        title: "R".into(),
        state: MetricState::Read,
        answer: Some(MetricAnswer { rows, picture: picture.map(str::to_string) }),
        read_at: Some(instant(9, 0)),
        kept,
    }
}

fn mock_reading(id: &str) -> MetricReading {
    super::mock::readings(PROJECT).readings.into_iter().find(|one| one.id == id).unwrap_or_else(|| panic!("the mock has {id}"))
}

#[test]
fn a_metric_block_is_spelled_metric_and_the_recipe_id_both_ways() {
    let kind = BlockKind::from_slug("metric:errors-24h").expect("a metric block is a kind");
    assert_eq!(kind, BlockKind::metric("errors-24h"));
    assert_eq!(kind.metric_id(), Some("errors-24h"));
    assert_eq!(kind.slug(), "metric:errors-24h", "the word Fleet keeps");
    assert_eq!(BlockKind::from_slug(&kind.slug()), Some(kind));
    assert_eq!(BlockKind::from_slug("metric:"), None, "a recipe with no id is no block");
    let kept = BoardLayout { blocks: vec![block("metric:errors-24h", 0, 0, 3, 2, "graph")] };
    let (layout, unknown) = Layout::from_wire(&kept);
    assert_eq!(layout.blocks, vec![Placed::new(kind, 0, 0, 3, 2, View::Graph)], "its view is one a metric draws");
    assert!(unknown.is_empty());
    assert_eq!(layout.to_wire(&unknown), kept, "and it goes back as it came");
}

#[gpui::test]
fn a_metric_block_with_no_recipe_in_the_readings_draws_nothing_and_is_kept_as_saved(cx: &mut TestAppContext) {
    let kept = BoardLayout { blocks: vec![block("run", 0, 0, 3, 2, "list"), block("metric:retired", 3, 0, 3, 2, "graph")] };
    let (board, cx) = board_keeping(cx, Some(kept));
    assert!(layout_of(&board, cx).block(BlockKind::metric("retired")).is_some(), "the layout still holds it");
    assert!(!drawn(cx, Role::Region, "retired"), "nothing is drawn for it");
    board.update(cx, |board, cx| board.edit_layout(PROJECT, |layout| layout.nudge(BlockKind::Run, 0, 1), cx));
    cx.executor().advance_clock(SAVE_AFTER);
    cx.run_until_parked();
    let sent = kept_of(&board, cx).expect("the move is kept");
    assert_eq!(sent.blocks.iter().find(|one| one.kind == "metric:retired"), Some(&block("metric:retired", 3, 0, 3, 2, "graph")), "kept as saved");
}

#[gpui::test]
fn a_recipe_the_board_has_no_block_for_is_put_at_the_bottom_and_drawn(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    let layout = layout_of(&board, cx);
    let floor = fixed(layout.clone()).rows();
    for id in ["errors-24h", "build-time", "deploys-week", "queue-depth"] {
        let at = layout.block(BlockKind::metric(id)).unwrap_or_else(|| panic!("{id} is on the board"));
        assert!(at.y >= floor, "{id} stands below the blocks that were there");
    }
    for title in ["Errors, last day", "Build time", "Deploys this week", "Queue depth"] {
        assert!(drawn(cx, Role::Region, title), "{title} is drawn");
    }
}

#[test]
fn picture_is_offered_only_where_the_answer_carries_one() {
    let without = reading(vec![number("Now", 1., None)], None, Vec::new());
    let with = reading(vec![number("Now", 1., None)], Some("https://example.com/a.png"), Vec::new());
    assert_eq!(metric::offered(&without), [View::List, View::Table, View::Cards, View::Graph]);
    assert_eq!(metric::offered(&with), [View::List, View::Table, View::Cards, View::Graph, View::Picture]);
    assert_eq!(metric::drawn_in(View::Picture, &without), View::List, "a saved Picture is drawn as the first view until the tool gives one");
    assert_eq!(metric::drawn_in(View::Picture, &with), View::Picture);
}

#[gpui::test]
fn the_view_switch_of_a_metric_block_offers_picture_only_with_a_picture(cx: &mut TestAppContext) {
    let (_board, cx) = board_keeping(cx, None);
    assert!(drawn(cx, Role::Button, "Picture view of Build time"));
    assert!(!drawn(cx, Role::Button, "Picture view of Errors, last day"));
    assert!(drawn(cx, Role::Button, "Graph view of Errors, last day"));
}

#[test]
fn graph_draws_the_rows_that_carry_a_time_and_else_what_fleet_kept() {
    let kept = vec![MetricPoint { at: instant(7, 0), value: 1. }, MetricPoint { at: instant(8, 0), value: 2. }];
    let timed = reading(vec![number("later", 5., Some(instant(9, 0))), number("earlier", 3., Some(instant(6, 0)))], None, kept.clone());
    match metric::plot(&timed) {
        Plot::Rows(points) => assert_eq!(points.iter().map(|(_, value)| *value).collect::<Vec<_>>(), [3., 5.], "in time, whatever order the rows came in"),
        other => panic!("rows that carry a time are the series: {other:?}"),
    }
    let bare = reading(vec![number("Main", 412., None)], None, kept.clone());
    match metric::plot(&bare) {
        Plot::Kept(points) => assert_eq!(points.iter().map(|(_, value)| *value).collect::<Vec<_>>(), [1., 2.]),
        other => panic!("rows with no time draw what was kept: {other:?}"),
    }
    let one = reading(vec![number("only", 9., Some(instant(9, 0)))], None, kept);
    assert!(matches!(metric::plot(&one), Plot::Kept(_)), "one timed row is no series");
}

#[gpui::test]
fn the_token_field_is_masked_keeps_no_draft_and_goes_when_the_token_is_sent(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    assert!(drawn(cx, Role::TextInput, "Token for Deploys this week"), "the credential's block has a field");
    assert!(drawn(cx, Role::Link, "Make a token for Deploys this week"), "the credential names where it is made");
    let input = board.read_with(cx, |board, _| board.token_fields.get("deploys-week").map(|field| field.input.clone())).expect("a field for the recipe with no token");
    // Typed, not set: typing is what tells the box a change was made, which is what a draft would be kept on.
    cx.update(|window, cx| {
        window.activate_window();
        input.update(cx, |state, cx| state.focus(window, cx));
    });
    cx.simulate_input("sk-live-secret");
    redraw(cx);
    assert!(cx.update(|_, cx| input.read(cx).value() == "sk-live-secret"), "the field holds what was pasted");
    assert!(!cx.update(|_, cx| crate::drafts::holds("sk-live-secret", cx)), "a secret is never kept as a draft");
    press(cx, Role::Button, "Save the token for Deploys this week");
    assert!(!drawn(cx, Role::TextInput, "Token for Deploys this week"), "the field is gone once the token is sent");
    assert!(board.read_with(cx, |board, _| board.token_fields.is_empty()), "and nothing holds the words");
    assert!(!cx.update(|_, cx| crate::drafts::holds("sk-live-secret", cx)));
    assert!(matches!(board.read_with(cx, |board, _| board.reading("deploys-week").map(|one| one.state.clone())), Some(MetricState::Read)), "the recipe reads from then on");
}

#[gpui::test]
fn save_is_off_while_the_field_is_empty(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    let at = find(cx, Role::Button, "Save the token for Deploys this week").expect("Save is drawn").center();
    cx.simulate_click(at, Modifiers::default());
    redraw(cx);
    assert!(board.read_with(cx, |board, _| board.token_fields.contains_key("deploys-week")), "an empty field sends nothing");
}

#[test]
fn a_broken_reading_carries_the_time_of_its_last_good_one() {
    let broken = mock_reading("queue-depth");
    let (said, full) = metric::last_good(&broken).expect("it still holds an answer");
    assert_eq!(said, format!("Last good reading {}", metric::clock(&instant(8, 12)).expect("a time")));
    assert!(!full.is_empty(), "the tooltip has the full time");
    let mut never = broken;
    never.answer = None;
    assert_eq!(metric::last_good(&never), None, "none ever read, none to name");
}

#[gpui::test]
fn a_broken_block_says_why_and_its_fix_puts_the_words_in_the_ask_box(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    assert!(drawn(cx, Role::Region, "Queue depth"));
    assert!(drawn(cx, Role::Image, "Could not read"), "its state is a glyph with words");
    assert!(drawn(cx, Role::Button, "Fix Queue depth with the librarian"));
    press(cx, Role::Button, "Fix Queue depth with the librarian");
    assert_eq!(asked(&screen, cx), "Fix Queue depth");
    let focused = screen.read_with(cx, |screen, cx| gpui::Focusable::focus_handle(screen.ask_box.read(cx).input.read(cx), cx));
    assert!(cx.update(|window, _| focused.is_focused(window)), "the box has the keys");
}

#[gpui::test]
fn a_saved_layout_with_no_block_for_a_recipe_leaves_it_off_and_the_add_menu_offers_it(cx: &mut TestAppContext) {
    let kept = BoardLayout { blocks: vec![block("run", 0, 0, 3, 2, "list")] };
    let (board, cx) = board_keeping(cx, Some(kept));
    assert!(layout_of(&board, cx).blocks.iter().all(|one| one.kind.metric_id().is_none()), "Fleet adds a recipe's block when it saves the recipe, so one with none was taken off");
    assert!(!drawn(cx, Role::Region, "Queue depth"));
    let offered = board.update(cx, |board, _| board.addable(PROJECT));
    assert!(offered.contains(&BlockKind::metric("queue-depth")), "the Add block menu has it");
    board.update(cx, |board, cx| board.add_block(BlockKind::metric("queue-depth"), cx));
    redraw(cx);
    assert!(drawn(cx, Role::Region, "Queue depth"), "and a pick puts it on the board");
}

#[gpui::test]
fn hiding_a_metric_block_saves_the_layout_without_it_and_it_stays_off(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    board.update(cx, |board, cx| board.toggle_edit(cx));
    redraw(cx);
    press(cx, Role::Button, "Hide Queue depth");
    cx.executor().advance_clock(SAVE_AFTER);
    cx.run_until_parked();
    let sent = kept_of(&board, cx).expect("the board is kept");
    assert!(sent.blocks.iter().all(|one| one.kind != "metric:queue-depth"), "saved without it, like any other block");
    assert!(sent.blocks.iter().any(|one| one.kind == "metric:errors-24h"), "the others are on it");
    board.update(cx, |board, cx| board.apply_readings(super::mock::readings(PROJECT), cx));
    redraw(cx);
    assert!(layout_of(&board, cx).block(BlockKind::metric("queue-depth")).is_none(), "the next reading does not put it back");
    assert!(!drawn(cx, Role::Region, "Queue depth"));
}

#[test]
fn the_host_of_a_picture_is_the_address_without_its_scheme_and_path() {
    assert_eq!(metric::host("https://grafana.example.com/render/d?id=3#top"), "grafana.example.com");
    assert_eq!(metric::host("https://example.com"), "example.com");
}

#[gpui::test]
fn the_picture_view_is_a_card_with_the_host_that_opens_the_address(cx: &mut TestAppContext) {
    let (_board, cx) = board_keeping(cx, None);
    press(cx, Role::Button, "Picture view of Build time");
    assert!(drawn(cx, Role::Button, "Open the picture of Build time from example.com"));
}

#[test]
fn the_number_shown_large_is_not_a_row_again_beneath_it() {
    let row = |label: &str, value: MetricValue, at: Option<&str>| MetricRow { label: label.to_string(), value, unit: None, at: at.map(Instant::carried), link: None };
    let answer = MetricAnswer {
        rows: vec![row("Open pull requests", MetricValue::Number(3.0), None), row("#4023 Say a stalled step waits", MetricValue::Text("NickMele".to_string()), None)],
        picture: None,
    };
    assert_eq!(metric::headline(&answer).map(|head| head.label), Some("Open pull requests".to_string()));
    let labels: Vec<String> = metric::items("open-pulls", &answer).into_iter().map(|item| item.title.to_string()).collect();
    assert_eq!(labels, ["#4023 Say a stalled step waits"], "the headline row is drawn large, not listed again");

    let series = MetricAnswer {
        rows: vec![row("errors", MetricValue::Number(4.0), Some("2026-11-24T10:00:00Z")), row("errors", MetricValue::Number(6.0), Some("2026-11-24T11:00:00Z"))],
        picture: None,
    };
    assert_eq!(metric::items("errors", &series).len(), 2, "a point of a series stays in the rows");
}

#[gpui::test]
fn a_block_with_rows_out_of_sight_keeps_the_wheel_and_the_board_stands_still(cx: &mut TestAppContext) {
    // Features one row tall, so its cards run out of sight, over enough blocks that the board scrolls too.
    let kept = BoardLayout {
        blocks: vec![
            block("features", 0, 0, 6, 1, "cards"),
            block("checks", 0, 1, 6, 2, "list"),
            block("reviews", 0, 3, 6, 2, "cards"),
            block("issues", 0, 5, 6, 2, "table"),
            block("needs-you", 0, 7, 6, 2, "list"),
        ],
    };
    let (board, cx) = board_keeping(cx, Some(kept));
    cx.simulate_resize(gpui::size(gpui::px(1280.), gpui::px(700.)));
    redraw(cx);
    let wheel = |cx: &mut VisualTestContext, at: gpui::Point<Pixels>| {
        cx.simulate_event(gpui::ScrollWheelEvent { position: at, delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(-60.))), ..Default::default() });
        redraw(cx);
    };
    let overflows = cx.update(|_, cx| board.read(cx).bodies.borrow().get("features-body").is_some_and(|body| body.max_offset().y > gpui::px(0.)));
    assert!(overflows, "Features one row tall has cards out of sight");
    let features = find(cx, Role::Region, "Features").expect("the Features block is drawn");

    wheel(cx, features.center());
    let after = find(cx, Role::Region, "Features").expect("still drawn");
    assert_eq!(after.origin.y, features.origin.y, "the board stood still while the block scrolled");

    let checks = find(cx, Role::Region, "Checks and commands").expect("the Checks block is drawn");
    wheel(cx, gpui::point(checks.origin.x + gpui::px(40.), checks.origin.y + gpui::px(10.)));
    let moved = find(cx, Role::Region, "Checks and commands").expect("still drawn");
    assert!(moved.origin.y < checks.origin.y, "a wheel over a block's header still moves the board");
}
