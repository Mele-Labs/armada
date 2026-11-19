use gpui::{px, TestAppContext, VisualTestContext};

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
fn down_and_up_move_the_row_past_the_area_heads_and_the_page_follows_the_row(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    assert_eq!(cursor(&screen, cx).as_deref(), Some("start-a-job"));
    cx.run_until_parked();
    assert_eq!(screen.read_with(cx, |screen, _| screen.opened().map(|row| row.slug.clone())).as_deref(), Some("start-a-job"), "the page of the row the keys are on is open");
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
    cx.simulate_keystrokes("k k k k k k k k k k k k");
    assert_eq!(screen.read_with(cx, |screen, _| screen.on_project), Some(super::ProjectPage::Overview), "past the first feature are the three project rows, and it stops at Overview");
    cx.simulate_keystrokes("j j j");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("start-a-job"), "down from them is the first feature");
    cx.simulate_keystrokes("enter");
    assert_eq!(screen.read_with(cx, |screen, _| screen.opened().map(|row| row.title.clone())).as_deref(), Some("Start a Job"));
    // Moving the row moves the page with it.
    cx.simulate_keystrokes("down");
    cx.run_until_parked();
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

#[gpui::test]
fn the_selected_features_page_is_open_beside_the_list_and_moving_the_row_replaces_it_and_counts_the_first_as_looked_at(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.update(|_, cx| super::looked::forget_all(cx));
    cx.run_until_parked();
    let page = screen.read_with(cx, |screen, _| screen.page.clone()).expect("the page of the selected feature is open without Enter");
    assert_eq!(page.read_with(cx, |page, _| page.slug().to_string()), "start-a-job");
    assert!(drawn(cx, gpui::Role::Region, "On it now"), "the cards are on its page");
    assert!(drawn(cx, gpui::Role::Heading, "Start a Job"));
    assert!(drawn(cx, gpui::Role::Label, "Area: Running a Job"), "the area chip is the area's title");
    assert_eq!(cx.update(|_, cx| super::looked::before(mock::PROJECT, "start-a-job", cx)), None, "opening is not looking");
    // The list keeps the keys until Tab; Enter on the feature that is open changes nothing.
    cx.simulate_keystrokes("enter");
    assert_eq!(screen.read_with(cx, |screen, _| screen.page.clone()).map(|one| one.entity_id()), Some(page.entity_id()));
    cx.simulate_keystrokes("down");
    cx.run_until_parked();
    let second = screen.read_with(cx, |screen, _| screen.page.clone()).expect("the next page");
    assert_eq!(second.read_with(cx, |page, _| page.slug().to_string()), "job-list-and-live-status", "the page is replaced");
    assert!(cx.update(|_, cx| super::looked::before(mock::PROJECT, "start-a-job", cx)).is_some(), "the one he left is looked at now");
    assert_eq!(cx.update(|_, cx| super::looked::before(mock::PROJECT, "job-list-and-live-status", cx)), None);
    cx.simulate_keystrokes("tab");
    assert!(cx.update(|window, cx| second.read(cx).rail.is_focused(window)), "Tab from the list goes to the page's sections");
    cx.simulate_keystrokes("j");
    assert_eq!(second.read_with(cx, |page, _| page.section()), super::page_model::Section::Bugs, "J moves the card, and the list stays");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("job-list-and-live-status"));
    cx.simulate_keystrokes("escape");
    assert!(cx.update(|window, cx| screen.read(cx).list_focus.is_focused(window)), "Esc gives the keys back to the list");
}

/// The list's and the page's widths with a feature open, the window being `wide` from the start.
fn widths_at(cx: &mut TestAppContext, wide: f32) -> (gpui::Pixels, gpui::Pixels) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.simulate_resize(gpui::size(px(wide), px(800.)));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    assert!(screen.read_with(cx, |screen, _| screen.page.is_some()));
    for _ in 0..2 {
        screen.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    let size = cx.update(|window, _| window.viewport_size());
    assert_eq!(size.width, px(wide), "the window is as wide as it was made");
    let width = |cx: &mut VisualTestContext, name: &str, role: gpui::Role| {
        // The book of a window with no label root keeps every frame's entries, so the last is the frame just drawn.
        cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds.size.width)).expect("drawn")
    };
    (width(cx, "Features", gpui::Role::ListBox), width(cx, "Feature page", gpui::Role::Region))
}

#[gpui::test]
fn the_list_is_288_wide_from_900_up_and_the_page_has_the_rest(cx: &mut TestAppContext) {
    // The list is a panel of six steps of the largest space, its box inside the panel's 1px border; the page's panel has what the padding, the gap and the list leave.
    assert_eq!(widths_at(cx, 900.), (px(286.), px(582.)));
}

#[gpui::test]
fn at_820_the_page_takes_the_whole_width_where_it_is_opened(cx: &mut TestAppContext) {
    let (_, page) = widths_at(cx, 820.);
    assert_eq!(page, px(796.), "the list is not beside it: the panel has the window less the padding either side");
}

#[gpui::test]
fn the_rail_does_not_grow_with_a_wide_window(cx: &mut TestAppContext) {
    let (list, page) = widths_at(cx, 1400.);
    assert_eq!((list, page), (px(286.), px(1082.)), "the page takes what the rail leaves");
}

#[gpui::test]
fn j_past_three_features_stamps_none_of_them(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.update(|_, cx| super::looked::forget_all(cx));
    cx.run_until_parked();
    let mut passed = vec![cursor(&screen, cx).expect("a row")];
    for _ in 0..3 {
        cx.simulate_keystrokes("j");
        cx.run_until_parked();
        passed.push(cursor(&screen, cx).expect("a row"));
    }
    assert_eq!(passed.len(), 4);
    for slug in &passed[..3] {
        assert_eq!(cx.update(|_, cx| super::looked::before(mock::PROJECT, slug, cx)), None, "{slug} was only passed over");
    }
}

#[gpui::test]
fn tab_into_a_page_and_leaving_it_stamps_it_and_a_page_only_passed_over_does_not(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.update(|_, cx| super::looked::forget_all(cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("tab");
    assert!(cx.update(|window, cx| screen.read(cx).page.clone().expect("a page").read(cx).rail.is_focused(window)), "Tab is in the page's sections");
    cx.simulate_keystrokes("escape");
    cx.simulate_keystrokes("down");
    cx.run_until_parked();
    assert!(cx.update(|_, cx| super::looked::before(mock::PROJECT, "start-a-job", cx)).is_some(), "he was in it, so leaving counts");
    cx.simulate_keystrokes("down");
    cx.run_until_parked();
    assert_eq!(cx.update(|_, cx| super::looked::before(mock::PROJECT, "job-list-and-live-status", cx)), None, "the one he only passed over is not looked at");
}

/// The screen on the mock library in a window `wide` across, drawn once.
fn open_wide(cx: &mut TestAppContext, wide: f32) -> (gpui::Entity<FeaturesScreen>, &mut VisualTestContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.simulate_resize(gpui::size(px(wide), px(800.)));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    redraw(cx);
    (screen, cx)
}

fn redraw(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
}

/// Where the last drawing of a thing is in the window's book, which keeps every frame's entries: a larger place is a later frame.
fn last_drawn(cx: &mut VisualTestContext, role: gpui::Role, name: &str) -> Option<(usize, gpui::Pixels)> {
    cx.update(|window, cx| {
        cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().enumerate().rev().find(|(_, one)| one.role == role && one.name == name).map(|(at, one)| (at, one.bounds.size.width))
    })
}

fn page_of(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext) -> Option<String> {
    screen.read_with(cx, |screen, _| screen.opened().map(|row| row.slug.clone()))
}

/// Whether the list is the screen's last drawing and the page is not: the list alone, with no Back row.
fn list_alone(cx: &mut VisualTestContext) -> bool {
    let list = last_drawn(cx, gpui::Role::ListBox, "Features").expect("the list was drawn");
    let back = last_drawn(cx, gpui::Role::Button, "Back");
    back.is_none_or(|back| list.0 > back.0) && list.1 == px(794.)
}

#[gpui::test]
fn at_820_j_moves_the_row_and_no_page_is_drawn_and_the_list_is_the_whole_width(cx: &mut TestAppContext) {
    let (screen, cx) = open_wide(cx, 820.);
    assert_eq!(cursor(&screen, cx).as_deref(), Some("start-a-job"));
    assert_eq!(page_of(&screen, cx), None, "no page opens beside a list that has the width");
    cx.simulate_keystrokes("j");
    redraw(cx);
    assert_eq!(cursor(&screen, cx).as_deref(), Some("job-list-and-live-status"), "J moves the row");
    cx.simulate_keystrokes("down down k up");
    redraw(cx);
    assert_eq!(cursor(&screen, cx).as_deref(), Some("job-list-and-live-status"), "the arrows and K move it too: two down, two back");
    assert!(screen.read_with(cx, |screen, _| screen.page.is_none()), "no page is held");
    assert!(list_alone(cx), "the list is drawn alone, across the window");
    assert!(last_drawn(cx, gpui::Role::Region, "Feature page").is_none_or(|page| page.0 < last_drawn(cx, gpui::Role::ListBox, "Features").expect("the list").0), "no page is drawn with it");
}

#[gpui::test]
fn at_820_enter_opens_the_page_full_width_with_a_back_row_and_escape_returns_to_the_same_row(cx: &mut TestAppContext) {
    let (screen, cx) = open_wide(cx, 820.);
    cx.simulate_keystrokes("j j");
    cx.simulate_keystrokes("enter");
    redraw(cx);
    assert_eq!(page_of(&screen, cx).as_deref(), Some("pause-resume-stop-snooze"));
    let (_, wide) = last_drawn(cx, gpui::Role::Region, "Feature page").expect("the page is drawn");
    assert_eq!(wide, px(796.), "the page panel has the whole width less the padding");
    let (back, _) = last_drawn(cx, gpui::Role::Button, "Back").expect("a Back row is over the page");
    let bounds = |cx: &mut VisualTestContext, role: gpui::Role, name: &str| cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds)).expect("drawn");
    let (panel, button, title) = (bounds(cx, gpui::Role::Region, "Feature page"), bounds(cx, gpui::Role::Button, "Back"), bounds(cx, gpui::Role::Button, "Start a Job on it"));
    assert!(panel.contains(&button.origin) && button.origin.y < title.origin.y, "Back is inside the page's panel and above its page");
    assert!(last_drawn(cx, gpui::Role::ListBox, "Features").expect("the list was drawn before").0 < back, "the list is not drawn with it");
    assert!(cx.update(|window, cx| screen.read(cx).page.clone().expect("a page").read(cx).rail.is_focused(window)), "the keys are on the page, the list not being drawn to hold them");
    cx.simulate_keystrokes("escape");
    redraw(cx);
    assert!(screen.read_with(cx, |screen, _| screen.page.is_none()));
    assert_eq!(cursor(&screen, cx).as_deref(), Some("pause-resume-stop-snooze"), "the same row is selected");
    assert!(list_alone(cx));
    cx.simulate_keystrokes("k");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("job-list-and-live-status"), "the list has the keys again");
}

#[gpui::test]
fn at_820_a_press_on_a_row_or_on_back_opens_and_closes_the_page(cx: &mut TestAppContext) {
    let (screen, cx) = open_wide(cx, 820.);
    let row = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), gpui::Role::ListBoxOption, "Job list and live status").map(|one| one.bounds)).expect("the row is drawn");
    cx.simulate_click(row.center(), gpui::Modifiers::default());
    redraw(cx);
    assert_eq!(page_of(&screen, cx).as_deref(), Some("job-list-and-live-status"));
    assert_eq!(last_drawn(cx, gpui::Role::Region, "Feature page").expect("the page").1, px(796.));
    let back = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), gpui::Role::Button, "Back").map(|one| one.bounds)).expect("Back is drawn");
    cx.simulate_click(back.center(), gpui::Modifiers::default());
    redraw(cx);
    assert!(screen.read_with(cx, |screen, _| screen.page.is_none()));
    assert_eq!(cursor(&screen, cx).as_deref(), Some("job-list-and-live-status"), "the row stays");
    assert!(list_alone(cx));
}

#[gpui::test]
fn at_820_tab_opens_the_page_as_enter_does(cx: &mut TestAppContext) {
    let (screen, cx) = open_wide(cx, 820.);
    cx.simulate_keystrokes("tab");
    redraw(cx);
    assert_eq!(page_of(&screen, cx).as_deref(), Some("start-a-job"));
    assert!(cx.update(|window, cx| screen.read(cx).page.clone().expect("a page").read(cx).rail.is_focused(window)), "Tab is in the page's sections");
    cx.simulate_keystrokes("j");
    assert_eq!(screen.read_with(cx, |screen, _| screen.page.clone()).expect("a page").read_with(cx, |page, _| page.section()), super::page_model::Section::Bugs, "J moves the card there");
}

#[gpui::test]
fn at_820_a_page_is_stamped_looked_at_when_he_leaves_it_and_a_row_passed_over_is_not(cx: &mut TestAppContext) {
    let (screen, cx) = open_wide(cx, 820.);
    cx.update(|_, cx| super::looked::forget_all(cx));
    cx.simulate_keystrokes("j j j");
    cx.run_until_parked();
    for slug in ["start-a-job", "job-list-and-live-status", "pause-resume-stop-snooze"] {
        assert_eq!(cx.update(|_, cx| super::looked::before(mock::PROJECT, slug, cx)), None, "{slug} was only passed over");
    }
    cx.simulate_keystrokes("enter");
    assert_eq!(cx.update(|_, cx| super::looked::before(mock::PROJECT, "model-and-context", cx)), None, "opening is not looking");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.update(|_, cx| super::looked::before(mock::PROJECT, "model-and-context", cx)).is_some(), "Enter was engaging, so leaving counts");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("model-and-context"));
}

#[gpui::test]
fn at_1400_the_page_still_follows_the_selection_beside_the_list(cx: &mut TestAppContext) {
    let (screen, cx) = open_wide(cx, 1400.);
    cx.run_until_parked();
    assert_eq!(page_of(&screen, cx).as_deref(), Some("start-a-job"));
    cx.simulate_keystrokes("j");
    cx.run_until_parked();
    assert_eq!(page_of(&screen, cx).as_deref(), Some("job-list-and-live-status"), "moving the row moves the page");
    redraw(cx);
    assert!(last_drawn(cx, gpui::Role::Button, "Back").is_none(), "no Back row where the page is beside the list");
}

#[gpui::test]
fn the_window_narrowing_puts_the_page_away_and_widening_brings_the_row_s_page_back(cx: &mut TestAppContext) {
    let (screen, cx) = open_wide(cx, 1400.);
    cx.run_until_parked();
    cx.simulate_keystrokes("j");
    cx.run_until_parked();
    assert_eq!(page_of(&screen, cx).as_deref(), Some("job-list-and-live-status"));
    cx.simulate_resize(gpui::size(px(820.), px(800.)));
    redraw(cx);
    assert_eq!(page_of(&screen, cx), None, "the list is alone");
    assert_eq!(cursor(&screen, cx).as_deref(), Some("job-list-and-live-status"), "on the row it was on");
    assert!(list_alone(cx));
    cx.simulate_keystrokes("j");
    assert_eq!(page_of(&screen, cx), None);
    cx.simulate_resize(gpui::size(px(1400.), px(800.)));
    redraw(cx);
    assert_eq!(page_of(&screen, cx).as_deref(), Some("pause-resume-stop-snooze"), "the page follows the row again");
}
