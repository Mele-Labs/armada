//! What the floor plan draws from a library and the Jobs. Where it stands is the Cockpit home's (`home/screen_tests.rs`).

use ipc::FeatureLibrary;

use super::floor_home::cards;
use super::mock;
use crate::look::token;

fn without_features() -> FeatureLibrary {
    FeatureLibrary { areas: Vec::new(), ..mock::library() }
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
