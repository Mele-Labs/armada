use std::time::Duration;

use chrono::Utc;
use gpui::{Role, TestAppContext, VisualTestContext};

use super::librarian::{card, Card, Press};
use super::mock::{self, Reading};
use super::{chosen, FeaturesScreen};

fn open(cx: &mut TestAppContext) -> (gpui::Entity<FeaturesScreen>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
        chosen::forget_all(cx);
    });
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    (screen, cx)
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn card_of(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext) -> Option<Card> {
    screen.read_with(cx, |screen, _| screen.librarian_card())
}

/// The mock's run takes this long to come back.
fn let_it_read(cx: &mut VisualTestContext) {
    cx.executor().advance_clock(Duration::from_millis(700));
    cx.run_until_parked();
}

#[gpui::test]
fn behind_says_where_it_stopped_and_lists_the_merged_numbers(cx: &mut TestAppContext) {
    let behind = card(&mock::librarian(Reading::Behind), &Press::Idle, Utc::now());
    assert_eq!(behind.said(), "Not read since #3861");
    assert_eq!(behind.merged_said().as_deref(), Some("#3862 #3863 #3864"));
    assert_eq!(behind.button(), Some(("Catch up", true, false)), "Catch up is the primary button");
    let (_screen, cx) = open(cx);
    assert!(drawn(cx, Role::Label, "Not read since #3861"));
    assert!(drawn(cx, Role::Label, "Merged since: #3862 #3863 #3864"));
    assert!(drawn(cx, Role::Button, "Catch up"));
}

#[test]
fn never_current_and_working_say_what_they_are() {
    let now = Utc::now();
    let never = card(&mock::librarian(Reading::Never), &Press::Idle, now);
    assert_eq!((never.said().as_str(), never.button()), ("Not read yet", Some(("Catch up", false, false))));
    let current = card(&mock::librarian(Reading::Current), &Press::Idle, now);
    assert_eq!(current.said(), "Read through #3864, 4 min ago");
    assert_eq!(current.button(), None);
    let working = card(&mock::librarian(Reading::Behind), &Press::Working, now);
    assert_eq!((working.said().as_str(), working.button()), ("Reading", Some(("Catch up", false, true))), "the button is off while it reads");
}

#[gpui::test]
fn a_press_reads_and_comes_back_current_with_the_new_read_through(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    assert!(matches!(card_of(&screen, cx), Some(Card::Behind { since: Some(3861), .. })));
    cx.simulate_keystrokes("c");
    assert_eq!(card_of(&screen, cx), Some(Card::Working));
    assert!(drawn(cx, Role::Image, "Reading"), "the spinner glyph names what it is doing");
    assert!(drawn(cx, Role::Label, "Reading"));
    let_it_read(cx);
    let Some(Card::Current { through, .. }) = card_of(&screen, cx) else { panic!("it is current once the run is back: {:?}", card_of(&screen, cx)) };
    assert_eq!(through, Some(3864), "the newest merged pull request is what it read through");
    assert_eq!(card_of(&screen, cx).and_then(|card| card.button()), None, "a current card has no button");
}

#[gpui::test]
fn a_refused_press_shows_the_sentence_and_retry(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    screen.update(cx, |screen, _| screen.mock_refusal = Some("No project is named armada".into()));
    cx.simulate_keystrokes("c");
    let_it_read(cx);
    assert_eq!(card_of(&screen, cx), Some(Card::Failed("No project is named armada".into())));
    assert!(drawn(cx, Role::Label, "No project is named armada"));
    assert!(drawn(cx, Role::Button, "Retry"));
    screen.update(cx, |screen, _| screen.mock_refusal = None);
    cx.simulate_keystrokes("c");
    let_it_read(cx);
    assert!(matches!(card_of(&screen, cx), Some(Card::Current { .. })), "Retry runs it again");
}

#[gpui::test]
fn a_chip_switch_changes_the_project_the_list_asks_for(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    let asked = |screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext| screen.read_with(cx, |screen, cx| (screen.wanted(cx), screen.listed.shown.iter().map(|one| one.row.title.clone()).collect::<Vec<_>>()));
    assert!(drawn(cx, Role::Button, "Armada") && drawn(cx, Role::Button, "Pocket") && drawn(cx, Role::Button, "Store"), "one chip for each project");
    assert_eq!(asked(&screen, cx).0, None, "nothing is chosen yet");
    cx.simulate_keystrokes("2");
    assert_eq!(asked(&screen, cx), (Some("pocket".to_string()), vec!["Pair a phone".to_string(), "Offline queue".to_string()]));
    assert_eq!(cx.update(|_, cx| chosen::get(cx)).as_deref(), Some("pocket"), "the choice is kept for the next start");
    assert_eq!(card_of(&screen, cx).map(|card| card.button()), Some(None), "Pocket's librarian is current, so its card has no button");
    cx.simulate_keystrokes("9");
    assert_eq!(asked(&screen, cx).0.as_deref(), Some("pocket"), "a number past the last chip does nothing");
    cx.simulate_keystrokes("3");
    assert_eq!(card_of(&screen, cx), Some(Card::Never), "Store's librarian never ran");
}
