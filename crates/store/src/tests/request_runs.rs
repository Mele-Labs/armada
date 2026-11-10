//! The runs of a saved request: newest first, the filters, the cap that drops the oldest, another
//! request's runs left alone, and every run kept across a reopen.

use crate::tests::{open, TempDir};
use crate::{NewRun, RunsAsked, StatusWanted, RUNS_KEPT};

fn run(request: &str, by: &str, status: Option<u16>, at: &str) -> NewRun {
    NewRun {
        repository: "/work/shop".to_string(),
        collection_id: "users".to_string(),
        request_id: request.to_string(),
        sent_at: at.to_string(),
        sent_by: by.to_string(),
        session_id: (by == "agent").then(|| "job-1".to_string()),
        session_title: (by == "agent").then(|| "Fix the login".to_string()),
        environment: Some("staging".to_string()),
        method: "POST".to_string(),
        url: "https://staging.example.test/v1/me".to_string(),
        request_headers: vec![("Authorization".to_string(), "Bearer {{token}}".to_string())],
        request_body: Some("{\"who\":\"me\"}".to_string()),
        status,
        ms: 12,
        response_headers: vec![("content-type".to_string(), "application/json".to_string())],
        response_body: "{}".to_string(),
        response_bytes: 2,
        response_cut: false,
        error: status.is_none().then(|| "connection refused".to_string()),
    }
}

fn asked<'a>(request: &'a str) -> RunsAsked<'a> {
    RunsAsked {
        repository: "/work/shop",
        collection_id: "users",
        request_id: request,
        sent_by: None,
        status: None,
        before: None,
        most: 50,
    }
}

fn statuses(store: &crate::Store, asked: &RunsAsked<'_>) -> Vec<Option<u16>> {
    store.request_runs(asked).expect("reads").iter().map(|kept| kept.run.status).collect()
}

#[test]
fn runs_come_back_newest_first_whole_and_survive_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.keep_request_run(&run("me", "owner", Some(200), "2026-11-10T10:00:00Z")).expect("kept");
    let second = store.keep_request_run(&run("me", "agent", Some(500), "2026-11-10T11:00:00Z")).expect("kept");
    assert!(second > first, "an id rises with each run");
    drop(store);

    let store = open(&dir);
    let runs = store.request_runs(&asked("me")).expect("reads");
    assert_eq!(runs.iter().map(|kept| kept.id).collect::<Vec<_>>(), [second, first]);
    assert_eq!(runs[0].run, run("me", "agent", Some(500), "2026-11-10T11:00:00Z"), "kept whole, headers and body too");
    assert_eq!(store.request_run("/work/shop", first).expect("reads").map(|kept| kept.run.status), Some(Some(200)));
    assert_eq!(store.request_run("/work/other", first).expect("reads"), None, "another repository's run is not found");
}

#[test]
fn a_request_keeps_its_newest_runs_and_the_oldest_go_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_request_run(&run("other", "owner", Some(200), "2026-11-10T09:00:00Z")).expect("kept");
    let mut ids = Vec::new();
    for turn in 0..RUNS_KEPT + 3 {
        let mut one = run("me", "owner", Some(200), "2026-11-10T10:00:00Z");
        one.ms = turn as u64;
        ids.push(store.keep_request_run(&one).expect("kept"));
    }
    let mut every = asked("me");
    every.most = RUNS_KEPT + 50;
    let runs = store.request_runs(&every).expect("reads");
    assert_eq!(runs.len(), RUNS_KEPT, "no more than the cap");
    assert_eq!(runs[0].run.ms, (RUNS_KEPT + 2) as u64, "the newest is kept");
    assert_eq!(runs.last().map(|kept| kept.run.ms), Some(3), "the three oldest were dropped");
    assert_eq!(store.request_run("/work/shop", ids[0]).expect("reads"), None);
    assert_eq!(store.request_runs(&asked("other")).expect("reads").len(), 1, "another request's runs are not counted");
}

#[test]
fn the_filters_take_a_sender_and_a_status_class_and_a_page_ends_where_it_is_told() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (by, status) in [("owner", Some(200)), ("agent", Some(404)), ("agent", Some(201)), ("owner", None), ("agent", Some(503))] {
        store.keep_request_run(&run("me", by, status, "2026-11-10T10:00:00Z")).expect("kept");
    }
    let mut filtered = asked("me");
    filtered.sent_by = Some("agent");
    assert_eq!(statuses(&store, &filtered), [Some(503), Some(201), Some(404)]);
    filtered.status = Some(StatusWanted::Class(2));
    assert_eq!(statuses(&store, &filtered), [Some(201)]);
    let mut failed = asked("me");
    failed.status = Some(StatusWanted::Failed);
    assert_eq!(statuses(&store, &failed), [None]);
    let mut client_errors = asked("me");
    client_errors.status = Some(StatusWanted::Class(4));
    assert_eq!(statuses(&store, &client_errors), [Some(404)]);

    let mut page = asked("me");
    page.most = 2;
    let first = store.request_runs(&page).expect("reads");
    assert_eq!(first.len(), 2);
    page.before = Some(first[1].id);
    assert_eq!(statuses(&store, &page), [Some(201), Some(404)], "the next page starts after the last one");
}
