//! The home blocks: what each draws, how the Needs cards are scoped, and that each fills the box it is given.

use gpui::{div, prelude::*, px, AnyView, Bounds, Context, Entity, Modifiers, Pixels, Role, TestAppContext, VisualTestContext, Window};
use ipc::SessionRecord;

use super::needs::needing;
use super::{HomeBlockKind, NeedsCards, NeedsScope, PlaceholderBlock, ScopeChanged};
use crate::features::floor_home::{cards, Floor};
use crate::features::{mock as features_mock, FeaturesScreen};
use crate::jobs::JobsList;

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
    });
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

fn press(cx: &mut VisualTestContext, role: Role, name: &str) {
    let at = find(cx, role, name).unwrap_or_else(|| panic!("{name} is drawn")).center();
    cx.simulate_click(at, Modifiers::none());
    redraw(cx);
}

/// A window whose root clears the labels each frame, as the app's does, and holds one block in a box of a given size.
struct Boxed {
    block: AnyView,
    wide: f32,
    tall: f32,
}

impl Render for Boxed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::labels::label_root(div().size_full().child(div().id("the-box").w(px(self.wide)).h(px(self.tall)).overflow_hidden().child(self.block.clone())))
    }
}

fn jobs() -> Vec<SessionRecord> {
    crate::mock::jobs("jobs-many").expect("the fixture decodes")
}

/// Needs cards over `records`, in a box, with the list that holds them.
fn needs_in(wide: f32, tall: f32, scope: NeedsScope, records: Vec<SessionRecord>, cx: &mut TestAppContext) -> (Entity<NeedsCards>, Entity<JobsList>, &mut VisualTestContext) {
    ready(cx);
    let mut held = None;
    let (_, cx) = cx.add_window_view(|_, cx| {
        let list = cx.new(|cx| JobsList::mock(&records, cx));
        let block = cx.new(|cx| NeedsCards::new(list.clone(), scope, cx));
        held = Some((block.clone(), list));
        Boxed { block: block.into(), wide, tall }
    });
    redraw(cx);
    let (block, list) = held.expect("the block was made");
    (block, list, cx)
}

/// The plan alone, in a box, on a library and the mock Jobs.
fn floor_in(wide: f32, tall: f32, library: ipc::FeatureLibrary, cx: &mut TestAppContext) -> (Entity<JobsList>, &mut VisualTestContext) {
    ready(cx);
    let mut held = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let list = cx.new(|cx| JobsList::mock(&jobs(), cx));
        let features = cx.new(|cx| FeaturesScreen::mock_holding(library, window, cx));
        let floor = cx.new(|cx| Floor::plan(features, list.clone(), cx));
        held = Some(list);
        Boxed { block: floor.into(), wide, tall }
    });
    redraw(cx);
    (held.expect("the list was made"), cx)
}

fn ids(cards: &[super::NeedCard]) -> Vec<&str> {
    cards.iter().map(|card| card.id.as_str()).collect()
}

// -- kinds and scope ------------------------------------------------------------------------------

#[test]
fn each_kind_is_named_by_its_slug_and_a_stranger_is_none() {
    let slugs: Vec<_> = HomeBlockKind::ALL.iter().map(|kind| kind.slug()).collect();
    assert_eq!(slugs, ["needs-cards", "floor-plan", "system-overview", "system-stats"]);
    for kind in HomeBlockKind::ALL {
        assert_eq!(HomeBlockKind::from_slug(kind.slug()), Some(kind));
        assert!(!kind.title().is_empty());
    }
    assert_eq!(HomeBlockKind::from_slug("dashboard"), None, "a kind a later Bridge adds reads as none");
    assert_eq!(HomeBlockKind::NeedsCards.title(), "Needs you");
}

#[test]
fn the_scope_is_kept_as_a_word_and_starts_on_the_selected_project() {
    assert_eq!(NeedsScope::default(), NeedsScope::Selected);
    for scope in NeedsScope::ALL {
        assert_eq!(NeedsScope::from_slug(scope.slug()), Some(scope));
    }
    assert_eq!((NeedsScope::Selected.slug(), NeedsScope::All.slug()), ("selected", "all"));
    assert_eq!(NeedsScope::from_slug("everything"), None);
}

// -- who needs him ----------------------------------------------------------------------------------

#[test]
fn the_selected_scope_keeps_one_projects_jobs_and_all_keeps_every_one() {
    let (records, projects) = (jobs(), crate::mock::projects());
    let in_all = needing(&records, Some(&projects), NeedsScope::All, Some("armada"));
    assert_eq!(ids(&in_all), ["mock-01", "mock-10"], "the Jobs whose mark says Needs you, longest waiting first");
    let armada = needing(&records, Some(&projects), NeedsScope::Selected, Some("armada"));
    assert_eq!(ids(&armada), ["mock-01"], "Armada's own");
    let storefront = needing(&records, Some(&projects), NeedsScope::Selected, Some("storefront"));
    assert_eq!(ids(&storefront), ["mock-10"], "a Job of a repository the project holds is in it");
    assert_eq!(storefront[0].project.as_deref(), Some("Storefront"));
    let none = needing(&records, Some(&projects), NeedsScope::Selected, None);
    assert_eq!(ids(&none), ["mock-01", "mock-10"], "with no project selected there is nothing to narrow to");
    assert!(needing(&records, Some(&projects), NeedsScope::Selected, Some("ledger")).is_empty(), "a project with no Job waiting has no card");
}

#[test]
fn a_snoozed_job_and_a_job_that_is_not_live_are_left_out_as_the_jobs_list_leaves_them() {
    let (mut records, projects) = (jobs(), crate::mock::projects());
    let at = records.iter().position(|job| job.id.as_str() == "mock-10").expect("the fixture holds the Job");
    records[at].snoozed_until = Some(ipc::Instant::carried("2099-01-01T00:00:00Z"));
    let at = records.iter().position(|job| job.id.as_str() == "mock-01").expect("the fixture holds the Job");
    records[at].state = ipc::SessionState::Ended;
    assert!(needing(&records, Some(&projects), NeedsScope::All, None).is_empty());
}

#[test]
fn a_card_says_what_its_job_waits_for_in_a_few_lines_and_says_there_is_more() {
    let (records, projects) = (jobs(), crate::mock::projects());
    let mut all = needing(&records, Some(&projects), NeedsScope::All, None);
    let bare = all.remove(1);
    let one = all.remove(0);
    assert_eq!(one.title, "Fix the merge line freshness chip");
    assert_eq!(one.lines, ["Plan to approve", "Tail layout", "Which colour does the stale chip take?"], "the first three open items");
    assert!(one.more, "it owes more than the card says");
    assert_eq!(one.since.as_deref(), Some("2026-11-09T13:41:00Z"), "the oldest item says how long it has waited");
    assert_eq!(one.name(), "Fix the merge line freshness chip, Plan to approve");
    assert!(bare.lines.is_empty() && !bare.more && bare.since.is_none(), "a Job marked Needs you with no item owed still has its card");
}

#[test]
fn the_same_jobs_give_equal_cards_which_is_what_keeps_a_redraw_away() {
    let (records, projects) = (jobs(), crate::mock::projects());
    assert_eq!(needing(&records, Some(&projects), NeedsScope::All, None), needing(&records, Some(&projects), NeedsScope::All, None));
}

// -- the Needs block drawn ------------------------------------------------------------------------------

#[gpui::test]
fn the_block_draws_a_card_for_each_job_that_needs_him_and_a_press_opens_it(cx: &mut TestAppContext) {
    let (block, list, cx) = needs_in(900., 400., NeedsScope::All, jobs(), cx);
    assert!(find(cx, Role::List, "Jobs that need you").is_some(), "the cards stand in a list");
    assert!(find(cx, Role::Button, "Fix the merge line freshness chip, Plan to approve").is_some(), "a card names its Job and what it waits for");
    assert!(find(cx, Role::Button, "Flaky login test").is_some());
    assert!(find(cx, Role::Button, "Checkout page loads twice").is_none(), "a Job that failed is not one that needs him");
    assert!(find(cx, Role::Status, "Nothing needs you").is_none());
    let before = list.read_with(cx, |list, _| list.picks());
    press(cx, Role::Button, "Flaky login test");
    assert_eq!(list.read_with(cx, |list, _| list.picks()), before + 1, "a press on a card opens its Job in the list");
    assert_eq!(block.read_with(cx, |block, _| block.cards().len()), 2);
}

#[gpui::test]
fn the_scope_control_narrows_to_the_selected_project_and_back_and_says_so_to_its_host(cx: &mut TestAppContext) {
    let (block, _list, cx) = needs_in(900., 400., NeedsScope::Selected, jobs(), cx);
    block.update(cx, |block, cx| block.select_project(Some("storefront".into()), cx));
    redraw(cx);
    assert!(find(cx, Role::Button, "Flaky login test").is_some(), "the selected project's Job");
    assert!(find(cx, Role::Button, "Fix the merge line freshness chip, Plan to approve").is_none(), "and not Armada's");
    assert!(find(cx, Role::Button, "Needs you in the selected project").is_some() && find(cx, Role::Button, "Needs you in all projects").is_some(), "both segments are named");

    let told = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = told.clone();
    let _watch = cx.update(|_, cx| cx.subscribe(&block, move |_, event: &ScopeChanged, _| sink.borrow_mut().push(event.0)));
    press(cx, Role::Button, "Needs you in all projects");
    assert_eq!(block.read_with(cx, |block, _| block.scope()), NeedsScope::All);
    assert_eq!(*told.borrow(), [NeedsScope::All], "the host is told once, to keep it with the layout");
    assert!(find(cx, Role::Button, "Fix the merge line freshness chip, Plan to approve").is_some(), "every project's Jobs");
    assert!(find(cx, Role::Button, "Flaky login test").is_some());

    press(cx, Role::Button, "Needs you in all projects");
    assert_eq!(told.borrow().len(), 1, "a press on the scope already held says nothing");
    press(cx, Role::Button, "Needs you in the selected project");
    assert_eq!(*told.borrow(), [NeedsScope::All, NeedsScope::Selected]);
    assert!(find(cx, Role::Button, "Fix the merge line freshness chip, Plan to approve").is_none());
}

#[gpui::test]
fn the_scope_control_stands_on_one_line_with_one_height_and_one_baseline(cx: &mut TestAppContext) {
    let (_block, _list, cx) = needs_in(900., 400., NeedsScope::All, jobs(), cx);
    let selected = find(cx, Role::Button, "Needs you in the selected project").expect("the first segment");
    let all = find(cx, Role::Button, "Needs you in all projects").expect("the second segment");
    assert_eq!(selected.size.height, all.size.height, "the segments are one height");
    assert_eq!(selected.origin.y, all.origin.y, "and stand on one line");
    assert!(selected.right() <= all.left() + px(0.5), "side by side, not over each other");
}

#[gpui::test]
fn with_nobody_waiting_the_block_says_so_and_keeps_its_scope_control(cx: &mut TestAppContext) {
    let calm: Vec<SessionRecord> = jobs().into_iter().filter(|job| job.mark.is_none_or(|mark| mark.lamp != ipc::JobLamp::Needs)).collect();
    let (_block, _list, cx) = needs_in(900., 400., NeedsScope::All, calm, cx);
    assert!(find(cx, Role::Status, "Nothing needs you").is_some());
    assert!(find(cx, Role::List, "Jobs that need you").is_none());
    assert!(find(cx, Role::Button, "Needs you in all projects").is_some(), "the scope can still be changed");
}

#[gpui::test]
fn the_cards_wrap_to_the_width_of_the_box_and_never_leave_it(cx: &mut TestAppContext) {
    let (_block, _list, cx) = needs_in(900., 400., NeedsScope::All, jobs(), cx);
    let first = find(cx, Role::Button, "Fix the merge line freshness chip, Plan to approve").expect("first card");
    let second = find(cx, Role::Button, "Flaky login test").expect("second card");
    assert_eq!(first.origin.y, second.origin.y, "a wide box holds the cards side by side");
    assert!(second.right() <= px(900.) + px(0.5), "inside the box");

    let (_block, _list, cx) = needs_in(220., 400., NeedsScope::All, jobs(), cx);
    let first = find(cx, Role::Button, "Fix the merge line freshness chip, Plan to approve").expect("first card");
    let second = find(cx, Role::Button, "Flaky login test").expect("second card");
    assert!(second.origin.y > first.origin.y, "a narrow box stacks them");
    assert!(first.right() <= px(220.) + px(0.5) && second.right() <= px(220.) + px(0.5), "and neither is wider than the box");
}

// -- the placeholders -------------------------------------------------------------------------------------

#[gpui::test]
fn a_system_block_is_a_title_and_one_line_saying_when_it_arrives(cx: &mut TestAppContext) {
    for (kind, line) in [(HomeBlockKind::SystemOverview, "Arrives with the System overview"), (HomeBlockKind::SystemStats, "Arrives with System stats")] {
        ready(cx);
        let (_, cx) = cx.add_window_view(|_, cx| Boxed { block: cx.new(|_| PlaceholderBlock::new(kind)).into(), wide: 300., tall: 160. });
        redraw(cx);
        let title = find(cx, Role::Heading, kind.title()).unwrap_or_else(|| panic!("{} is drawn", kind.title()));
        let said = find(cx, Role::Status, line).unwrap_or_else(|| panic!("{line} is drawn"));
        assert!(said.origin.y > title.origin.y, "the line is under the title");
        assert!(said.right() <= px(300.) + px(0.5), "inside its box");
    }
}

// -- the Floor plan as a block ----------------------------------------------------------------------------

#[gpui::test]
fn the_plan_block_is_the_map_alone_with_no_header_no_state_chip_and_no_switch(cx: &mut TestAppContext) {
    let (_list, cx) = floor_in(900., 600., features_mock::floor_library(), cx);
    assert!(find(cx, Role::Region, "Feature map").is_some(), "the map is drawn");
    assert!(find(cx, Role::Heading, "Floor").is_none(), "with no Floor header");
    assert!(find(cx, Role::Status, "Operator off").is_none(), "no word that the Operator is off");
    assert!(find(cx, Role::Button, "Switch the Operator on").is_none(), "and no Switch on: that stays with the Cockpit");
    assert!(find(cx, Role::Region, "Floor").is_none(), "the grid names the block, not the plan");
    assert!(find(cx, Role::Button, "Armada").is_some() && find(cx, Role::Button, "Pocket").is_some(), "the project chips stay, as they choose which plan");
    assert!(find(cx, Role::Button, "Native Bridge foundation, Working").is_some(), "a Job on a feature is its chip");
}

#[gpui::test]
fn the_plan_block_without_features_draws_the_code_map_and_no_header(cx: &mut TestAppContext) {
    let library = ipc::FeatureLibrary { areas: Vec::new(), ..features_mock::library() };
    let (_list, cx) = floor_in(900., 600., library, cx);
    assert!(find(cx, Role::Region, "Code map").is_some());
    assert!(find(cx, Role::Region, "Feature map").is_none());
    assert!(find(cx, Role::Button, "Switch the Operator on").is_none());
}

#[gpui::test]
fn a_press_on_a_job_chip_in_the_plan_block_opens_that_job(cx: &mut TestAppContext) {
    let (list, cx) = floor_in(900., 600., features_mock::floor_library(), cx);
    let before = list.read_with(cx, |list, _| list.picks());
    press(cx, Role::Button, "Native Bridge foundation, Working");
    assert_eq!(list.read_with(cx, |list, _| list.picks()), before + 1);
}

#[gpui::test]
fn the_plan_block_lays_its_areas_out_by_its_own_width_not_the_windows(cx: &mut TestAppContext) {
    let library = features_mock::floor_library();
    let titles: Vec<String> = cards(&library, &[]).into_iter().map(|card| card.title).collect();
    assert!(titles.len() >= 2, "the fixture has areas enough to fill two columns");
    let (_list, cx) = floor_in(1000., 600., library.clone(), cx);
    let at = |cx: &mut VisualTestContext, title: &str| find(cx, Role::Group, title).unwrap_or_else(|| panic!("{title} is drawn"));
    let (a, b) = (at(cx, &titles[0]), at(cx, &titles[1]));
    assert_eq!(a.origin.y, b.origin.y, "a wide box puts two areas on a line");
    assert!(b.right() <= px(1000.) + px(0.5));

    let (_list, cx) = floor_in(260., 600., library, cx);
    let (a, b) = (at(cx, &titles[0]), at(cx, &titles[1]));
    assert!(b.origin.y > a.origin.y, "a narrow box stacks them");
    assert!(a.right() <= px(260.) + px(0.5) && b.right() <= px(260.) + px(0.5), "none wider than its box");
}
