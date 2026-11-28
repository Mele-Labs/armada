use std::rc::Rc;

use gpui::{Entity, Modifiers, Role, TestAppContext, VisualTestContext};
use serde_json::{json, Value};

use super::slugs_of;
use crate::annotate::Layer;
use crate::features::overlay::Source;

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
    });
}

/// A note as the Electron layer writes it: where the JSX that drew the element is, and its component.
fn note(id: &str, file: Option<&str>, component: Option<&str>) -> Value {
    let mut raw = json!({
        "id": id, "status": "open", "text": "The row clips its title", "component": component, "owners": [], "ownersFrom": "parent",
        "selector": "div.row", "element": { "tag": "div", "text": "", "label": null }, "screen": "Jobs", "layer": null,
        "location": "/", "scenario": null, "box": { "x": 120, "y": 140, "width": 160, "height": 32 }, "window": { "width": 1000, "height": 700 },
        "createdAt": "2026-11-17T10:00:00.000Z", "updatedAt": "2026-11-17T10:00:00.000Z",
    });
    if let Some(file) = file {
        raw["source"] = json!({ "file": file, "line": 12 });
    }
    raw
}

const TWO: &str = "apps/desktop/src/renderer/src/jobs/JobRow.tsx";

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
}

/// The layer on, over a window with the mock's features, its first note's card open.
fn open(cx: &mut TestAppContext, notes: Vec<Value>) -> (Entity<Layer>, &mut VisualTestContext) {
    ready(cx);
    let (layer, cx) = cx.add_window_view(|_, cx| {
        let mut layer = Layer::new(Rc::default(), Vec::new(), cx);
        layer.screen = Some("Jobs".into());
        layer
    });
    cx.update(|window, cx| {
        layer.update(cx, |layer, cx| {
            layer.feature_source(Source::Mock, cx);
            layer.toggle(window, cx);
            layer.read_back(notes, cx);
            layer.open_note(1, window, cx);
        })
    });
    settle(cx);
    (layer, cx)
}

fn bounds_of(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|drawn| drawn.bounds))
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    bounds_of(cx, role, name).is_some()
}

fn focus_card(layer: &Entity<Layer>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let card = layer.read(cx).card_focus.clone();
        window.focus(&card, cx);
    });
    cx.run_until_parked();
}

fn beside(layer: &Entity<Layer>, cx: &mut VisualTestContext) -> Option<String> {
    layer.read_with(cx, |layer, cx| layer.beside_slug(cx))
}

#[gpui::test]
fn a_note_with_two_features_draws_a_chip_for_each(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, vec![note("a", Some(TWO), None)]);
    assert!(drawn(cx, Role::Button, "Feature: Job list and live status"));
    assert!(drawn(cx, Role::Button, "Feature: Pause, resume, stop, snooze"));
    assert!(!drawn(cx, Role::Image, "Loading"), "the read is done");
}

/// A note made on an open Job's view: no feature owns the folder that draws it, so the note names the Job's own feature.
#[gpui::test]
fn a_note_naming_the_open_jobs_feature_sends_to_that_features_jobs(cx: &mut TestAppContext) {
    let mut raw = note("on-a-job", None, None);
    raw["paths"] = json!(["crates/bridge-native/src/job_view"]);
    raw["features"] = json!([{ "project": crate::features::mock::PROJECT, "slug": "start-a-job", "title": "Start a Job", "area": "Running a Job" }]);
    let (layer, cx) = open(cx, vec![raw]);
    assert!(drawn(cx, Role::Button, "Start a Job's Jobs"), "the card's send names the feature");
    let feature = layer.read_with(cx, |layer, cx| layer.feature_of(1, cx)).expect("the note maps to the feature it names");
    assert_eq!(feature.jobs.iter().map(|job| job.title.as_str()).collect::<Vec<_>>(), ["Add the model picker to the start card"], "the Jobs on it now, as the library has them");
}

#[gpui::test]
fn a_note_that_maps_to_no_feature_draws_no_chip_and_no_bones(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("no-feature", Some("apps/desktop/src/renderer/src/elsewhere/Zebra.tsx"), Some("Zebra"))]);
    assert!(!drawn(cx, Role::Button, "Feature: Job list and live status"));
    assert!(!drawn(cx, Role::Image, "Loading"));
    assert_eq!(layer.read_with(cx, |layer, cx| layer.beside.read(cx).reads.get("no-feature").map(|read| matches!(read, super::Read::Done(chips) if chips.is_empty()))), Some(true), "it was read, and maps to none");
    assert!(slugs_of("no-feature").is_empty());
}

#[gpui::test]
fn where_no_file_maps_the_component_names_the_feature(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, vec![note("by-component", None, Some("ThreadComposerView"))]);
    assert!(drawn(cx, Role::Button, "Feature: Thread and composer"));
    assert_eq!(slugs_of("by-component"), ["thread-and-composer"]);
}

#[gpui::test]
fn a_note_is_read_when_its_card_is_shown_and_not_before(cx: &mut TestAppContext) {
    ready(cx);
    let (layer, cx) = cx.add_window_view(|_, cx| {
        let mut layer = Layer::new(Rc::default(), Vec::new(), cx);
        layer.screen = Some("Jobs".into());
        layer
    });
    cx.update(|window, cx| {
        layer.update(cx, |layer, cx| {
            layer.feature_source(Source::Mock, cx);
            layer.toggle(window, cx);
            layer.read_back(vec![note("when-shown", Some(TWO), None)], cx);
        })
    });
    cx.run_until_parked();
    assert!(layer.read_with(cx, |layer, cx| layer.beside.read(cx).reads.is_empty()), "no card was shown, so nothing was read");
    cx.update(|window, cx| layer.update(cx, |layer, cx| layer.open_note(1, window, cx)));
    settle(cx);
    assert_eq!(slugs_of("when-shown").len(), 2);
}

#[gpui::test]
fn without_a_source_the_card_draws_no_chip(cx: &mut TestAppContext) {
    ready(cx);
    let (layer, cx) = cx.add_window_view(|_, cx| {
        let mut layer = Layer::new(Rc::default(), Vec::new(), cx);
        layer.screen = Some("Jobs".into());
        layer
    });
    cx.update(|window, cx| {
        layer.update(cx, |layer, cx| {
            layer.toggle(window, cx);
            layer.read_back(vec![note("a", Some(TWO), None)], cx);
            layer.open_note(1, window, cx);
        })
    });
    settle(cx);
    assert!(drawn(cx, Role::Dialog, "Note"));
    assert!(!drawn(cx, Role::Button, "Feature: Job list and live status"));
}

#[gpui::test]
fn a_press_on_a_chip_opens_its_page_beside_the_note(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("a", Some(TWO), None)]);
    assert_eq!(beside(&layer, cx), None, "nothing opens until a press");
    let chip = bounds_of(cx, Role::Button, "Feature: Job list and live status").expect("the chip is drawn");
    cx.simulate_click(chip.center(), Modifiers::none());
    settle(cx);
    assert_eq!(beside(&layer, cx).as_deref(), Some("job-list-and-live-status"));
    let card = bounds_of(cx, Role::Dialog, "Note").expect("the card stays");
    let page = bounds_of(cx, Role::Dialog, "Feature page").expect("the page is drawn");
    assert!(page.left() >= card.right(), "the page stands to the right of the card: {page:?} {card:?}");
    assert!(drawn(cx, Role::Button, "Back"), "the page is in an overlay with its Back");
}

#[gpui::test]
fn a_press_on_the_other_chip_replaces_the_page(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("a", Some(TWO), None)]);
    let first = bounds_of(cx, Role::Button, "Feature: Job list and live status").unwrap();
    cx.simulate_click(first.center(), Modifiers::none());
    settle(cx);
    let second = bounds_of(cx, Role::Button, "Feature: Pause, resume, stop, snooze").unwrap();
    cx.simulate_click(second.center(), Modifiers::none());
    settle(cx);
    assert_eq!(beside(&layer, cx).as_deref(), Some("pause-resume-stop-snooze"), "one page at a time");
    assert_eq!(layer.read_with(cx, |layer, cx| layer.features_overlay(cx).map(|overlay| overlay.read(cx).depth())), Some(1), "in the one overlay");
}

#[gpui::test]
fn the_key_opens_the_first_feature_and_then_the_next(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("a", Some(TWO), None)]);
    focus_card(&layer, cx);
    cx.simulate_keystrokes("f");
    settle(cx);
    assert_eq!(beside(&layer, cx).as_deref(), Some("job-list-and-live-status"));
    assert!(drawn(cx, Role::Dialog, "Feature page"));
    // The keys are in the page now; the card gets them back to take the key again.
    focus_card(&layer, cx);
    cx.simulate_keystrokes("f");
    settle(cx);
    assert_eq!(beside(&layer, cx).as_deref(), Some("pause-resume-stop-snooze"));
    focus_card(&layer, cx);
    cx.simulate_keystrokes("f");
    settle(cx);
    assert_eq!(beside(&layer, cx).as_deref(), Some("job-list-and-live-status"), "round again");
}

#[gpui::test]
fn the_key_does_nothing_on_a_note_with_no_feature(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("nothing", None, None)]);
    focus_card(&layer, cx);
    cx.simulate_keystrokes("f");
    settle(cx);
    assert_eq!(beside(&layer, cx), None);
    assert!(!drawn(cx, Role::Dialog, "Feature page"));
}

#[gpui::test]
fn tab_reaches_the_chips_and_enter_opens_one(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("a", Some(TWO), None)]);
    focus_card(&layer, cx);
    let on_chip = |layer: &Entity<Layer>, cx: &mut VisualTestContext| {
        cx.update(|window, cx| layer.read(cx).beside.read(cx).on_chip(window))
    };
    let mut reached = None;
    for _ in 0..12 {
        cx.simulate_keystrokes("tab");
        cx.run_until_parked();
        if let Some(at) = on_chip(&layer, cx) {
            reached = Some(at);
            break;
        }
    }
    let at = reached.expect("Tab reaches a chip");
    settle(cx);
    cx.simulate_keystrokes("enter");
    // The harness presses a key and does not release it; a chip takes the release as the click.
    cx.update(|window, cx| {
        use gpui::InputEvent as _;
        window.dispatch_event(gpui::KeyUpEvent { keystroke: gpui::Keystroke::parse("enter").expect("a key") }.to_platform_input(), cx);
    });
    settle(cx);
    let slugs = slugs_of("a");
    assert_eq!(beside(&layer, cx).as_deref(), Some(slugs[at].as_str()));
}

#[gpui::test]
fn esc_closes_the_page_first_and_then_the_card(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("a", Some(TWO), None)]);
    focus_card(&layer, cx);
    cx.simulate_keystrokes("f");
    settle(cx);
    assert!(beside(&layer, cx).is_some());
    // From inside the page.
    cx.simulate_keystrokes("escape");
    settle(cx);
    assert_eq!(beside(&layer, cx), None, "the page closed");
    assert!(layer.read_with(cx, |layer, _| layer.opened.is_some()), "the card is still open");
    // From the card, the keys having come back to it.
    cx.simulate_keystrokes("f");
    settle(cx);
    focus_card(&layer, cx);
    cx.simulate_keystrokes("escape");
    settle(cx);
    assert_eq!(beside(&layer, cx), None);
    assert!(layer.read_with(cx, |layer, _| layer.opened.is_some()), "the first Esc took the page only");
    cx.simulate_keystrokes("escape");
    settle(cx);
    assert!(layer.read_with(cx, |layer, _| layer.opened.is_none()), "the next Esc is the card's own");
}

#[gpui::test]
fn a_story_opened_from_the_page_replaces_it_and_one_esc_goes_back(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("a", Some(TWO), None)]);
    focus_card(&layer, cx);
    cx.simulate_keystrokes("f");
    settle(cx);
    let overlay = layer.read_with(cx, |layer, cx| layer.features_overlay(cx)).expect("the page is open");
    cx.update(|window, cx| overlay.update(cx, |overlay, cx| overlay.push_story(ipc::SessionId::carried(crate::job_view::story::mock::JOB), "ask-3".into(), window, cx)));
    settle(cx);
    assert_eq!(overlay.read_with(cx, |overlay, _| overlay.depth()), 2);
    assert!(drawn(cx, Role::Dialog, "Feature page"), "still the one panel");
    cx.simulate_keystrokes("escape");
    settle(cx);
    assert_eq!(overlay.read_with(cx, |overlay, _| overlay.depth()), 1, "back to the page");
    assert!(beside(&layer, cx).is_some());
    cx.simulate_keystrokes("escape");
    settle(cx);
    assert_eq!(beside(&layer, cx), None, "the last Esc closes it");
}

#[gpui::test]
fn opening_another_note_puts_the_page_of_the_first_away(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("a", Some(TWO), None), note("b", Some(TWO), None)]);
    focus_card(&layer, cx);
    cx.simulate_keystrokes("f");
    settle(cx);
    assert!(beside(&layer, cx).is_some());
    cx.update(|window, cx| layer.update(cx, |layer, cx| layer.open_note(2, window, cx)));
    settle(cx);
    assert_eq!(beside(&layer, cx), None);
    assert!(layer.read_with(cx, |layer, cx| layer.features_overlay(cx).is_none()));
}

#[gpui::test]
fn a_chip_is_a_button_found_by_its_role_and_name(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, vec![note("a", Some(TWO), None)]);
    // A test window keeps every frame it drew, so each name is read once.
    let names: Vec<String> = cx.update(|window, cx| {
        let mut names: Vec<String> = Vec::new();
        for one in cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()) {
            if one.role == Role::Button && one.name.starts_with("Feature: ") && !names.contains(&one.name.to_string()) {
                names.push(one.name.to_string());
            }
        }
        names
    });
    assert_eq!(names, ["Feature: Job list and live status", "Feature: Pause, resume, stop, snooze"]);
}

#[test]
fn the_page_stands_to_the_right_of_a_card_where_it_fits_else_to_the_left() {
    use gpui::{point, px, size, Bounds};
    let room = Bounds { origin: point(px(0.), px(36.)), size: size(px(1400.), px(780.)) };
    let gap = px(8.);
    let want = px(832.);
    let card = Bounds { origin: point(px(100.), px(200.)), size: size(px(448.), px(300.)) };
    assert_eq!(super::side(card, room, gap, want), (px(556.), px(832.)));
    let right = Bounds { origin: point(px(900.), px(200.)), size: size(px(448.), px(300.)) };
    assert_eq!(super::side(right, room, gap, want), (px(900. - 8. - 832.), px(832.)));
    // Neither side holds it: the side with more room, at what it has.
    let narrow = Bounds { origin: point(px(0.), px(36.)), size: size(px(1100.), px(780.)) };
    let (x, wide) = super::side(Bounds { origin: point(px(100.), px(200.)), size: size(px(448.), px(300.)) }, narrow, gap, want);
    assert_eq!((x, wide), (px(556.), px(1100. - 8. - 556.)));
    // Neither holds even three fifths of it: it stands at the right edge over the card.
    let tight = Bounds { origin: point(px(0.), px(36.)), size: size(px(900.), px(780.)) };
    let (x, wide) = super::side(Bounds { origin: point(px(100.), px(200.)), size: size(px(448.), px(300.)) }, tight, gap, want);
    assert_eq!(wide, want * 0.6);
    assert!((x + wide - px(900. - 8.)).abs() < px(0.01));
}

#[gpui::test]
fn a_note_on_a_feature_sends_to_its_jobs_first_and_enter_sends_it_there(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("to-feature", Some(TWO), None)]);
    let asked = Rc::new(std::cell::RefCell::new(Vec::new()));
    let heard = Rc::clone(&asked);
    let _heard = cx.update(|_, cx| cx.subscribe(&layer, move |_, said: &crate::annotate::Asked, _| heard.borrow_mut().push(said.clone())));
    assert!(drawn(cx, Role::Button, "Job list and live status's Jobs"), "the first choice names the feature");
    assert!(drawn(cx, Role::Button, "Send to job"), "the Job picker stays beside it");
    // The send to the feature holds the keys Send to job holds on a note with no feature.
    cx.update(|window, cx| {
        let lead = layer.read(cx).card_keys.0.clone();
        window.focus(&lead, cx);
    });
    settle(cx);
    cx.simulate_keystrokes("enter");
    // The harness presses a key and does not release it; a button takes the release as the click.
    cx.update(|window, cx| {
        use gpui::InputEvent as _;
        window.dispatch_event(gpui::KeyUpEvent { keystroke: gpui::Keystroke::parse("enter").expect("a key") }.to_platform_input(), cx);
    });
    settle(cx);
    let asked = asked.borrow();
    match asked.as_slice() {
        [crate::annotate::Asked::ToFeature { feature, paths, notes, .. }] => {
            assert_eq!(feature, "Job list and live status");
            assert_eq!(paths, &[TWO.to_string()]);
            assert_eq!(notes, &["The row clips its title".to_string()]);
        }
        other => panic!("Enter sent {other:?}"),
    }
}

#[gpui::test]
fn the_jobs_a_feature_send_goes_to_are_named_under_it(cx: &mut TestAppContext) {
    let (layer, cx) = open(cx, vec![note("named", Some(TWO), None)]);
    let feature = layer.read_with(cx, |layer, cx| layer.feature_of(1, cx)).expect("the note maps to a feature");
    assert_eq!(feature.title, "Job list and live status");
    assert_eq!(
        feature.jobs.iter().map(|job| job.title.as_str()).collect::<Vec<_>>(),
        ["Keep the Jobs list at its place on refresh", "Show what a Job waits on in its row"]
    );
}
