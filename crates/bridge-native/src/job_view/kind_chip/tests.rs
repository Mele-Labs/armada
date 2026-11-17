use gpui::{prelude::*, Entity, Modifiers, Role, TestAppContext, VisualTestContext};
use ipc::{AskKind, AskState, Instant, MessageKind, MessageSorting, SessionId, SortedBy};

use super::model::{self, Choice, Hue};
use super::store;
use super::{mock, spot_key};
use crate::dropdown::Entry;
use crate::job_view::JobView;

const JOB: &str = "mock-02";
const FIX: &str = "m-331";
const BUG: &str = "m-326";
const TALK: &str = "m-321";
const OPENED: &str = "m-301";

fn sorting(kind: MessageKind, ask_kind: Option<AskKind>, by: SortedBy) -> MessageSorting {
    MessageSorting {
        session_id: SessionId::carried(JOB),
        row_id: "m-1".into(),
        kind,
        ask_kind,
        ask_id: None,
        title: None,
        sorted_by: by,
        sorted_at: Instant::carried("2026-11-17T10:00:00Z"),
    }
}

// ---- the model -------------------------------------------------------------------------------

#[test]
fn a_new_ask_says_its_ask_kind_and_the_rest_say_what_they_are_each_in_its_hue() {
    use AskKind::*;
    use MessageKind::*;
    let said: Vec<(&str, Hue)> = [
        sorting(NewAsk, Some(Bug), SortedBy::Fleet),
        sorting(NewAsk, Some(ChangeRequest), SortedBy::Fleet),
        sorting(NewAsk, Some(Idea), SortedBy::Fleet),
        sorting(NewAsk, Some(Fix), SortedBy::Fleet),
        sorting(NewAsk, None, SortedBy::Fleet),
        sorting(Revision, None, SortedBy::Fleet),
        sorting(Answer, None, SortedBy::Fleet),
        sorting(Talk, None, SortedBy::Fleet),
    ]
    .iter()
    .map(|one| (model::label(one), model::hue(one)))
    .collect();
    assert_eq!(
        said,
        [
            ("Bug", Hue::Bug),
            ("Change request", Hue::Change),
            ("Idea", Hue::Idea),
            ("Fix", Hue::Change),
            ("Change request", Hue::Change),
            ("Revision", Hue::Neutral),
            ("Answer", Hue::Neutral),
            ("Talk", Hue::Neutral),
        ],
        "a new ask with no kind reads as a change request, as Fleet takes it"
    );
}

#[test]
fn the_tooltip_says_who_sorted_the_message() {
    assert_eq!(model::sorted_by(&sorting(MessageKind::Talk, None, SortedBy::Fleet)), "Sorted by Fleet");
    assert_eq!(model::sorted_by(&sorting(MessageKind::Talk, None, SortedBy::Owner)), "Sorted by you");
}

fn rows(entries: &[Entry]) -> Vec<(String, Option<bool>, Option<String>)> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Row(row) => Some((row.label.to_string(), row.ticked, row.off.as_ref().map(|why| why.to_string()))),
            _ => None,
        })
        .collect()
}

#[test]
fn the_first_menu_lists_the_seven_kinds_with_the_current_one_marked() {
    let held = mock::held(JOB);
    let current = held.sortings.iter().find(|one| one.row_id == FIX).expect("sorted");
    let said = rows(&model::kind_entries(current, &held.asks));
    let labels: Vec<&str> = said.iter().map(|(label, _, _)| label.as_str()).collect();
    assert_eq!(labels, ["Bug", "Change request", "Idea", "Fix", "Revision", "Answer", "Talk"]);
    assert_eq!(said.iter().filter(|(_, ticked, _)| *ticked == Some(true)).map(|(label, _, _)| label.as_str()).collect::<Vec<_>>(), ["Fix"], "one is marked");
    assert!(said.iter().all(|(_, _, off)| off.is_none()), "the Job holds open asks, so every line can be picked");
}

#[test]
fn revision_and_answer_are_off_where_the_job_holds_no_open_ask_and_say_why() {
    let mut held = mock::held(JOB);
    held.asks.iter_mut().for_each(|ask| ask.state = AskState::Built);
    let said = rows(&model::kind_entries(&held.sortings[0], &held.asks));
    assert_eq!(said.iter().filter(|(_, _, why)| why.is_some()).map(|(label, _, _)| label.as_str()).collect::<Vec<_>>(), ["Revision", "Answer"]);
    assert!(said.iter().filter_map(|(_, _, why)| why.as_deref()).all(|why| why == "No open ask in this Job"));
}

#[test]
fn the_second_menu_lists_the_open_asks_by_title_with_their_kind_and_the_one_joined_marked() {
    let held = mock::held(JOB);
    let joined = held.sortings.iter().find(|one| one.row_id == "m-311").expect("a revision");
    let said: Vec<(String, String, Option<bool>)> = model::ask_entries(joined, &held.asks)
        .iter()
        .filter_map(|entry| match entry {
            Entry::Row(row) => Some((row.label.to_string(), row.detail.as_ref().map(|detail| detail.to_string()).unwrap_or_default(), row.ticked)),
            _ => None,
        })
        .collect();
    assert_eq!(
        said,
        [
            ("Look at the router".to_string(), "Change request".to_string(), Some(true)),
            ("Check the retry rules".to_string(), "Idea".to_string(), Some(false)),
            ("The audit table builds on every call".to_string(), "Bug".to_string(), Some(false)),
        ],
        "the built ask is not open, so it is not offered"
    );
    assert_eq!(model::ask_picked(&held.asks, "ask:1").map(|ask| ask.ask_id.as_str()), Some("ask-2"));
    assert!(model::ask_picked(&held.asks, "ask:9").is_none());
}

#[test]
fn each_pick_sends_its_own_request_and_a_join_without_an_ask_sends_nothing() {
    let sent: Vec<(MessageKind, Option<AskKind>, Option<String>)> = [
        (Choice::Ask(AskKind::Bug), None),
        (Choice::Ask(AskKind::ChangeRequest), None),
        (Choice::Ask(AskKind::Idea), None),
        (Choice::Ask(AskKind::Fix), None),
        (Choice::Talk, None),
        (Choice::Revision, Some("ask-2")),
        (Choice::Answer, Some("ask-1")),
    ]
    .into_iter()
    .map(|(choice, ask)| {
        let set = model::request(JOB, "m-9", choice, ask).expect("a request");
        assert_eq!((set.session_id.as_str(), set.row_id.as_str()), (JOB, "m-9"));
        (set.kind, set.ask_kind, set.ask_id)
    })
    .collect();
    assert_eq!(
        sent,
        [
            (MessageKind::NewAsk, Some(AskKind::Bug), None),
            (MessageKind::NewAsk, Some(AskKind::ChangeRequest), None),
            (MessageKind::NewAsk, Some(AskKind::Idea), None),
            (MessageKind::NewAsk, Some(AskKind::Fix), None),
            (MessageKind::Talk, None, None),
            (MessageKind::Revision, None, Some("ask-2".to_string())),
            (MessageKind::Answer, None, Some("ask-1".to_string())),
        ]
    );
    assert!(model::request(JOB, "m-9", Choice::Revision, None).is_none());
    assert!(model::request(JOB, "m-9", Choice::Answer, None).is_none());
}

#[test]
fn the_mock_refuses_in_fleets_words_an_ask_that_holds_revisions_becoming_something_else() {
    let mut held = mock::held(JOB);
    let set = model::request(JOB, OPENED, Choice::Talk, None).expect("a request");
    let why = mock::apply(&mut held, &set).expect_err("ask-1 holds a revision");
    assert_eq!(why.code.as_deref(), Some("fleet.ask_has_words"));
    assert_eq!(why.said, "the ask this message opened holds revisions or answers, so it cannot become something else");
    assert_eq!(held, mock::held(JOB), "nothing was kept");
    let set = model::request(JOB, OPENED, Choice::Ask(AskKind::Idea), None).expect("a request");
    assert!(mock::apply(&mut held, &set).is_ok(), "a new ask may change its own kind");
    assert_eq!(held.asks[0].kind, AskKind::Idea);
}

// ---- the thread ------------------------------------------------------------------------------

/// The window's root clears the label book each frame, as the app's does, so what is no longer drawn is gone from it.
struct Root(Entity<JobView>);

impl gpui::Render for Root {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        crate::labels::label_root(gpui::div().size_full().child(self.0.clone()))
    }
}

fn job(cx: &mut TestAppContext) -> (Entity<JobView>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        super::init(cx);
    });
    let jobs = crate::mock::jobs("job-view").expect("the fixture decodes");
    let threads = crate::mock::threads("job-view").expect("the threads decode");
    let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
    let (root, cx) = cx.add_window_view({
        let list = list.clone();
        move |window, cx| Root(cx.new(|cx| JobView::new(list, Some(threads), window, cx)))
    });
    let view = root.read_with(cx, |root, _| root.0.clone());
    // The window the keys go to is the one in front.
    cx.update(|window, _| window.activate_window());
    // Tall enough that the last four of his messages are all drawn.
    cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(2400.)));
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

fn press(cx: &mut VisualTestContext, role: Role, name: &str) {
    let at = bounds(cx, role, name).unwrap_or_else(|| panic!("{name} is drawn")).center();
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
}

fn kept(cx: &mut VisualTestContext, row: &str) -> Option<MessageSorting> {
    cx.update(|_, cx| store::sorting(JOB, row, cx))
}

/// A key pressed and let go, which is what opens a menu from a focused chip: gpui clicks on the release of Enter and Space.
fn tap(cx: &mut VisualTestContext, key: &'static str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent { keystroke: gpui::Keystroke::parse(key).expect("a key") });
    cx.run_until_parked();
}

fn focus_chip(view: &Entity<JobView>, row: &str, cx: &mut VisualTestContext) {
    let spot = view.read_with(cx, |view, _| view.pages.get(JOB).and_then(|page| page.keys.spot_of(&spot_key(row)))).expect("the chip was drawn");
    cx.update(|window, cx| window.focus(&spot.focus, cx));
    cx.run_until_parked();
}

#[gpui::test]
fn each_sorted_message_of_his_draws_a_chip_named_for_its_kind_and_an_unsorted_one_draws_none(cx: &mut TestAppContext) {
    let (view, cx) = job(cx);
    assert!(drawn(cx, Role::Button, "Message kind: Fix"), "the newest message is a new ask of kind fix");
    assert!(drawn(cx, Role::Button, "Message kind: Bug"));
    assert!(drawn(cx, Role::Button, "Message kind: Talk"));
    // Fleet has not sorted his newest message: the thread shows nothing under it.
    cx.update(|_, cx| {
        let mut held = mock::held(JOB);
        held.sortings.retain(|one| one.row_id != FIX);
        store::hold(JOB, held, cx);
    });
    view.update(cx, |_, cx| cx.notify());
    assert!(!drawn(cx, Role::Button, "Message kind: Fix"), "no sorting, no chip");
    assert!(drawn(cx, Role::Button, "Message kind: Bug"));
}

#[gpui::test]
fn a_press_opens_the_menu_with_the_current_kind_marked_and_a_pick_sends_it_and_the_chip_follows(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx);
    press(cx, Role::Button, "Message kind: Fix");
    assert!(drawn(cx, Role::Menu, "Message kind"), "one press opens it");
    for name in ["Bug", "Change request", "Idea", "Fix", "Revision", "Answer", "Talk"] {
        assert!(drawn(cx, Role::MenuItemCheckBox, name), "{name} is a line");
    }
    press(cx, Role::MenuItemCheckBox, "Idea");
    let now = kept(cx, FIX).expect("sorted");
    assert_eq!((now.kind, now.ask_kind, now.sorted_by), (MessageKind::NewAsk, Some(AskKind::Idea), SortedBy::Owner));
    assert!(!drawn(cx, Role::Menu, "Message kind"), "the pick shuts the menu");
    assert!(drawn(cx, Role::Button, "Message kind: Idea"), "the chip follows what Fleet kept");
    assert!(!drawn(cx, Role::Button, "Message kind: Fix"));
}

#[gpui::test]
fn talk_is_sent_as_talk_and_a_revision_goes_on_to_the_asks_the_job_holds(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx);
    press(cx, Role::Button, "Message kind: Bug");
    press(cx, Role::MenuItemCheckBox, "Talk");
    assert_eq!(kept(cx, BUG).map(|one| (one.kind, one.ask_id)), Some((MessageKind::Talk, None)), "talk joins no ask");
    // The talk of the message before it is now sorted a revision.
    press(cx, Role::Button, "Message kind: Talk");
    let before = kept(cx, TALK);
    press(cx, Role::MenuItemCheckBox, "Revision");
    assert!(drawn(cx, Role::Menu, "Revision of"), "a second step lists the open asks");
    assert_eq!(kept(cx, TALK), before, "nothing is sent until an ask is picked");
    press(cx, Role::MenuItemCheckBox, "Look at the router");
    let now = kept(cx, TALK).expect("sorted");
    assert_eq!((now.kind, now.ask_id.as_deref()), (MessageKind::Revision, Some("ask-1")));
}

#[gpui::test]
fn an_answer_joins_the_ask_picked(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx);
    press(cx, Role::Button, "Message kind: Talk");
    press(cx, Role::MenuItemCheckBox, "Answer");
    assert!(drawn(cx, Role::Menu, "Answer to"));
    press(cx, Role::MenuItemCheckBox, "Check the retry rules");
    let now = kept(cx, TALK).expect("sorted");
    assert_eq!((now.kind, now.ask_id.as_deref(), now.sorted_by), (MessageKind::Answer, Some("ask-2"), SortedBy::Owner));
}

#[gpui::test]
fn with_no_open_ask_revision_and_answer_cannot_be_picked(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx);
    cx.update(|_, cx| {
        let mut held = mock::held(JOB);
        held.asks.clear();
        store::hold(JOB, held, cx);
    });
    press(cx, Role::Button, "Message kind: Talk");
    let before = kept(cx, TALK);
    press(cx, Role::MenuItemCheckBox, "Revision");
    assert!(!drawn(cx, Role::Menu, "Revision of"), "there is no second step to go on to");
    assert_eq!(kept(cx, TALK), before);
}

#[gpui::test]
fn fleets_refusal_reaches_a_toast_and_the_chip_stays_as_it_was(cx: &mut TestAppContext) {
    let (_view, cx) = job(cx);
    let before = kept(cx, OPENED);
    cx.update(|_, cx| store::send(JOB, model::request(JOB, OPENED, Choice::Talk, None).expect("a request"), None, cx));
    cx.run_until_parked();
    assert_eq!(store::told_in_tests(cx), ["the ask this message opened holds revisions or answers, so it cannot become something else"], "Fleet's own words");
    assert_eq!(kept(cx, OPENED), before);
}

#[gpui::test]
fn the_keys_open_the_menu_from_the_chip_move_pick_and_put_it_away(cx: &mut TestAppContext) {
    let (view, cx) = job(cx);
    focus_chip(&view, FIX, cx);
    tap(cx, "enter");
    assert!(drawn(cx, Role::Menu, "Message kind"), "Enter on the chip opens its menu");
    cx.simulate_keystrokes("escape");
    assert!(!drawn(cx, Role::Menu, "Message kind"), "Esc puts it away");
    focus_chip(&view, FIX, cx);
    tap(cx, "space");
    assert!(drawn(cx, Role::Menu, "Message kind"), "so does Space");
    assert!(!drawn(cx, Role::MenuItem, "Mod") && !drawn(cx, Role::MenuItem, "Report"), "it is the chip's own menu, which carries nothing of a context menu");
    cx.simulate_keystrokes("down down enter");
    let now = kept(cx, FIX).expect("sorted");
    assert_eq!((now.kind, now.ask_kind), (MessageKind::NewAsk, Some(AskKind::ChangeRequest)), "the first arrow lands on the first line, the second on the next, Enter picked it");
}

#[gpui::test]
fn the_registered_act_opens_the_menu_of_the_chip_that_holds_focus(cx: &mut TestAppContext) {
    let (view, cx) = job(cx);
    focus_chip(&view, TALK, cx);
    cx.simulate_keystrokes("alt-k");
    assert!(drawn(cx, Role::Menu, "Message kind"));
    cx.simulate_keystrokes("escape");
    assert!(crate::generated::actions::act("message_kind").is_some_and(|act| act.shortcut == "⌥K"));
}
