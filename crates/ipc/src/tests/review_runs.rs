//! What the Reviews tab's runs put on the wire: the event keeps its own kind beside the run's,
//! an empty field leaves no key, and a result reads back as the shape it was written in.

use crate::{
    decode, encode, DiffSide, Event, ManifestId, ReviewFinding, ReviewPlace, ReviewResult, ReviewRun,
    ReviewRunKind, ReviewRunState, ReviewSeverity,
};

fn a_run() -> ReviewRun {
    ReviewRun {
        manifest_id: ManifestId::carried("01MANIFEST"),
        number: 12,
        kind: ReviewRunKind::Tool,
        name: "risks".into(),
        head: "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e".into(),
        state: ReviewRunState::Queued,
        started_at: "2026-10-16T09:00:00Z".into(),
        finished_at: None,
        failed: None,
        result: None,
        cost_micros: None,
    }
}

#[test]
fn the_event_nests_the_run_so_neither_kind_overwrites_the_other() {
    let text = encode(&Event::ReviewRunChanged { run: a_run() }).expect("encodes");
    assert!(text.contains(r#""kind":"review_run.changed""#), "{text}");
    assert!(text.contains(r#""run":{"#) && text.contains(r#""kind":"tool""#), "{text}");
    assert!(text.contains(r#""state":"queued""#), "{text}");
}

#[test]
fn a_run_with_nothing_to_add_leaves_no_null_keys() {
    let text = encode(&a_run()).expect("encodes");
    for absent in ["finished_at", "failed", "result", "cost_micros", "null"] {
        assert!(!text.contains(absent), "{absent} in {text}");
    }
}

#[test]
fn a_result_is_tagged_by_its_shape_and_reads_back_whole() {
    let mut run = a_run();
    run.state = ReviewRunState::Done;
    run.cost_micros = Some(250_000);
    run.result = Some(ReviewResult::Findings {
        findings: vec![ReviewFinding {
            title: "Overflow".into(),
            body: "`x` can wrap.".into(),
            severity: ReviewSeverity::High,
            at: ReviewPlace { path: "a.rs".into(), side: DiffSide::Right, start: 1, end: 2 },
        }],
    });
    let text = encode(&run).expect("encodes");
    assert!(text.contains(r#""shape":"findings""#), "{text}");
    assert_eq!(decode::<ReviewRun>("a run", text.as_bytes()).expect("decodes"), run);
}

#[test]
fn a_walk_place_leaves_no_key_for_what_it_does_not_have() {
    use crate::{WalkHow, WalkPlace};
    let deployed = WalkPlace { how: WalkHow::Deployed, address: "https://pr-9.preview.invalid".into(), command: String::new(), api: String::new() };
    let text = encode(&deployed).expect("encodes");
    assert_eq!(text, r#"{"how":"deployed","address":"https://pr-9.preview.invalid"}"#);
    assert_eq!(decode::<WalkPlace>("a place", text.as_bytes()).expect("decodes"), deployed);
    let pulled = decode::<WalkPlace>("a place", br#"{"how":"pull","command":"pnpm dev","api":"http://localhost:8080"}"#).expect("decodes");
    assert_eq!((pulled.address.as_str(), pulled.command.as_str()), ("", "pnpm dev"));
}

#[test]
fn a_search_is_a_run_kept_under_no_pull_request() {
    use crate::SEARCH_RUN_SCOPE;
    let mut run = a_run();
    run.manifest_id = ManifestId::carried(SEARCH_RUN_SCOPE);
    run.number = 0;
    run.head = String::new();
    run.kind = ReviewRunKind::Search;
    run.name = "prs that touch the billing service".into();
    run.state = ReviewRunState::Done;
    run.result = Some(ReviewResult::Matches { matches: Vec::new() });
    let text = encode(&run).expect("encodes");
    assert!(text.contains(r#""kind":"search""#) && text.contains(r#""shape":"matches""#), "{text}");
    assert_eq!(decode::<ReviewRun>("a run", text.as_bytes()).expect("decodes"), run);
}

#[test]
fn a_walk_result_carries_the_place_it_was_written_against() {
    use crate::{ReviewRequest, ReviewRequestMethod, WalkHow, WalkPlace};
    let place = WalkPlace { how: WalkHow::Deployed, address: String::new(), command: String::new(), api: "https://api.preview.invalid".into() };
    let result = ReviewResult::ApiWalk {
        place,
        requests: vec![ReviewRequest {
            say: "Create an invoice".into(),
            method: ReviewRequestMethod::Post,
            path: "/invoices".into(),
            body: Some(r#"{"total":10}"#.into()),
            look: "201 and an id".into(),
        }],
    };
    let text = encode(&result).expect("encodes");
    assert!(text.contains(r#""shape":"api_walk""#) && text.contains(r#""method":"POST""#), "{text}");
    assert_eq!(decode::<ReviewResult>("a result", text.as_bytes()).expect("decodes"), result);
}

#[test]
fn the_walk_event_nests_the_walk() {
    use crate::{ReviewWalk, ReviewWalkState};
    let walk = ReviewWalk {
        id: "w1".into(),
        manifest_id: ManifestId::carried("01MANIFEST"),
        number: 12,
        state: ReviewWalkState::CheckingOut,
        slot: Some(3),
        address: "http://localhost:5173".into(),
        api: String::new(),
        failed: None,
    };
    let text = encode(&Event::ReviewWalkChanged { walk }).expect("encodes");
    assert!(text.contains(r#""kind":"review_walk.changed""#) && text.contains(r#""state":"checking_out""#), "{text}");
    assert!(!text.contains("failed") && !text.contains(r#""api""#), "{text}");
}
