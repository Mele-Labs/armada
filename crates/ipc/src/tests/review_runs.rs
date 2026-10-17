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
