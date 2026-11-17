use gpui::{prelude::*, px, Entity, Modifiers, Role, TestAppContext, VisualTestContext};
use ipc::{Ask, AskKind, AskState, ThreadItem};

use super::model::{self, Mark};
use super::Rail;
use crate::job_view::kind_chip::model::Hue;
use crate::job_view::kind_chip::{mock, store};
use crate::job_view::JobView;

const JOB: &str = "mock-02";
const NAME: &str = "Asks in this thread";

fn items() -> Vec<ThreadItem> {
    crate::mock::threads("job-view").expect("the threads decode").remove(JOB).expect("the second Job has a thread").items
}

fn mark(id: &str, kind: AskKind, state: AskState, item: usize) -> Mark {
    Mark { ask_id: id.into(), title: format!("Ask {id}"), kind, state, item }
}

// ---- the model -------------------------------------------------------------------------------

#[test]
fn the_ask_in_view_is_the_last_whose_first_row_is_at_or_above_the_scroll_top() {
    let entries = [2, 5, 9, 14];
    assert_eq!(model::in_view(&entries, 0), None, "the top is above the first ask");
    assert_eq!(model::in_view(&entries, 1), None);
    assert_eq!(model::in_view(&entries, 2), Some(0), "a first row at the top is in view");
    assert_eq!(model::in_view(&entries, 4), Some(0));
    assert_eq!(model::in_view(&entries, 5), Some(1));
    assert_eq!(model::in_view(&entries, 13), Some(2));
    assert_eq!(model::in_view(&entries, 14), Some(3));
    assert_eq!(model::in_view(&entries, 400), Some(3), "below the last, the last is in view");
    assert_eq!(model::in_view(&[], 3), None, "no asks, none in view");
}

#[test]
fn the_asks_are_listed_in_thread_order_and_one_with_no_first_row_in_the_thread_is_left_out() {
    let items = items();
    let held = mock::held(JOB);
    // Fleet gives the open ones first: the order here is the opposite of the thread's.
    let mut asks: Vec<Ask> = held.asks.iter().rev().cloned().collect();
    let mut stray = asks[0].clone();
    stray.ask_id = "ask-elsewhere".into();
    stray.first_row = "note:somewhere-else".into();
    asks.insert(1, stray);
    let found = model::marks(&asks, &items);
    assert_eq!(found.iter().map(|one| one.ask_id.as_str()).collect::<Vec<_>>(), ["ask-1", "ask-2", "ask-3", "ask-4"], "thread order, and the stray one is not listed");
    assert!(found.windows(2).all(|pair| pair[0].item < pair[1].item), "each stands later in the thread than the one before");
}

#[test]
fn each_kind_wears_its_hue_and_each_state_its_word() {
    assert_eq!(model::hue(AskKind::Bug), Hue::Bug);
    assert_eq!(model::hue(AskKind::ChangeRequest), Hue::Change);
    assert_eq!(model::hue(AskKind::Fix), Hue::Change, "a fix is read as a change request");
    assert_eq!(model::hue(AskKind::Idea), Hue::Idea);
    let said: Vec<String> = [
        mark("a", AskKind::Bug, AskState::Built, 0),
        mark("b", AskKind::ChangeRequest, AskState::Building, 1),
        mark("c", AskKind::Idea, AskState::Handed, 2),
        mark("d", AskKind::Bug, AskState::Backlog, 3),
        mark("e", AskKind::Fix, AskState::Dropped, 4),
    ]
    .iter()
    .map(model::meta)
    .collect();
    assert_eq!(said, ["Bug, fixed", "Change request, open", "Idea, handed on", "Bug, backlog", "Fix, dropped"]);
}

#[test]
fn a_bar_is_named_for_its_ask_and_kind_and_the_keys_stay_inside_the_list() {
    let one = mark("a", AskKind::ChangeRequest, AskState::Building, 0);
    assert_eq!(model::bar_name(&one), "Ask a, Change request");
    assert_eq!(model::row_name(&one), "Ask a, Change request, open");
    assert_eq!((model::step(0, 4, false), model::step(0, 4, true), model::step(3, 4, true), model::step(3, 4, false)), (0, 1, 3, 2));
    assert_eq!(model::step(0, 0, true), 0);
}

#[test]
fn no_key_the_rail_takes_is_option_up_or_option_down() {
    let act = crate::generated::actions::act("thread_rail").expect("the act is registered");
    assert_eq!((act.shortcut, act.verb), ("⌥T", "Open the asks of this thread"));
    assert!(!act.shortcut.contains('↑') && !act.shortcut.contains('↓'));
    let taking_those: Vec<&str> = crate::generated::actions::ACTIONS.iter().filter(|one| one.shortcut.contains("⌥↑") || one.shortcut.contains("⌥↓")).map(|one| one.id).collect();
    assert!(!taking_those.contains(&"thread_rail"), "those two keys are another act's");
    let same: Vec<&str> = crate::generated::actions::ACTIONS.iter().filter(|one| one.shortcut == "⌥T").map(|one| one.id).collect();
    assert_eq!(same, ["thread_rail"], "no other act has the chord");
}

// ---- the thread ------------------------------------------------------------------------------

/// The window's root clears the label book each frame, as the app's does, so what is no longer drawn is gone from it.
struct Root(Entity<JobView>);

impl gpui::Render for Root {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        crate::labels::label_root(gpui::div().size_full().child(self.0.clone()))
    }
}

fn job(cx: &mut TestAppContext, height: f32) -> (Entity<JobView>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::job_view::kind_chip::init(cx);
    });
    let jobs = crate::mock::jobs("job-view").expect("the fixture decodes");
    let threads = crate::mock::threads("job-view").expect("the threads decode");
    let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
    let (root, cx) = cx.add_window_view({
        let list = list.clone();
        move |window, cx| Root(cx.new(|cx| JobView::new(list, Some(threads), window, cx)))
    });
    let view = root.read_with(cx, |root, _| root.0.clone());
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(gpui::size(px(1400.), px(height)));
    list.update(cx, |list, cx| list.open(JOB.to_string().into(), cx));
    cx.run_until_parked();
    cx.update(|_, cx| store::shown(JOB, cx));
    cx.run_until_parked();
    (view, cx)
}

fn bounds(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| crate::job_view::head::drawn(window, cx, role, name))
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    bounds(cx, role, name).is_some()
}

const ROUTER: &str = "Look at the router, Change request";
const RETRY: &str = "Check the retry rules, Idea";
const AUDIT: &str = "The audit table builds on every call, Bug";
const CACHE: &str = "Rename the cache keys, Fix";

/// The list's scroll top as an item of the thread, and where each ask's first row is as such.
fn scroll(view: &Entity<JobView>, cx: &mut VisualTestContext) -> (usize, Vec<usize>) {
    view.read_with(cx, |view, cx| {
        let id: gpui::SharedString = JOB.into();
        let page = view.pages.get(&id).expect("the Job was drawn");
        let (marks, _) = view.rail_marks(&id, cx);
        (page.list.logical_scroll_top().item_ix, marks.iter().map(|mark| page.entry(mark.item)).collect())
    })
}

fn in_view(view: &Entity<JobView>, cx: &mut VisualTestContext) -> Option<usize> {
    view.read_with(cx, |view, cx| view.rail_marks(&JOB.into(), cx).1)
}

fn hover_rail(cx: &mut VisualTestContext) {
    let at = bounds(cx, Role::Button, AUDIT).expect("the bar is drawn").center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.run_until_parked();
}

#[gpui::test]
fn the_rail_draws_a_bar_for_each_ask_in_thread_order_named_for_the_ask_and_its_kind(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx, 2400.);
    assert!(drawn(cx, Role::Region, NAME), "the rail is a region");
    let tops: Vec<f32> = [ROUTER, RETRY, AUDIT, CACHE].iter().map(|name| f32::from(bounds(cx, Role::Button, name).unwrap_or_else(|| panic!("{name} is a bar")).origin.y)).collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "the bars run down the edge in the order the asks come: {tops:?}");
    assert!(!drawn(cx, Role::ListBox, NAME), "the list is shut while nothing is on the rail");
}

#[gpui::test]
fn the_ask_in_view_follows_the_scroll_top_of_the_thread(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 500.);
    let (_, entries) = scroll(&view, cx);
    for (at, entry) in entries.iter().enumerate() {
        view.read_with(cx, |view, _| view.pages.get(JOB).expect("drawn").list.scroll_to(gpui::ListOffset { item_ix: *entry, offset_in_item: px(0.) }));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(in_view(&view, cx), Some(at), "with ask {at}'s first row at the top, it is the one in view");
    }
    view.read_with(cx, |view, _| view.pages.get(JOB).expect("drawn").list.scroll_to(gpui::ListOffset { item_ix: 0, offset_in_item: px(0.) }));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(in_view(&view, cx), model::in_view(&entries, 0));
}

#[gpui::test]
fn a_thread_with_no_asks_draws_no_rail_and_no_list(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 2400.);
    assert!(drawn(cx, Role::Region, NAME));
    cx.update(|_, cx| {
        let mut held = mock::held(JOB);
        held.asks.clear();
        store::hold(JOB, held, cx);
    });
    view.update(cx, |_, cx| cx.notify());
    assert!(!drawn(cx, Role::Region, NAME), "no ask, no rail");
    assert!(!drawn(cx, Role::Button, ROUTER));
    // An ask whose first words are not in this thread is no bar either.
    cx.update(|_, cx| {
        let mut held = mock::held(JOB);
        held.asks.iter_mut().for_each(|ask| ask.first_row = format!("note:{}", ask.ask_id));
        store::hold(JOB, held, cx);
    });
    view.update(cx, |_, cx| cx.notify());
    assert!(!drawn(cx, Role::Region, NAME));
    cx.simulate_keystrokes("alt-t");
    assert!(!drawn(cx, Role::ListBox, NAME), "the key does nothing where there is no rail");
}

#[gpui::test]
fn hovering_the_bars_opens_the_list_and_leaving_shuts_it(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx, 2400.);
    hover_rail(cx);
    assert!(drawn(cx, Role::ListBox, NAME), "the list is open beside the bars");
    for name in ["Look at the router, Change request, open", "Check the retry rules, Idea, backlog", "The audit table builds on every call, Bug, open", "Rename the cache keys, Fix, fixed"] {
        assert!(drawn(cx, Role::ListBoxOption, name), "{name} is a row");
    }
    let list = bounds(cx, Role::ListBox, NAME).expect("the list");
    let rail = bounds(cx, Role::Region, NAME).expect("the rail");
    assert!(list.right() <= rail.left() + px(1.), "it stands beside the bars, not over them");
    cx.simulate_mouse_move(gpui::point(px(40.), px(40.)), None, Modifiers::none());
    cx.run_until_parked();
    assert!(!drawn(cx, Role::ListBox, NAME), "the pointer left");
}

#[gpui::test]
fn a_press_on_a_row_scrolls_the_thread_to_where_the_ask_started(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 500.);
    let (before, entries) = scroll(&view, cx);
    assert!(entries.len() == 4 && entries.windows(2).all(|pair| pair[0] < pair[1]));
    let first = entries[0];
    assert!(before != first, "the thread opens at its end, not at the first ask");
    hover_rail(cx);
    let at = bounds(cx, Role::ListBoxOption, "Look at the router, Change request, open").expect("the row").center();
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    let (after, _) = scroll(&view, cx);
    assert_eq!(after, first, "the first ask's first row is at the top");
    assert_eq!(in_view(&view, cx), Some(0), "so it is the ask in view");
}

#[gpui::test]
fn a_press_on_a_bar_scrolls_there_too(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 500.);
    let (_, entries) = scroll(&view, cx);
    let at = bounds(cx, Role::Button, RETRY).expect("the bar").center();
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(scroll(&view, cx).0, entries[1]);
    assert_eq!(in_view(&view, cx), Some(1));
}

#[gpui::test]
fn the_key_opens_the_list_on_the_ask_in_view_and_the_arrows_move_enter_jumps_and_esc_puts_it_away(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 500.);
    let (_, entries) = scroll(&view, cx);
    let thread = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&thread, cx));
    cx.simulate_keystrokes("alt-t");
    assert!(drawn(cx, Role::ListBox, NAME), "⌥T opens the list");
    let start = cx.update(|_, cx| cx.global::<Rail>().cursor).expect("the keys are on the list");
    assert_eq!(Some(start), in_view(&view, cx), "they start on the ask in view");
    cx.simulate_keystrokes("up up up up");
    assert_eq!(cx.update(|_, cx| cx.global::<Rail>().cursor), Some(0), "Up stops at the first");
    cx.simulate_keystrokes("down");
    assert_eq!(cx.update(|_, cx| cx.global::<Rail>().cursor), Some(1));
    cx.simulate_keystrokes("alt-down");
    assert_eq!(cx.update(|_, cx| cx.global::<Rail>().cursor), Some(1), "⌥↓ is not the list's");
    cx.simulate_keystrokes("enter");
    assert_eq!(scroll(&view, cx).0, entries[1], "Enter scrolls to the ask the keys were on");
    assert!(!drawn(cx, Role::ListBox, NAME), "and puts the list away");
    cx.simulate_keystrokes("alt-t");
    assert!(drawn(cx, Role::ListBox, NAME));
    cx.simulate_keystrokes("escape");
    assert!(!drawn(cx, Role::ListBox, NAME), "Esc puts it away");
    assert_eq!(scroll(&view, cx).0, entries[1], "and moves nothing");
    assert!(cx.update(|window, _| thread.is_focused(window)), "the keys are the thread's again");
}

#[gpui::test]
fn every_part_has_a_role_and_a_name(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx, 2400.);
    cx.simulate_keystrokes("alt-t");
    assert!(drawn(cx, Role::Region, NAME));
    assert!(drawn(cx, Role::ListBox, NAME));
    for name in [ROUTER, RETRY, AUDIT, CACHE] {
        assert!(drawn(cx, Role::Button, name), "{name} is a button");
    }
    let rows = crate::job_view::parts_fold::drawn_like(cx, Role::ListBoxOption, "");
    assert_eq!(rows.len(), 4, "a row for each ask, each an option");
    assert!(rows.iter().all(|name| name.contains(", ")), "each says its title and its kind and state: {rows:?}");
}
