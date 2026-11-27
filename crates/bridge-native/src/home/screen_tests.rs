//! The Cockpit's home: the grid it draws, the layout it keeps on Fleet and reads back, and its place on the Cockpit's right side.

use std::time::Duration;

use gpui::{div, prelude::*, AnyView, Bounds, Context, Entity, Modifiers, Pixels, Render, Role, TestAppContext, VisualTestContext, Window};
use ipc::BoardLayout;

use super::super::layout::{join, preset, split};
use super::{HomeScreen, SAVE_AFTER};
use crate::features::{mock, FeaturesScreen};
use crate::grid::View;
use crate::home::{HomeBlockKind, NeedsScope};
use crate::jobs::{JobsList, JobsScreen};
use crate::operator::page::OperatorPageView;
use crate::operator::OperatorScreen;

fn ready(cx: &mut TestAppContext) {
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

fn redraw(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
}

fn find(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<Bounds<Pixels>> {
    cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).map(|one| one.bounds))
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    find(cx, role, name).is_some()
}

fn press(cx: &mut VisualTestContext, role: Role, name: &str) {
    let at = find(cx, role, name).unwrap_or_else(|| panic!("{name} is drawn")).center();
    cx.simulate_click(at, Modifiers::none());
    redraw(cx);
}

/// The home alone on the mock Fleet, with the Jobs the walks use.
fn home_alone(cx: &mut TestAppContext) -> (Entity<HomeScreen>, &mut VisualTestContext) {
    ready(cx);
    let mut held = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let records = crate::mock::jobs("jobs-many").expect("the fixture decodes");
        let list = cx.new(|cx| JobsList::mock(&records, cx));
        let page = cx.new(|cx| OperatorPageView::mock(window, cx));
        let features = cx.new(|cx| FeaturesScreen::mock_holding(mock::floor_library(), window, cx));
        let home = cx.new(|cx| HomeScreen::mock(page, features, list, cx));
        held = Some(home.clone());
        Rooted(home.into())
    });
    cx.simulate_resize(gpui::size(gpui::px(1280.), gpui::px(1400.)));
    redraw(cx);
    (held.expect("the home was made"), cx)
}

fn kinds(home: &Entity<HomeScreen>, cx: &mut VisualTestContext) -> Vec<HomeBlockKind> {
    home.read_with(cx, |home, _| home.layout().blocks.iter().map(|placed| placed.kind).collect())
}

/// Wait out the save, as the clock runs.
fn kept_after_wait(cx: &mut VisualTestContext) {
    cx.executor().advance_clock(SAVE_AFTER + Duration::from_millis(10));
    redraw(cx);
}

// -- the preset ----------------------------------------------------------------------------------

#[gpui::test]
fn with_nothing_kept_the_home_is_the_preset_and_nothing_is_saved_for_looking_at_it(cx: &mut TestAppContext) {
    let (home, cx) = home_alone(cx);
    assert_eq!(kinds(&home, cx), [HomeBlockKind::NeedsCards, HomeBlockKind::SystemOverview, HomeBlockKind::FloorPlan]);
    assert!(!home.read_with(cx, |home, _| home.is_edited()), "it is the preset, not a layout he kept");
    for title in ["Needs you", "Floor plan", "System overview"] {
        assert!(drawn(cx, Role::Region, title), "{title} is drawn");
    }
    assert!(!drawn(cx, Role::Region, "System stats"), "System stats is not on the preset");
    let first = home.read_with(cx, |home, _| home.layout().block(HomeBlockKind::NeedsCards).copied()).expect("needs block");
    assert_eq!((first.x, first.y, first.w), (0, 0, 6), "the Needs cards run the full width at the top");
    press(cx, Role::Button, "Edit");
    press(cx, Role::Button, "Done");
    kept_after_wait(cx);
    assert!(home.read_with(cx, |home, _| home.kept().is_none()), "Edit and Done change nothing, so nothing is sent");
}

// -- the empty state, add and hide -----------------------------------------------------------------

#[gpui::test]
fn hiding_every_block_shows_nothing_here_yet_with_add_block(cx: &mut TestAppContext) {
    let (home, cx) = home_alone(cx);
    press(cx, Role::Button, "Edit");
    for title in ["Needs you", "Floor plan", "System overview"] {
        press(cx, Role::Button, &format!("Hide {title}"));
    }
    assert!(kinds(&home, cx).is_empty());
    assert!(drawn(cx, Role::Status, "Nothing here yet"), "the empty state says so");
    assert!(drawn(cx, Role::Button, "Add block"), "and offers the way back");
    assert!(!drawn(cx, Role::Region, "Floor plan"));
}

#[gpui::test]
fn add_block_lists_the_kinds_that_are_off_the_grid_and_a_pick_puts_it_on(cx: &mut TestAppContext) {
    let (home, cx) = home_alone(cx);
    press(cx, Role::Button, "Edit");
    press(cx, Role::Button, "Hide Floor plan");
    assert!(!drawn(cx, Role::Region, "Floor plan"));
    press(cx, Role::Button, "Add block");
    assert!(drawn(cx, Role::Menu, "Blocks to add"));
    assert!(drawn(cx, Role::MenuItem, "Floor plan") && drawn(cx, Role::MenuItem, "System stats"), "the kinds that are off");
    assert!(!drawn(cx, Role::MenuItem, "Needs you"), "a kind on the grid is not offered");
    press(cx, Role::MenuItem, "Floor plan");
    assert!(kinds(&home, cx).contains(&HomeBlockKind::FloorPlan));
    assert!(drawn(cx, Role::Region, "Floor plan"));
    assert!(!drawn(cx, Role::Menu, "Blocks to add"), "the menu shuts on a pick");
}

#[gpui::test]
fn from_the_empty_state_add_block_brings_a_block_back(cx: &mut TestAppContext) {
    let (home, cx) = home_alone(cx);
    home.update(cx, |home, cx| home.edit_layout(|layout| layout.blocks.clear(), cx));
    redraw(cx);
    assert!(drawn(cx, Role::Status, "Nothing here yet"));
    press(cx, Role::Button, "Add block");
    press(cx, Role::MenuItem, "System stats");
    assert_eq!(kinds(&home, cx), [HomeBlockKind::SystemStats]);
    assert!(!drawn(cx, Role::Status, "Nothing here yet"));
    assert!(drawn(cx, Role::Region, "System stats"));
}

#[gpui::test]
fn the_needs_block_switches_between_cards_and_lines(cx: &mut TestAppContext) {
    let (home, cx) = home_alone(cx);
    assert!(drawn(cx, Role::Group, "View of Needs you"), "the Needs block has a view switch");
    assert!(!drawn(cx, Role::Group, "View of Floor plan"), "a block with one view has none");
    press(cx, Role::Button, "List view of Needs you");
    let view = home.read_with(cx, |home, _| home.layout().block(HomeBlockKind::NeedsCards).map(|one| one.view));
    assert_eq!(view, Some(View::List));
}

// -- the layout on Fleet ---------------------------------------------------------------------------------

#[gpui::test]
fn a_change_is_sent_whole_once_the_grid_has_stood_still_and_read_back_as_it_was(cx: &mut TestAppContext) {
    let (home, cx) = home_alone(cx);
    press(cx, Role::Button, "Edit");
    press(cx, Role::Button, "Hide Floor plan");
    cx.executor().advance_clock(SAVE_AFTER / 3);
    assert!(home.read_with(cx, |home, _| home.kept().is_none()), "not sent while the grid may still move");
    kept_after_wait(cx);
    let kept = home.read_with(cx, |home, _| home.kept().cloned()).expect("Fleet kept the layout");
    let sent: Vec<&str> = kept.blocks.iter().map(|block| block.kind.as_str()).collect();
    assert_eq!(sent, ["needs-cards", "system-overview"]);
    // Another window's read of it: the same layout comes back.
    home.update(cx, |home, cx| {
        home.stored.layout = preset();
        home.reread_layout(cx);
    });
    assert_eq!(kinds(&home, cx), [HomeBlockKind::NeedsCards, HomeBlockKind::SystemOverview]);
    assert!(home.read_with(cx, |home, _| home.is_edited()));
}

#[gpui::test]
fn a_read_that_comes_while_a_change_is_unsent_leaves_the_change(cx: &mut TestAppContext) {
    let (home, cx) = home_alone(cx);
    press(cx, Role::Button, "Edit");
    press(cx, Role::Button, "Hide Floor plan");
    home.update(cx, |home, cx| home.apply_kept(None, cx));
    assert!(!kinds(&home, cx).contains(&HomeBlockKind::FloorPlan), "his change stands over Fleet's answer");
}

#[gpui::test]
fn a_layout_with_blocks_this_build_does_not_know_sends_them_back_as_they_came(cx: &mut TestAppContext) {
    let (home, cx) = home_alone(cx);
    let block = |kind: &str, y: u8, view: &str| ipc::BoardBlock { kind: kind.into(), x: 0, y, w: 3, h: 2, view: view.into() };
    let kept = BoardLayout { blocks: vec![block("needs-cards", 0, "cards"), block("weather", 2, "graph")] };
    home.update(cx, |home, cx| home.apply_kept(Some(kept), cx));
    redraw(cx);
    assert_eq!(kinds(&home, cx), [HomeBlockKind::NeedsCards]);
    press(cx, Role::Button, "Edit");
    press(cx, Role::Button, "Add block");
    press(cx, Role::MenuItem, "Floor plan");
    kept_after_wait(cx);
    let sent = home.read_with(cx, |home, _| home.kept().cloned()).expect("kept");
    assert!(sent.blocks.iter().any(|block| block.kind == "weather"), "the stranger goes back");
}

// -- the Needs block's scope -------------------------------------------------------------------------

#[gpui::test]
fn the_needs_scope_is_kept_with_the_layout_and_is_the_scope_again_when_it_is_read(cx: &mut TestAppContext) {
    let (home, cx) = home_alone(cx);
    assert_eq!(home.read_with(cx, |home, _| home.scope()), NeedsScope::Selected);
    press(cx, Role::Button, "Needs you in all projects");
    assert_eq!(home.read_with(cx, |home, _| home.scope()), NeedsScope::All, "the header's control sets it");
    kept_after_wait(cx);
    let kept = home.read_with(cx, |home, _| home.kept().cloned()).expect("a scope change keeps the layout");
    let needs = kept.blocks.iter().find(|block| block.kind == "needs-cards").expect("needs block");
    assert_eq!(needs.view, "cards:all", "the scope rides in the view word");
    home.update(cx, |home, cx| {
        home.stored.scope = NeedsScope::Selected;
        home.reread_layout(cx);
    });
    redraw(cx);
    assert_eq!(home.read_with(cx, |home, _| home.scope()), NeedsScope::All, "read back, the scope is All");
    assert!(drawn(cx, Role::Group, "Scope of Needs you"), "and the control is in the block's header");
    // A change after it keeps the scope.
    press(cx, Role::Button, "Edit");
    press(cx, Role::Button, "Hide Floor plan");
    kept_after_wait(cx);
    let after = home.read_with(cx, |home, _| home.kept().cloned()).expect("kept");
    assert_eq!(after.blocks.iter().find(|block| block.kind == "needs-cards").map(|block| block.view.as_str()), Some("cards:all"));
}

#[test]
fn the_scope_word_is_split_from_the_view_and_a_word_not_known_is_the_default() {
    let block = |view: &str| ipc::BoardBlock { kind: "needs-cards".into(), x: 0, y: 0, w: 6, h: 2, view: view.into() };
    let (plain, scope) = split(&BoardLayout { blocks: vec![block("list:all")] });
    assert_eq!((plain.blocks[0].view.as_str(), scope), ("list", NeedsScope::All));
    let (plain, scope) = split(&BoardLayout { blocks: vec![block("cards")] });
    assert_eq!((plain.blocks[0].view.as_str(), scope), ("cards", NeedsScope::Selected), "a layout saved before the scope was kept has the default");
    let (_, scope) = split(&BoardLayout { blocks: vec![block("cards:everything")] });
    assert_eq!(scope, NeedsScope::Selected);
    let joined = join(BoardLayout { blocks: vec![block("cards")] }, NeedsScope::Selected);
    assert_eq!(joined.blocks[0].view, "cards", "the default is left off");
    let joined = join(BoardLayout { blocks: vec![block("list")] }, NeedsScope::All);
    assert_eq!(joined.blocks[0].view, "list:all");
}

// -- the Cockpit -----------------------------------------------------------------------------------------

/// A window that draws the screen as the app does.
struct Hosted(Entity<OperatorScreen>);

impl Render for Hosted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::labels::label_root(div().size_full().child(self.0.clone()))
    }
}

fn cockpit(off: bool, cx: &mut TestAppContext) -> (Entity<OperatorScreen>, &mut VisualTestContext) {
    ready(cx);
    let (host, cx) = cx.add_window_view(|window, cx| {
        let records = crate::mock::jobs("jobs-many").expect("the fixture decodes");
        let list = cx.new(|cx| JobsList::mock(&records, cx));
        let threads = crate::operator::scenes::page_threads().expect("the threads decode");
        let jobs = cx.new(|cx| JobsScreen::new(list, Some(threads), window, cx));
        Hosted(cx.new(|cx| OperatorScreen::mock_cockpit_holding(jobs, mock::floor_library(), window, cx)))
    });
    cx.simulate_resize(gpui::size(gpui::px(1600.), gpui::px(1200.)));
    let screen = host.read_with(cx, |host, _| host.0.clone());
    redraw(cx);
    if off {
        screen.read_with(cx, |screen, _| screen.page.clone()).update(cx, |page, cx| page.scene("off", cx).expect("the off scene"));
        redraw(cx);
    }
    (screen, cx)
}

#[gpui::test]
fn with_the_operator_off_the_right_side_is_the_home_under_a_slim_strip_with_switch_on(cx: &mut TestAppContext) {
    let (_screen, cx) = cockpit(true, cx);
    assert!(drawn(cx, Role::Group, "Home"), "the home stands in the page's place");
    for title in ["Needs you", "Floor plan", "System overview"] {
        assert!(drawn(cx, Role::Region, title), "{title} is a block");
    }
    assert!(drawn(cx, Role::Status, "Operator off"), "the strip says the Operator is off");
    assert!(drawn(cx, Role::Button, "Switch the Operator on"), "and holds the page's Switch on");
    assert!(drawn(cx, Role::Button, "Edit"));
    assert!(!drawn(cx, Role::Region, "Floor"), "the old Floor view is gone");
    assert!(drawn(cx, Role::Region, "Feature map"), "the floor plan block draws the map");
}

#[gpui::test]
fn a_press_on_switch_on_turns_the_operator_on_and_the_page_comes_back(cx: &mut TestAppContext) {
    let (screen, cx) = cockpit(true, cx);
    press(cx, Role::Button, "Switch the Operator on");
    let page = screen.read_with(cx, |screen, _| screen.page.clone());
    assert!(page.read_with(cx, |page, _| !page.is_off()), "the Operator reads on");
    assert!(!drawn(cx, Role::Group, "Home"), "the home gives the side back");
    assert!(drawn(cx, Role::TextInput, "Message"), "the Operator's page is at the right again");
}

#[gpui::test]
fn with_the_operator_on_the_page_is_unchanged_and_there_is_no_home(cx: &mut TestAppContext) {
    let (_screen, cx) = cockpit(false, cx);
    assert!(!drawn(cx, Role::Group, "Home"));
    assert!(!drawn(cx, Role::Button, "Switch the Operator on"));
    assert!(drawn(cx, Role::TextInput, "Message"));
}

#[gpui::test]
fn a_job_open_takes_the_right_side_from_the_home(cx: &mut TestAppContext) {
    let (screen, cx) = cockpit(true, cx);
    press(cx, Role::Row, "Fix the merge line freshness chip");
    assert!(!screen.read_with(cx, |screen, cx| screen.page_shown(cx)), "the press opened the Job");
    assert!(!drawn(cx, Role::Group, "Home"), "an open Job fills the right side");
}

#[gpui::test]
fn a_press_on_a_jobs_chip_in_the_floor_plan_opens_that_job(cx: &mut TestAppContext) {
    let (screen, cx) = cockpit(true, cx);
    press(cx, Role::Button, "Native Bridge foundation, Working");
    assert!(!screen.read_with(cx, |screen, cx| screen.page_shown(cx)));
}
