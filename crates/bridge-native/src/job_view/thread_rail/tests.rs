use gpui::{prelude::*, px, Entity, Modifiers, Role, TestAppContext, VisualTestContext};
use ipc::{ArtifactForm, ArtifactState, Ask, Instant, JobStory, SessionId, StoryAsk, StoryBuilt, StoryShown, ThreadItem};

use super::model::{self, Bar, Item, Mark};
use super::Rail;
use crate::job_view::story::model::Kind;
use crate::job_view::story::Load;
use crate::job_view::kind_chip::{mock, store};
use crate::job_view::JobView;

const JOB: &str = "mock-02";
const NAME: &str = "Story";

fn items() -> Vec<ThreadItem> {
    crate::mock::threads("job-view").expect("the threads decode").remove(JOB).expect("the second Job has a thread").items
}

fn bar(key: &str, kind: Kind, title: &str, says: &str, when: &str, at: Option<usize>) -> Bar {
    Bar { item: Item { key: key.into(), kind, title: title.into(), says: says.into(), when: when.into() }, at }
}

// ---- the model -------------------------------------------------------------------------------

#[test]
fn the_ask_in_view_is_the_last_whose_first_row_is_at_or_above_the_scroll_top() {
    // Bars 1, 2, 4 and 5 are asks, the rest other items of the Story.
    let asks = [(1, 2), (2, 5), (4, 9), (5, 14)];
    assert_eq!(model::in_view(&asks, 0), None, "the top is above the first ask");
    assert_eq!(model::in_view(&asks, 1), None);
    assert_eq!(model::in_view(&asks, 2), Some(1), "a first row at the top is in view");
    assert_eq!(model::in_view(&asks, 4), Some(1));
    assert_eq!(model::in_view(&asks, 5), Some(2));
    assert_eq!(model::in_view(&asks, 13), Some(4));
    assert_eq!(model::in_view(&asks, 14), Some(5));
    assert_eq!(model::in_view(&asks, 400), Some(5), "below the last, the last is in view");
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

fn item(key: &str, kind: Kind, title: &str, says: &str) -> Item {
    Item { key: key.into(), kind, title: title.into(), says: says.into(), when: "2d ago".into() }
}

#[test]
fn there_is_a_bar_for_each_story_item_in_the_storys_order_and_an_ask_stands_where_the_thread_holds_it() {
    let items = vec![
        item("ask-ask-1", Kind::You, "Router", "Open"),
        item("shown-sketches/a", Kind::Artifact, "A sketch", "Approved"),
        item("ask-ask-2", Kind::You, "Retry", "Open"),
        item("built-3720", Kind::Pull, "Pull request #3720", "Merged"),
    ];
    let marks = [Mark { ask_id: "ask-1".into(), item: 7 }];
    let found = model::bars(items, &marks);
    assert_eq!(found.iter().map(|one| one.item.key.as_str()).collect::<Vec<_>>(), ["ask-ask-1", "shown-sketches/a", "ask-ask-2", "built-3720"], "the Story's order");
    assert_eq!(found.iter().map(|one| one.at).collect::<Vec<_>>(), [Some(7), None, None, None], "only an ask the thread holds has a place in it");
}

#[test]
fn a_bar_is_coloured_by_the_storys_kind_named_for_its_title_and_kind_and_a_row_says_what_and_when() {
    for kind in Kind::ALL {
        assert!(!kind.said().is_empty());
    }
    let pull = bar("built-3720", Kind::Pull, "Pull request #3720", "Merged", "3h ago", None);
    assert_eq!(model::bar_name(&pull), "Pull request #3720, Pull request");
    assert_eq!(model::row_name(&pull), "Pull request #3720, Merged, 3h ago");
    assert_eq!(model::meta(&pull), "Merged · 3h ago");
    let plain = bar("handed-1", Kind::Job, "Handed to Retry", "", "now", None);
    assert_eq!(model::row_name(&plain), "Handed to Retry, now", "a row that says nothing leaves no gap");
    assert_eq!(model::meta(&plain), "now");
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
    job_at(cx, 1400., height)
}

fn job_at(cx: &mut TestAppContext, width: f32, height: f32) -> (Entity<JobView>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::job_view::kind_chip::init(cx);
        crate::job_view::thread_rail::init(cx);
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
    cx.simulate_resize(gpui::size(px(width), px(height)));
    list.update(cx, |list, cx| list.open(JOB.to_string().into(), cx));
    cx.run_until_parked();
    cx.update(|_, cx| store::shown(JOB, cx));
    cx.run_until_parked();
    install_story(&view, true, cx);
    (view, cx)
}

fn at(day: u32) -> Instant {
    Instant::carried(format!("2026-11-{day:02}T12:00:00Z"))
}

/// The Job's Story as its tab holds it: the four asks of the kind chip's store, a sketch shown under the first and a pull
/// request merged for the last, so there are six items, the asks first. None where `held` is false.
fn install_story(view: &Entity<JobView>, held: bool, cx: &mut VisualTestContext) {
    let asks: Vec<Ask> = if held { mock::held(JOB).asks } else { Vec::new() };
    let mut told: Vec<StoryAsk> = asks.into_iter().map(|ask| StoryAsk { ask, shown: Vec::new(), built: Vec::new(), handed: Vec::new(), open: true }).collect();
    if let Some(first) = told.first_mut() {
        first.shown = vec![StoryShown { artifact: "sketches/router".into(), title: "Router sketch".into(), form: ArtifactForm::Sketch, state: ArtifactState::Approved, words: None, votes: Vec::new(), presses: Vec::new(), shown_at: at(20) }];
    }
    if let Some(last) = told.last_mut() {
        last.built = vec![StoryBuilt { pull_request: "3720".into(), merged_at: at(21) }];
    }
    let story = JobStory { session_id: SessionId::carried(JOB), asks: told, shown_otherwise: Vec::new(), lead: Vec::new(), features: Vec::new() };
    let tab = view.update_in(cx, |view, window, cx| view.story_of(JOB, window, cx));
    tab.update(cx, |tab, cx| {
        tab.story = Load::Read(story);
        tab.rebuild(cx);
    });
    cx.run_until_parked();
}

fn bounds(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| crate::job_view::head::drawn(window, cx, role, name))
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    bounds(cx, role, name).is_some()
}

const ROUTER: &str = "Look at the router, You";
const RETRY: &str = "Check the retry rules, You";
const AUDIT: &str = "The audit table builds on every call, You";
const CACHE: &str = "Rename the cache keys, You";
const SKETCH: &str = "Router sketch, Artifact";
const PULL: &str = "Pull request #3720, Pull request";

/// The full name of the one row of the list that starts with `title`.
fn row_of(cx: &mut VisualTestContext, title: &str) -> String {
    let rows = crate::job_view::parts_fold::drawn_like(cx, Role::ListBoxOption, title);
    assert_eq!(rows.len(), 1, "one row starts with {title}: {rows:?}");
    rows.into_iter().next().expect("one row")
}

/// The list's scroll top as an item of the thread, and where each ask's first row is as such.
fn scroll(view: &Entity<JobView>, cx: &mut VisualTestContext) -> (usize, Vec<usize>) {
    view.read_with(cx, |view, cx| {
        let id: gpui::SharedString = JOB.into();
        let page = view.pages.get(&id).expect("the Job was drawn");
        let (bars, _) = view.rail_marks(&id, cx);
        (page.list.logical_scroll_top().item_ix, bars.iter().filter_map(|bar| bar.at).map(|item| page.entry(item)).collect())
    })
}

/// The bars as (key, place in the thread) and the one in view, for a message.
fn dump(view: &Entity<JobView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, cx| {
        let (bars, now) = view.rail_marks(&JOB.into(), cx);
        format!("{:?} in view {now:?}", bars.iter().map(|bar| (bar.item.key.clone(), bar.at)).collect::<Vec<_>>())
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
fn the_rail_draws_a_bar_for_each_story_item_in_the_storys_order_named_for_the_item_and_its_kind(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx, 2400.);
    assert!(drawn(cx, Role::Region, NAME), "the rail is a region");
    let tops: Vec<f32> = [ROUTER, RETRY, AUDIT, CACHE, SKETCH, PULL].iter().map(|name| f32::from(bounds(cx, Role::Button, name).unwrap_or_else(|| panic!("{name} is a bar")).origin.y)).collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "the bars run down the edge in the Story's order: {tops:?}");
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
    assert_eq!(in_view(&view, cx), model::in_view(&entries.iter().copied().enumerate().collect::<Vec<_>>(), 0));
}

/// The rail's width with the thread scrolled so item `item` is at its top, and which ask that puts in view.
fn rail_wide_at(view: &Entity<JobView>, cx: &mut VisualTestContext, item: usize) -> (f32, Option<usize>) {
    view.read_with(cx, |view, _| view.pages.get(JOB).expect("drawn").list.scroll_to(gpui::ListOffset { item_ix: item, offset_in_item: px(0.) }));
    let wide = f32::from(bounds(cx, Role::Region, NAME).expect("the rail is drawn").size.width);
    (wide, in_view(view, cx))
}

#[gpui::test]
fn every_bar_is_one_length_so_the_bars_line_up_whichever_ask_is_in_view(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 500.);
    let (_, entries) = scroll(&view, cx);
    let (none, at_top) = rail_wide_at(&view, cx, 0);
    assert_eq!(at_top, None, "the thread starts before its first ask");
    for (at, entry) in entries.iter().enumerate() {
        let (wide, now) = rail_wide_at(&view, cx, *entry);
        assert_eq!(now, Some(at));
        assert_eq!(wide, none, "the bar of ask {at} in view is no longer than the rest, so both edges of the bars line up");
    }
}

#[gpui::test]
fn the_room_kept_for_the_rail_leaves_the_threads_cards_centred_with_the_composer(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx, 2400.);
    assert!(drawn(cx, Role::Region, NAME), "the rail is drawn, so the thread keeps room for it");
    let card = bounds(cx, Role::Group, "Agent message").expect("the thread holds an agent's message");
    let composer = bounds(cx, Role::TextInput, "Message").expect("the composer is drawn");
    let (card, composer) = (f32::from(card.center().x), f32::from(composer.center().x));
    assert!((card - composer).abs() <= 1., "a card in the thread stands on the composer's centre ({card} against {composer})");
}

#[gpui::test]
fn a_job_with_no_story_items_draws_no_rail_and_no_list(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 2400.);
    assert!(drawn(cx, Role::Region, NAME));
    install_story(&view, false, cx);
    view.update(cx, |_, cx| cx.notify());
    assert!(!drawn(cx, Role::Region, NAME), "no item, no rail");
    assert!(!drawn(cx, Role::Button, ROUTER));
    cx.simulate_keystrokes("alt-t");
    assert!(!drawn(cx, Role::ListBox, NAME), "the key does nothing where there is no rail");
}

#[gpui::test]
fn an_ask_whose_first_words_the_thread_does_not_hold_is_still_a_bar(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 2400.);
    cx.update(|_, cx| {
        let mut held = mock::held(JOB);
        held.asks.iter_mut().for_each(|ask| ask.first_row = format!("note:{}", ask.ask_id));
        store::hold(JOB, held, cx);
    });
    view.update(cx, |_, cx| cx.notify());
    assert!(drawn(cx, Role::Region, NAME), "the Story still has items");
    assert!(drawn(cx, Role::Button, ROUTER));
    assert_eq!(in_view(&view, cx), None, "and none of them is an ask in view");
}

#[gpui::test]
fn hovering_the_bars_opens_the_list_and_leaving_shuts_it(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx, 2400.);
    hover_rail(cx);
    assert!(drawn(cx, Role::ListBox, NAME), "the list is open beside the bars");
    for title in ["Look at the router, ", "Check the retry rules, ", "The audit table builds on every call, ", "Rename the cache keys, ", "Router sketch, You approved it, ", "Pull request #3720, Merged, "] {
        row_of(cx, title);
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
    let name = row_of(cx, "Look at the router, ");
    let at = bounds(cx, Role::ListBoxOption, &name).expect("the row").center();
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    let (after, _) = scroll(&view, cx);
    assert_eq!(after, first, "the first ask's first row is at the top");
    let said = dump(&view, cx);
    assert_eq!(in_view(&view, cx), Some(0), "so it is the ask in view: {said}");
}

#[gpui::test]
fn a_press_on_a_bar_scrolls_there_too(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 500.);
    let (_, entries) = scroll(&view, cx);
    let at = bounds(cx, Role::Button, RETRY).expect("the bar").center();
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(scroll(&view, cx).0, entries[1]);
    let said = dump(&view, cx);
    assert_eq!(in_view(&view, cx), Some(1), "{said}");
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
    for name in [ROUTER, RETRY, AUDIT, CACHE, SKETCH, PULL] {
        assert!(drawn(cx, Role::Button, name), "{name} is a button");
    }
    let rows = crate::job_view::parts_fold::drawn_like(cx, Role::ListBoxOption, "");
    assert_eq!(rows.len(), 6, "a row for each Story item, each an option");
    assert!(rows.iter().all(|name| name.contains(", ")), "each says its title and when: {rows:?}");
}

#[gpui::test]
fn the_thread_pads_by_the_rails_width_while_a_rail_is_drawn_and_not_otherwise(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 2400.);
    let room = |view: &Entity<JobView>, cx: &mut VisualTestContext| view.read_with(cx, |view, cx| view.rail_width(&JOB.into(), cx));
    assert_eq!(room(&view, cx), Some(px(super::RAIL_WIDTH)), "the Job has Story items");
    install_story(&view, false, cx);
    assert_eq!(room(&view, cx), None, "no item, no padding");
}

#[gpui::test]
fn a_thread_with_the_rail_is_narrower_by_its_width_on_each_side_and_no_card_sits_under_the_bars(cx: &mut TestAppContext) {
    // Narrow enough that the cards are as wide as the column lets them be.
    let (view, cx) = job_at(cx, 1000., 2400.);
    let card = bounds(cx, Role::Group, "Agent message").expect("a card is drawn");
    let bars = bounds(cx, Role::Region, NAME).expect("the rail is drawn");
    assert!(card.right() <= bars.left(), "the card ends at {:?} and the bars start at {:?}", card.right(), bars.left());
    install_story(&view, false, cx);
    view.update(cx, |_, cx| cx.notify());
    let bare = bounds(cx, Role::Group, "Agent message").expect("a card is drawn");
    assert_eq!(bare.size.width - card.size.width, px(super::RAIL_WIDTH * 2.), "the column gives up the rail's width on each side, so it keeps its centre, and no more");
}

#[gpui::test]
fn a_press_on_an_item_that_is_not_an_ask_opens_the_story_on_its_row_unfolded(cx: &mut TestAppContext) {
    let (view, cx) = job(cx, 500.);
    let (before, _) = scroll(&view, cx);
    let at = bounds(cx, Role::Button, SKETCH).expect("the bar").center();
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    let tab = view.update_in(cx, |view, window, cx| view.story_of(JOB, window, cx));
    assert_eq!(tab.read_with(cx, |tab, _| tab.unfolded.clone()).as_deref(), Some("shown-sketches/router"), "the sketch's row is open in the Story");
    assert_eq!(scroll(&view, cx).0, before, "and the thread did not move");
    // The pull request too.
    let at = bounds(cx, Role::Button, PULL).expect("the bar").center();
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(tab.read_with(cx, |tab, _| tab.unfolded.clone()).as_deref(), Some("built-3720"));
}
