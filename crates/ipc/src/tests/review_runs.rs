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
        since: None,
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
    for absent in ["finished_at", "failed", "result", "cost_micros", "since", "null"] {
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
            comment: "This sum can wrap past the limit. Use a saturating add.".into(),
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
fn a_tour_is_tagged_tour_and_its_stops_read_back_in_order() {
    use crate::ReviewTourStop;
    let stop = |title: &str, path: &str| ReviewTourStop {
        title: title.into(),
        at: ReviewPlace { path: path.into(), side: DiffSide::Right, start: 3, end: 9 },
    };
    let result = ReviewResult::Tour { stops: vec![stop("The route takes the request", "routes.rs"), stop("The store keeps it", "store.rs")] };
    let text = encode(&result).expect("encodes");
    assert!(text.starts_with(r#"{"shape":"tour","stops":[{"title":"The route takes the request","at":{"path":"routes.rs""#), "{text}");
    assert_eq!(decode::<ReviewResult>("a result", text.as_bytes()).expect("decodes"), result);
}

#[test]
fn a_fix_is_tagged_fix_and_its_start_carries_the_finding() {
    use crate::StartReviewRun;
    let at = ReviewPlace { path: "src/total.rs".into(), side: DiffSide::Right, start: 4, end: 5 };
    let result = ReviewResult::Fix {
        at: at.clone(),
        before: "    let total = a + b;\n    total".into(),
        after: "    a.saturating_add(b)".into(),
        why: "The sum no longer wraps.".into(),
    };
    let text = encode(&result).expect("encodes");
    assert!(text.starts_with(r#"{"shape":"fix","at":{"path":"src/total.rs","side":"right","start":4,"end":5}"#), "{text}");
    assert_eq!(decode::<ReviewResult>("a result", text.as_bytes()).expect("decodes"), result);

    let start = StartReviewRun {
        number: 12,
        kind: ReviewRunKind::Fix,
        name: "src/total.rs:4-5 Overflow".into(),
        again: false,
        place: None,
        finding: Some(ReviewFinding {
            title: "Overflow".into(),
            body: "It wraps.".into(),
            severity: ReviewSeverity::High,
            at,
            comment: String::new(),
        }),
        since: None,
    };
    let text = encode(&start).expect("encodes");
    assert!(text.contains(r#""kind":"fix""#) && text.contains(r#""finding":{"title":"Overflow""#), "{text}");
    assert_eq!(decode::<StartReviewRun>("a start", text.as_bytes()).expect("decodes"), start);
    // Any other kind leaves no `finding` key.
    let tool = StartReviewRun { kind: ReviewRunKind::Tool, finding: None, ..start };
    assert!(!encode(&tool).expect("encodes").contains("finding"));
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

#[test]
fn change_sets_overview_and_practices_read_back_whole_under_their_own_kinds() {
    use crate::{
        ChangeSet, ChangeSetKind, HunkRef, OverviewFinding, OverviewSeverity, PracticeCheck, PracticeSource,
        PracticeVerdict,
    };
    let at = ReviewPlace { path: "db/0007_orders.sql".into(), side: DiffSide::Right, start: 3, end: 3 };
    let results = [
        (ReviewRunKind::ChangeSets, "change_sets", ReviewResult::ChangeSets {
            sets: vec![ChangeSet {
                id: "set-1".into(),
                title: "Add the orders table".into(),
                kind: ChangeSetKind::Migration,
                purpose: "Orders are kept.".into(),
                area: "db".into(),
                hunks: vec![HunkRef { path: at.path.clone(), old_start: 0, old_lines: 0, new_start: 1, new_lines: 9 }],
            }],
        }),
        (ReviewRunKind::Overview, "overview", ReviewResult::Overview {
            findings: vec![OverviewFinding {
                severity: OverviewSeverity::Blocker,
                set: "set-1".into(),
                title: "Not reversible".into(),
                reason: "The migration drops a column with no way back, so add a down step.".into(),
                at: at.clone(),
            }],
        }),
        (ReviewRunKind::Practices, "practices", ReviewResult::Practices {
            set: "set-1".into(),
            kind: ChangeSetKind::Migration,
            area: "db".into(),
            read_at: "2026-11-10T09:00:00Z".into(),
            reused: true,
            sources: vec![PracticeSource { path: "docs/migrations.md".into(), blob: None }],
            checks: vec![PracticeCheck {
                item: "Every migration has a down step".into(),
                source: "docs/migrations.md".into(),
                verdict: PracticeVerdict::NotApplicable,
                note: "Only adds a table.".into(),
            }],
        }),
    ];
    for (kind, word, result) in results {
        let mut run = a_run();
        run.kind = kind;
        run.state = ReviewRunState::Done;
        run.result = Some(result);
        let text = encode(&run).expect("encodes");
        assert!(text.contains(&format!(r#""kind":"{word}""#)), "{text}");
        assert!(!text.contains("null") && !text.contains(r#""blob""#), "{text}");
        assert_eq!(decode::<ReviewRun>("a run", text.as_bytes()).expect("decodes"), run);
    }
}
