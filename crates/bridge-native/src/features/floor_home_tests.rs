use gpui::{AppContext, Entity, Role, TestAppContext, VisualTestContext};
use ipc::FeatureLibrary;

use super::floor_home::{cards, code_map};
use super::mock;
use crate::jobs::{JobsList, JobsScreen};
use crate::look::token;
use crate::operator::OperatorScreen;

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

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
    cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).map(|one| one.bounds))
}

/// The Cockpit on the walk's Jobs holding `library`, the Operator switched off or left on.
fn cockpit(library: FeatureLibrary, off: bool, cx: &mut TestAppContext) -> (Entity<OperatorScreen>, &mut VisualTestContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| {
        let records = crate::mock::jobs("jobs-many").expect("the fixture decodes");
        let list = cx.new(|cx| JobsList::mock(&records, cx));
        let threads = crate::operator::scenes::page_threads().expect("the threads decode");
        let jobs = cx.new(|cx| JobsScreen::new(list, Some(threads), window, cx));
        OperatorScreen::mock_cockpit_holding(jobs, library, window, cx)
    });
    redraw(cx);
    if off {
        screen.read_with(cx, |screen, _| screen.page.clone()).update(cx, |page, cx| page.scene("off", cx).expect("the off scene"));
        redraw(cx);
    }
    (screen, cx)
}

fn without_features() -> FeatureLibrary {
    FeatureLibrary { areas: Vec::new(), ..mock::library() }
}

#[gpui::test]
fn off_with_features_the_right_side_is_the_feature_map_under_a_switch_on(cx: &mut TestAppContext) {
    let (_screen, cx) = cockpit(mock::floor_library(), true, cx);
    assert!(drawn(cx, Role::Region, "Floor").is_some(), "the floor stands in the page's place");
    assert!(drawn(cx, Role::Region, "Feature map").is_some(), "the project holds features: the feature map");
    assert!(drawn(cx, Role::Region, "Code map").is_none(), "and no code map");
    assert!(drawn(cx, Role::Status, "Operator off").is_some(), "its header says the Operator is off");
    assert!(drawn(cx, Role::Button, "Switch the Operator on").is_some(), "with the switch above the map");
    assert!(drawn(cx, Role::Heading, "Running a Job").is_some(), "an area is a heading");
    assert!(drawn(cx, Role::ListItem, "Start a Job").is_some(), "a feature is a row");
    assert!(drawn(cx, Role::Button, "Native Bridge foundation, Working").is_some(), "a Job on a feature is a chip that names its state");
    assert!(drawn(cx, Role::Button, "Fix the merge line freshness chip, Needs you").is_some());
    assert!(drawn(cx, Role::ListItem, "Agent notes on a diff").is_none(), "a pending feature is not on the map yet");
    assert!(drawn(cx, Role::Button, "Armada").is_some() && drawn(cx, Role::Button, "Pocket").is_some(), "one chip for each project");
}

#[gpui::test]
fn off_with_no_features_the_right_side_is_the_code_map_under_the_same_switch_on(cx: &mut TestAppContext) {
    let (_screen, cx) = cockpit(without_features(), true, cx);
    assert!(drawn(cx, Role::Region, "Floor").is_some());
    assert!(drawn(cx, Role::Region, "Code map").is_some(), "no feature in the catalog: the modules");
    assert!(drawn(cx, Role::Region, "Feature map").is_none());
    assert!(drawn(cx, Role::Button, "Switch the Operator on").is_some(), "the switch is above the code map as it is above the feature map");
    let said = code_map(&mock::overview());
    assert!(!said.rooms.is_empty() && said.doors.is_empty(), "the rooms of the overview, and no doors");
    assert!(drawn(cx, Role::Image, &super::floor_map::said(&said)).is_some(), "the plan is drawn and named by what it holds");
}

#[gpui::test]
fn a_press_on_switch_on_turns_the_operator_on_and_the_page_comes_back_with_no_restart(cx: &mut TestAppContext) {
    let (screen, cx) = cockpit(mock::floor_library(), true, cx);
    let at = drawn(cx, Role::Button, "Switch the Operator on").expect("the switch is drawn");
    cx.simulate_click(at.center(), gpui::Modifiers::none());
    redraw(cx);
    let page = screen.read_with(cx, |screen, _| screen.page.clone());
    assert!(page.read_with(cx, |page, _| !page.is_off()), "the Operator reads on");
    assert!(drawn(cx, Role::Region, "Floor").is_none(), "the floor is gone");
    assert!(drawn(cx, Role::Button, "Switch the Operator on").is_none());
    assert!(drawn(cx, Role::TextInput, "Message").is_some(), "the Operator's page is at the right again");
}

#[gpui::test]
fn with_the_operator_on_there_is_no_floor(cx: &mut TestAppContext) {
    let (_screen, cx) = cockpit(mock::floor_library(), false, cx);
    assert!(drawn(cx, Role::Region, "Floor").is_none());
    assert!(drawn(cx, Role::Region, "Feature map").is_none());
    assert!(drawn(cx, Role::TextInput, "Message").is_some(), "the page is drawn");
}

#[gpui::test]
fn a_job_open_takes_the_right_side_from_the_floor(cx: &mut TestAppContext) {
    let (screen, cx) = cockpit(mock::floor_library(), true, cx);
    let row = drawn(cx, Role::Row, "Fix the merge line freshness chip").expect("the Job's row is drawn");
    cx.simulate_click(row.center(), gpui::Modifiers::none());
    redraw(cx);
    let home = screen.read_with(cx, |screen, cx| screen.page_shown(cx));
    assert!(!home, "the press opened the Job, so the Cockpit is no longer at home");
    assert!(drawn(cx, Role::Region, "Floor").is_none(), "an open Job fills the right side");
}

#[gpui::test]
fn a_press_on_a_jobs_chip_opens_that_job(cx: &mut TestAppContext) {
    let (screen, cx) = cockpit(mock::floor_library(), true, cx);
    let chip = drawn(cx, Role::Button, "Native Bridge foundation, Working").expect("the chip is drawn");
    cx.simulate_click(chip.center(), gpui::Modifiers::none());
    redraw(cx);
    let home = screen.read_with(cx, |screen, cx| screen.page_shown(cx));
    assert!(!home, "the press opened the Job, so the Cockpit is no longer at home");
    assert!(drawn(cx, Role::Region, "Floor").is_none(), "the Job opens over the floor");
}

#[gpui::test]
fn a_press_on_a_project_chip_shows_that_projects_areas(cx: &mut TestAppContext) {
    let (_screen, cx) = cockpit(mock::floor_library(), true, cx);
    let pocket = drawn(cx, Role::Button, "Pocket").expect("the project's chip is drawn");
    cx.simulate_click(pocket.center(), gpui::Modifiers::none());
    redraw(cx);
    assert!(drawn(cx, Role::Heading, "Sync and pairing").is_some(), "Pocket's own areas");
    assert!(drawn(cx, Role::Heading, "Running a Job").is_none());
}

/// The Jobs standing in the groups the chip test names: the fixture leaves 04 and 14 idle, so 04 gets a pull
/// request whose checks still run, and 14 a build to try as its one open item, as the Jobs list's own tests stage them.
fn stand(records: &mut [ipc::SessionRecord]) {
    let record = |id: &str, records: &mut [ipc::SessionRecord]| records.iter().position(|job| job.id.as_str() == id).expect("the fixture holds the Job");
    let at = record("mock-04", records);
    records[at].running.clear();
    records[at].attachments.retain(|one| one.kind != "pr");
    let pr = r#"{"kind":"pr","target":"41","state":"standing","detail":{"state":"open","checks":"pending"},"since":"2026-11-12T09:30:00Z","changed_at":"2026-11-12T09:30:00Z"}"#;
    records[at].attachments.push(ipc::decode::<ipc::Attachment>("Attachment", pr.as_bytes()).expect("an attachment"));
    let at = record("mock-14", records);
    records[at].running.clear();
    records[at].mark = Some(ipc::JobMark { state: ipc::JobState::Waiting, lamp: ipc::JobLamp::Needs, group: ipc::JobGroup::Needs });
    let item = r#"{"id":"try:j","text":"Ready to try","since":"2026-11-09T14:20:00Z","source":"ready_to_try","artifact":"try:j"}"#;
    records[at].waiting_for = vec![ipc::decode::<ipc::WaitingItem>("WaitingItem", item.as_bytes()).expect("an item")];
}

#[test]
fn a_chip_wears_the_hue_of_the_group_the_jobs_list_puts_that_job_in_and_a_stranger_is_muted() {
    let mut records = crate::mock::jobs("jobs-many").expect("the fixture decodes");
    stand(&mut records);
    let library = mock::library_on(&[("start-a-job", "mock-02"), ("start-a-job", "mock-14"), ("start-a-job", "mock-04"), ("start-a-job", "mock-01"), ("terminal-jobs", "someone-else")]);
    let drawn = cards(&library, &records);
    let chips = |slug: &str| drawn.iter().flat_map(|card| &card.places).find(|place| place.slug == slug).expect("the feature is on the map").jobs.clone();
    let on: Vec<_> = chips("start-a-job").into_iter().map(|chip| (chip.said, chip.hue)).collect();
    assert_eq!(on, [("Working", token::STATUS_RUNNING), ("Ready to try", token::STATUS_COMPLETED_SUCCESS), ("Waiting on ci", token::STATUS_NOT_STARTED), ("Needs you", token::STATUS_AWAITING_REVIEW)]);
    let stranger = chips("terminal-jobs");
    assert_eq!((stranger[0].open.clone(), stranger[0].hue), (None, token::FG_MUTED), "a Job the list does not hold has no hue to join and nothing to open");
    assert_eq!(chips("start-a-job")[0].open.as_deref(), Some("mock-02"));
}

#[test]
fn the_map_holds_no_pending_area_and_no_area_with_nothing_in_it() {
    let library = mock::library();
    assert!(library.areas.iter().any(|area| area.slug == "pending"), "the mock library does hold a pending area");
    let shown: Vec<_> = cards(&library, &[]).into_iter().map(|card| card.slug).collect();
    assert_eq!(shown, ["running-a-job", "job-thread-and-terminal", "pull-requests-and-review"]);
    assert!(cards(&without_features(), &[]).is_empty(), "a library with no area has no card, so the code map stands in");
}
