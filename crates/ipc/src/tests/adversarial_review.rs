//! What an adversarial review puts on the wire: the subject and the anchor are tagged by `kind`,
//! an empty field leaves no key, and a review reads back as it was written.

use crate::{
    decode, encode, AdversarialFinding, AdversarialReview, AdversarialReviewState, FindingAnchor, FindingHow,
    FindingSettled, Instant, ReviewSubject, SessionId, StartAdversarialReview,
};

fn subject() -> ReviewSubject {
    ReviewSubject::Plan { session_id: SessionId::carried("session-1"), version: 3 }
}

fn a_review() -> AdversarialReview {
    AdversarialReview {
        subject: subject(),
        state: AdversarialReviewState::Running,
        started_at: Instant::carried("2026-10-25T09:00:00.000Z"),
        finished_at: None,
        failed: None,
        findings: Vec::new(),
        cost_micros: None,
    }
}

#[test]
fn a_subject_is_tagged_by_its_kind() {
    let text = encode(&subject()).expect("encodes");
    assert_eq!(text, r#"{"kind":"plan","session_id":"session-1","version":3}"#);
    assert_eq!(decode::<ReviewSubject>("a subject", text.as_bytes()).expect("decodes"), subject());
}

#[test]
fn a_review_with_nothing_to_add_leaves_no_null_keys() {
    let text = encode(&a_review()).expect("encodes");
    for absent in ["finished_at", "failed", "cost_micros", "null"] {
        assert!(!text.contains(absent), "{absent} in {text}");
    }
    assert!(text.contains(r#""state":"running""#) && text.contains(r#""findings":[]"#), "{text}");
}

#[test]
fn a_finding_is_anchored_to_the_whole_plan_a_group_or_a_task_and_reads_back_whole() {
    let finding = |id: &str, about: FindingAnchor| AdversarialFinding {
        id: id.into(),
        about,
        consequence: "A rotation can run twice".into(),
        expected: "One rotation a day".into(),
        found: "The task has no guard".into(),
        impact: None,
        cites: Vec::new(),
        settled: None,
    };
    let mut review = a_review();
    review.state = AdversarialReviewState::Done;
    review.cost_micros = Some(180_000);
    review.findings = vec![
        finding("f1", FindingAnchor::Plan),
        finding("f2", FindingAnchor::Group { id: "g1".into() }),
        AdversarialFinding {
            impact: Some("Someone rotating their keys finds two on one day".into()),
            cites: vec!["crates/store/src/lib.rs:12".into(), "crates/store".into()],
            settled: Some(FindingSettled { how: FindingHow::Sent, at: Instant::carried("2026-10-25T09:05:00.000Z") }),
            ..finding("f3", FindingAnchor::Task { id: "t1".into() })
        },
    ];
    let text = encode(&review).expect("encodes");
    assert!(text.contains(r#""about":{"kind":"plan"}"#), "{text}");
    assert!(text.contains(r#""about":{"kind":"group","id":"g1"}"#), "{text}");
    assert!(text.contains(r#""about":{"kind":"task","id":"t1"}"#), "{text}");
    assert!(text.contains(r#""settled":{"how":"sent","at":"2026-10-25T09:05:00.000Z"}"#), "{text}");
    assert_eq!(text.matches("cites").count(), 1, "an empty list leaves no key: {text}");
    assert_eq!(text.matches("impact").count(), 1, "an absent impact leaves no key: {text}");
    let without: AdversarialFinding = decode(
        "a finding",
        br#"{"id":"f9","about":{"kind":"plan"},"consequence":"c","expected":"e","found":"f"}"#,
    )
    .expect("a finding kept before impact was asked for decodes");
    assert_eq!(without.impact, None);
    assert_eq!(decode::<AdversarialReview>("a review", text.as_bytes()).expect("decodes"), review);
}

#[test]
fn a_start_without_again_reads_as_not_again() {
    let start: StartAdversarialReview =
        decode("a start", br#"{"subject":{"kind":"plan","session_id":"session-1","version":3}}"#).expect("decodes");
    assert!(!start.again);
    assert_eq!(start.subject, subject());
}
