//! A pull request's agent runs, kept per commit and still there after a restart.

use crate::tests::{open, TempDir};
use crate::KeptReviewRun;

fn run(head: &str, kind: &str, name: &str, started: &str) -> KeptReviewRun {
    KeptReviewRun {
        manifest_id: "storefront".to_string(),
        number: 7,
        head: head.to_string(),
        kind: kind.to_string(),
        name: name.to_string(),
        label: name.to_string(),
        state: "done".to_string(),
        started_at: started.to_string(),
        finished_at: Some(started.to_string()),
        failed: None,
        result: Some(r#"{"shape":"text","body":"x"}"#.to_string()),
        cost_micros: Some(120_000),
        slot: None,
        holder: None,
    }
}

#[test]
fn a_run_is_kept_at_its_key_and_read_back_whole() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let kept = run("aaa", "tool", "risks", "2026-10-16T10:00:00Z");
    store.keep_review_run(&kept).expect("kept");
    assert_eq!(store.review_run("storefront", 7, "aaa", "tool", "risks").expect("read"), Some(kept));
    assert_eq!(store.review_run("storefront", 7, "bbb", "tool", "risks").expect("read"), None);
    assert_eq!(store.review_run("storefront", 8, "aaa", "tool", "risks").expect("read"), None);
}

#[test]
fn a_new_head_leaves_the_old_result_where_it_was() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_review_run(&run("aaa", "tool", "risks", "2026-10-16T10:00:00Z")).unwrap();
    store.keep_review_run(&run("bbb", "tool", "risks", "2026-10-16T11:00:00Z")).unwrap();
    store.keep_review_run(&run("aaa", "explanation", "why", "2026-10-16T09:00:00Z")).unwrap();
    let all = store.review_runs("storefront", 7).expect("read");
    let heads: Vec<(&str, &str)> = all.iter().map(|one| (one.head.as_str(), one.name.as_str())).collect();
    assert_eq!(heads, [("bbb", "risks"), ("aaa", "risks"), ("aaa", "why")], "newest started first");
    assert!(store.review_runs("storefront", 8).unwrap().is_empty());
}

#[test]
fn running_it_again_replaces_the_row_at_that_key() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut going = run("aaa", "ask", "what changed", "2026-10-16T10:00:00Z");
    going.state = "running".to_string();
    going.result = None;
    going.slot = Some(3);
    going.holder = Some("review-1".to_string());
    store.keep_review_run(&going).unwrap();
    assert_eq!(store.review_runs_going().unwrap(), [going.clone()]);
    let done = KeptReviewRun { state: "done".to_string(), slot: None, holder: None, result: Some("{}".to_string()), ..going };
    store.keep_review_run(&done).unwrap();
    assert!(store.review_runs_going().unwrap().is_empty());
    assert_eq!(store.review_runs("storefront", 7).unwrap(), [done]);
}

#[test]
fn a_cost_that_was_never_said_is_not_a_cost_of_zero() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let unsaid = KeptReviewRun { cost_micros: None, ..run("aaa", "tool", "risks", "2026-10-16T10:00:00Z") };
    store.keep_review_run(&unsaid).unwrap();
    assert_eq!(store.review_run("storefront", 7, "aaa", "tool", "risks").unwrap().unwrap().cost_micros, None);
}

#[test]
fn forgetting_a_run_drops_it_at_every_head_and_is_not_an_error_when_it_is_gone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_review_run(&run("aaa", "tool", "risks", "2026-10-16T10:00:00Z")).unwrap();
    store.keep_review_run(&run("bbb", "tool", "risks", "2026-10-16T11:00:00Z")).unwrap();
    store.keep_review_run(&run("aaa", "tool", "bugs", "2026-10-16T09:00:00Z")).unwrap();
    store.forget_review_run("storefront", 7, "tool", "risks").unwrap();
    store.forget_review_run("storefront", 7, "tool", "risks").unwrap();
    let left: Vec<String> = store.review_runs("storefront", 7).unwrap().into_iter().map(|one| one.name).collect();
    assert_eq!(left, ["bugs"]);
}

#[test]
fn the_pull_requests_run_on_are_named_once_each_by_their_latest_run_newest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_review_run(&run("aaa", "tool", "risks", "2026-10-16T10:00:00Z")).unwrap();
    store.keep_review_run(&run("bbb", "overview", "overview", "2026-10-16T12:00:00Z")).unwrap();
    let other = KeptReviewRun { number: 9, ..run("ccc", "change_sets", "change-sets", "2026-10-16T11:00:00Z") };
    store.keep_review_run(&other).unwrap();
    let asked = KeptReviewRun { number: 11, ..run("ddd", "ask", "what changed", "2026-10-16T13:00:00Z") };
    store.keep_review_run(&asked).unwrap();
    let search = KeptReviewRun { number: 0, ..run("", "tool", "risks", "2026-10-16T14:00:00Z") };
    store.keep_review_run(&search).unwrap();
    let kinds = ["tool", "change_sets", "overview"];
    let named: Vec<(u64, String)> =
        store.review_run_pulls(&kinds).unwrap().into_iter().map(|one| (one.number, one.latest)).collect();
    assert_eq!(
        named,
        [(7, "2026-10-16T12:00:00Z".to_string()), (9, "2026-10-16T11:00:00Z".to_string())],
        "an ask and the search scope are not runs of a tool"
    );
    assert!(store.review_run_pulls(&[]).unwrap().is_empty());
}
