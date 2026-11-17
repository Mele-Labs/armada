use gpui::{TestAppContext, VisualTestContext};

use super::model::{self, Line};
use super::{mock, FeaturesScreen, Load};

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
    });
}

fn open(cx: &mut TestAppContext) -> (gpui::Entity<FeaturesScreen>, &mut VisualTestContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    (screen, cx)
}

fn drawn(cx: &mut VisualTestContext, role: gpui::Role, name: &str) -> bool {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn cursor(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext) -> Option<String> {
    screen.read_with(cx, |screen, _| screen.cursor.clone())
}

#[test]
fn areas_come_in_fleets_order_each_with_its_features_and_the_pending_one_under_pending() {
    let listed = model::listed(&mock::library(), "");
    let heads: Vec<&str> = listed.lines.iter().filter_map(|line| if let Line::Head { title, .. } = line { Some(title.as_str()) } else { None }).collect();
    assert_eq!(heads, ["Running a Job", "Job thread and terminal", "Pull requests and review", "pending"]);
    let titles: Vec<&str> = listed.shown.iter().map(|one| one.row.title.as_str()).collect();
    assert_eq!(titles.len(), 10);
    assert_eq!(&titles[..4], ["Start a Job", "Job list and live status", "Pause, resume, stop, snooze", "Model and context"]);
    // A head is followed by its own features and no other area's.
    assert_eq!(listed.lines[0], Line::Head { slug: "running-a-job".into(), title: "Running a Job".into() });
    assert_eq!(listed.lines[5], Line::Head { slug: "job-thread-and-terminal".into(), title: "Job thread and terminal".into() });
    let last = listed.shown.last().expect("the mock has features");
    assert_eq!((last.row.title.as_str(), last.row.state), ("Agent notes on a diff", ipc::FeatureState::Pending));
    assert!(matches!(listed.lines.last(), Some(Line::Feature(9))) && matches!(&listed.lines[listed.lines.len() - 2], Line::Head { slug, .. } if slug == "pending"));
}

#[test]
fn a_feature_names_the_jobs_on_it_and_carries_a_stale_mark_where_a_section_is_old() {
    let listed = model::listed(&mock::library(), "");
    let by = |title: &str| listed.shown.iter().find(|one| one.row.title == title).expect("the mock has it");
    assert_eq!(by("Start a Job").on, ["Add the model picker to the start card"]);
    assert_eq!(by("Job list and live status").on.len(), 2);
    assert_eq!(by("Job list and live status").on_said(), "Keep the Jobs list at its place on refresh\nShow what a Job waits on in its row");
    assert_eq!(by("Pause, resume, stop, snooze").stale.as_deref(), Some("stale since #3712"));
    assert_eq!(by("Model and context").stale.as_deref(), Some("stale since #3655"));
    assert!(by("Terminal Jobs").on.is_empty() && by("Terminal Jobs").stale.is_none());
    assert_eq!(listed.shown.iter().filter(|one| !one.on.is_empty()).count(), 2);
    assert_eq!(listed.shown.iter().filter(|one| one.stale.is_some()).count(), 2);
}

#[test]
fn the_filter_keeps_the_features_it_names_and_drops_an_area_left_with_none() {
    let library = mock::library();
    let listed = model::listed(&library, "terminal jobs");
    let titles: Vec<&str> = listed.shown.iter().map(|one| one.row.title.as_str()).collect();
    assert_eq!(titles, ["Terminal Jobs"]);
    assert_eq!(listed.lines.len(), 2, "one head and the one feature");
    // An area's own name keeps all of its features.
    assert_eq!(model::listed(&library, "pull requests").shown.len(), 2);
    assert_eq!(model::listed(&library, "terminal").shown.len(), 3, "the area Job thread and terminal");
    assert!(model::listed(&library, "nothing like this").lines.is_empty());
}

#[gpui::test]
fn down_and_up_move_the_row_past_the_area_heads_and_enter_opens_the_feature(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    assert_eq!(cursor(&screen, cx).as_deref(), Some("start-a-job"));
    assert!(screen.read_with(cx, |screen, _| screen.opened().is_none()), "nothing is open until Enter");
    cx.simulate_keystrokes("down");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("job-list-and-live-status"));
    cx.simulate_keystrokes("down down");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("model-and-context"));
    cx.simulate_keystrokes("down");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("thread-and-composer"), "over the next area's head");
    cx.simulate_keystrokes("up");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("model-and-context"));
    cx.simulate_keystrokes("j");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("thread-and-composer"), "J is Down");
    cx.simulate_keystrokes("k k k k k k k k k k");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("start-a-job"), "it stops at the first");
    cx.simulate_keystrokes("enter");
    assert_eq!(screen.read_with(cx, |screen, _| screen.opened().map(|row| row.title.clone())).as_deref(), Some("Start a Job"));
    // Moving the row leaves the page open until Enter picks another.
    cx.simulate_keystrokes("down");
    assert_eq!(screen.read_with(cx, |screen, _| screen.opened().map(|row| row.slug.clone())).as_deref(), Some("start-a-job"));
    cx.simulate_keystrokes("enter");
    assert_eq!(screen.read_with(cx, |screen, _| screen.opened().map(|row| row.slug.clone())).as_deref(), Some("job-list-and-live-status"));
}

#[gpui::test]
fn the_keys_are_the_registrys_features_acts_and_a_rebound_key_moves_the_row(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    let bound = crate::keymap::bound(&crate::keymap::Rebinding::default());
    for id in ["features_move", "features_open", "features_filter"] {
        assert!(bound.iter().any(|key| key.act.id == id && key.context == Some("features")), "{id} is bound under features");
    }
    cx.update(|_, cx| crate::keymap::install(&crate::keymap::Rebinding::read(r#"{"version":1,"bindings":{"features_move":["n"]}}"#), cx));
    cx.simulate_keystrokes("n");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("job-list-and-live-status"), "the person's key moves the row");
    cx.simulate_keystrokes("j");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("job-list-and-live-status"), "the key he gave up does nothing");
}

#[gpui::test]
fn the_filter_takes_a_bare_key_as_typing_and_escape_gives_the_keys_back(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("/");
    cx.simulate_input("terminal jobs");
    cx.run_until_parked();
    assert_eq!(screen.read_with(cx, |screen, _| screen.listed.shown.len()), 1, "the list narrows as he types");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("terminal-jobs"), "the row keeps to what is left");
    cx.simulate_keystrokes("escape");
    cx.simulate_keystrokes("j");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("terminal-jobs"), "Esc is back on the list, where J moves and there is nowhere to");
    assert!(!cx.update(|window, cx| screen.read(cx).typing(window, cx)), "the filter no longer has the keys");
}

#[gpui::test]
fn the_list_draws_a_skeleton_while_the_read_is_pending_and_the_features_once_it_answers(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    screen.update(cx, |screen, cx| {
        screen.library = Load::Reading;
        screen.relist(cx);
    });
    cx.run_until_parked();
    assert!(drawn(cx, gpui::Role::Image, "Loading"), "bones stand where the rows will be");
    assert!(screen.read_with(cx, |screen, _| screen.listed.shown.is_empty()), "no row is held while the read is pending");
    screen.update(cx, |screen, cx| {
        screen.library = Load::Read(mock::library());
        screen.relist(cx);
    });
    cx.run_until_parked();
    assert_eq!(screen.read_with(cx, |screen, _| screen.listed.shown.len()), 10, "the features are held once it answers");
    assert!(drawn(cx, gpui::Role::ListBoxOption, "Start a Job"));
}

#[gpui::test]
fn every_element_is_found_by_its_role_and_name(cx: &mut TestAppContext) {
    let (_screen, cx) = open(cx);
    assert!(drawn(cx, gpui::Role::ListBox, "Features"));
    assert!(drawn(cx, gpui::Role::SearchInput, "Filter features"));
    assert!(drawn(cx, gpui::Role::Region, "Feature page"));
    for head in ["Running a Job", "Job thread and terminal"] {
        assert!(drawn(cx, gpui::Role::Heading, head), "{head} is a heading");
    }
    for feature in ["Start a Job", "Job list and live status", "Pause, resume, stop, snooze"] {
        assert!(drawn(cx, gpui::Role::ListBoxOption, feature), "{feature} is an option");
    }
    assert!(drawn(cx, gpui::Role::Image, "On it now: Add the model picker to the start card"));
    assert!(drawn(cx, gpui::Role::Image, "stale since #3712"));
}
