use gpui::{Modifiers, Role, TestAppContext, VisualTestContext};

use super::curate::{self, Write};
use super::curate_view::{confirm_name, merge_name, rename_name};
use super::{mock, FeaturesScreen, Load};
use ipc::FeatureState;

const PENDING: &str = "agent-notes-on-a-diff";
const TITLE: &str = "Agent notes on a diff";

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
    });
}

fn open_with(cx: &mut TestAppContext, library: ipc::FeatureLibrary) -> (gpui::Entity<FeaturesScreen>, &mut VisualTestContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    screen.update(cx, |screen, cx| {
        screen.library = Load::Read(library);
        screen.relist(cx);
    });
    cx.run_until_parked();
    (screen, cx)
}

/// The librarian's card lists its steps, which push the pending row's editor past this window; these tests are not about it, so it lists none.
fn open(cx: &mut TestAppContext) -> (gpui::Entity<FeaturesScreen>, &mut VisualTestContext) {
    open_with(cx, ipc::FeatureLibrary { librarian: Default::default(), ..mock::library() })
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn press(cx: &mut VisualTestContext, role: Role, name: &str) {
    // The book keeps every frame the test drew, so the newest is the one that is on screen.
    let at = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds.center())).unwrap_or_else(|| panic!("{name} is not drawn"));
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
}

fn sent(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext) -> Vec<Write> {
    screen.read_with(cx, |screen, _| screen.curate.sent.clone())
}

fn set(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext, which: usize, value: &str) {
    cx.update(|window, cx| {
        screen.update(cx, |screen, cx| {
            let form = screen.curate.form.as_ref().expect("an editor is open");
            let input = if which == 0 { form.one.clone() } else { form.two.clone() };
            input.update(cx, |state, cx| state.set_value(value.to_string(), window, cx));
        })
    });
    cx.run_until_parked();
}

fn name_the_area(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext, name: &str) {
    cx.update(|window, cx| {
        screen.update(cx, |screen, cx| {
            let input = screen.curate.sheet.as_ref().expect("the sheet is open").name.clone();
            input.update(cx, |state, cx| state.set_value(name.to_string(), window, cx));
        })
    });
    cx.run_until_parked();
}

#[gpui::test]
fn confirm_sends_upsert_features_confirmed_for_the_pending_slug_in_the_first_area(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    for name in [confirm_name(TITLE), merge_name(TITLE), rename_name(TITLE)] {
        assert!(drawn(cx, Role::Button, &name), "{name} is drawn on the pending row");
    }
    press(cx, Role::Button, &confirm_name(TITLE));
    assert!(drawn(cx, Role::Form, &format!("Confirm {TITLE}")), "the editor opens under the row");
    set(&screen, cx, 1, "Agent notes on diff lines");
    press(cx, Role::Button, "Confirm");
    let Some(Write::Upsert(upsert)) = sent(&screen, cx).into_iter().next() else { panic!("an upsert was sent") };
    assert_eq!(upsert.project, mock::PROJECT);
    assert!(upsert.areas.is_empty());
    let seed = &upsert.features[0];
    assert_eq!((seed.slug.as_str(), seed.area.as_str(), seed.title.as_str(), seed.summary.as_str(), seed.state), (PENDING, "running-a-job", TITLE, "Agent notes on diff lines", Some(FeatureState::Confirmed)));
    assert!(serde_json::to_string(&upsert).expect("encodes").contains("\"state\":\"confirmed\""));
    assert!(screen.read_with(cx, |screen, _| screen.curate.form.is_none()), "the editor shuts once it is sent");
}

#[gpui::test]
fn confirm_waits_on_a_title_and_a_line(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    press(cx, Role::Button, &confirm_name(TITLE));
    set(&screen, cx, 1, "");
    assert!(!screen.read_with(cx, |screen, cx| screen.form_ready(cx)), "a blank line is refused by Fleet, so it is not sent");
    press(cx, Role::Button, "Confirm");
    assert!(sent(&screen, cx).is_empty());
}

#[gpui::test]
fn merge_into_sends_change_feature_with_the_slug_picked_in_a_filterable_list(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    press(cx, Role::Button, &merge_name(TITLE));
    assert!(screen.read_with(cx, |screen, _| screen.curate.menu.is_some()), "the picker is open");
    cx.simulate_keystrokes("t e r m i n a l space j o b s");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    let Some(Write::Change(change)) = sent(&screen, cx).into_iter().next() else { panic!("a change was sent") };
    assert_eq!((change.feature.as_str(), change.into.as_deref(), change.slug, change.title, change.area), (PENDING, Some("terminal-jobs"), None, None, None));
    assert!(screen.read_with(cx, |screen, _| screen.curate.menu.is_none()), "the picker shuts on a pick");
}

#[test]
fn merge_into_lists_the_catalogs_features_and_never_the_pending_ones() {
    let library = mock::library();
    let targets: Vec<String> = curate::merge_targets(&library).into_iter().map(|(row, _)| row.slug.clone()).collect();
    assert_eq!(targets.len(), 9);
    assert!(!targets.iter().any(|slug| slug == PENDING));
}

#[gpui::test]
fn rename_sends_the_new_slug_and_title(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    press(cx, Role::Button, &rename_name(TITLE));
    assert!(drawn(cx, Role::Form, &format!("Rename {TITLE}")));
    set(&screen, cx, 0, "notes-on-a-diff");
    set(&screen, cx, 1, "Notes on a diff");
    press(cx, Role::Button, "Save");
    let Some(Write::Change(change)) = sent(&screen, cx).into_iter().next() else { panic!("a change was sent") };
    assert_eq!((change.feature.as_str(), change.slug.as_deref(), change.title.as_deref(), change.into), (PENDING, Some("notes-on-a-diff"), Some("Notes on a diff"), None));
}

#[test]
fn rename_sends_only_what_changed_and_nothing_where_nothing_did() {
    let row = mock::library().areas[3].features[0].clone();
    assert_eq!(curate::rename_write("armada", &row, PENDING, TITLE), None);
    let Some(Write::Change(change)) = curate::rename_write("armada", &row, PENDING, "Notes") else { panic!("a change") };
    assert_eq!((change.slug, change.title.as_deref()), (None, Some("Notes")));
}

#[gpui::test]
fn an_area_of_nine_draws_split_and_one_of_eight_does_not(cx: &mut TestAppContext) {
    let (_, nine) = open_with(cx, mock::library_with(5));
    assert!(drawn(nine, Role::Button, "Split Running a Job"));
    assert!(drawn(nine, Role::Image, "9 features"));
}

#[gpui::test]
fn an_area_of_eight_draws_no_split(cx: &mut TestAppContext) {
    let (_, eight) = open_with(cx, mock::library_with(4));
    assert!(!drawn(eight, Role::Button, "Split Running a Job"), "eight features is not past eight");
    assert!(!drawn(eight, Role::Image, "8 features"));
}

#[gpui::test]
fn split_sends_one_upsert_for_the_new_area_then_one_change_per_ticked_feature(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::library_with(5));
    press(cx, Role::Button, "Split Running a Job");
    assert!(drawn(cx, Role::Dialog, "Split Running a Job"));
    assert!(screen.read_with(cx, |screen, cx| !screen.split_ready(cx)), "no name and nothing ticked");
    name_the_area(&screen, cx, "Starting Jobs");
    assert!(screen.read_with(cx, |screen, cx| !screen.split_ready(cx)), "a name and nothing ticked");
    press(cx, Role::CheckBox, "Start a Job");
    press(cx, Role::CheckBox, "Quick dispatch");
    assert!(screen.read_with(cx, |screen, cx| screen.split_ready(cx)));
    press(cx, Role::Button, "Split");
    let writes = sent(&screen, cx);
    assert_eq!(writes.len(), 3);
    let Write::Upsert(upsert) = &writes[0] else { panic!("the upsert comes first") };
    let areas: Vec<&str> = upsert.areas.iter().map(|area| area.slug.as_str()).collect();
    assert_eq!(areas, ["running-a-job", "starting-jobs", "job-thread-and-terminal", "pull-requests-and-review"], "the new area stands right after the one it is split from, and the pending group is not kept");
    assert_eq!(upsert.areas[1].title, "Starting Jobs");
    assert!(upsert.features.is_empty());
    let moved: Vec<(&str, Option<&str>)> = writes[1..]
        .iter()
        .map(|write| match write {
            Write::Change(change) => (change.feature.as_str(), change.area.as_deref()),
            _ => panic!("a change"),
        })
        .collect();
    assert_eq!(moved, [("start-a-job", Some("starting-jobs")), ("quick-dispatch-0", Some("starting-jobs"))]);
}

#[gpui::test]
fn split_is_off_for_a_name_an_area_already_has(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::library_with(5));
    press(cx, Role::Button, "Split Running a Job");
    press(cx, Role::CheckBox, "Start a Job");
    name_the_area(&screen, cx, "Job thread and terminal");
    assert!(screen.read_with(cx, |screen, cx| !screen.split_ready(cx) && screen.split_why(cx) == "An area has that name"));
}

#[gpui::test]
fn escape_closes_the_sheet_the_editor_and_the_picker(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::library_with(5));
    press(cx, Role::Button, "Split Running a Job");
    cx.simulate_keystrokes("escape");
    assert!(screen.read_with(cx, |screen, _| screen.curate.sheet.is_none()), "Esc closes the sheet");
    press(cx, Role::Button, &rename_name(TITLE));
    cx.simulate_keystrokes("escape");
    assert!(screen.read_with(cx, |screen, _| screen.curate.form.is_none()), "Esc closes the editor");
    press(cx, Role::Button, &merge_name(TITLE));
    cx.simulate_keystrokes("escape");
    assert!(screen.read_with(cx, |screen, _| screen.curate.menu.is_none()), "Esc closes the picker");
    assert!(sent(&screen, cx).is_empty(), "closing sends nothing");
}

#[gpui::test]
fn tab_goes_into_the_pending_row_s_buttons_and_enter_presses_the_one_it_is_on(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    screen.update(cx, |screen, cx| {
        screen.cursor = Some(PENDING.into());
        cx.notify();
    });
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.focus_list(window, cx)));
    cx.simulate_keystrokes("tab");
    let focused = |cx: &mut VisualTestContext, at: usize| cx.update(|window, cx| screen.read(cx).curate.stops[PENDING][at].is_focused(window));
    assert!(focused(cx, 0), "Tab lands on Confirm");
    cx.simulate_keystrokes("tab");
    assert!(focused(cx, 1), "then Merge into");
    cx.simulate_keystrokes("shift-tab");
    assert!(focused(cx, 0), "Shift+Tab goes back");
    cx.simulate_keystrokes("shift-tab");
    assert!(cx.update(|window, cx| screen.read(cx).list_focus.is_focused(window)), "and out to the list");
    cx.simulate_keystrokes("tab tab tab");
    assert!(focused(cx, 2), "Rename");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(drawn(cx, Role::Form, &format!("Rename {TITLE}")), "Enter presses the button the keys are on");
}

#[test]
fn a_slug_is_made_from_a_name_in_lower_case_and_dashes() {
    assert_eq!(curate::slug_of("Starting Jobs"), "starting-jobs");
    assert_eq!(curate::slug_of("  Pull requests & review! "), "pull-requests-review");
    assert_eq!(curate::slug_of("!!!"), "");
}

#[test]
fn a_confirm_sends_back_the_prefixes_the_feature_holds() {
    let library = mock::library();
    let page = mock::page("start-a-job");
    let Write::Upsert(upsert) = curate::confirm_write("armada", &library.areas[3].features[0], "running-a-job", " Title ", " Line ", Some(&page)) else { panic!("an upsert") };
    assert_eq!(upsert.features[0].paths, page.paths);
    assert_eq!(upsert.features[0].entry_points, page.entry_points);
    assert_eq!((upsert.features[0].title.as_str(), upsert.features[0].summary.as_str()), ("Title", "Line"));
}
