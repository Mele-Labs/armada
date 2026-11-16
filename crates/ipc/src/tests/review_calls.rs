//! A review asked of him on the calls wire: its kinds in Bridge's words, the pull request it names, and
//! a count of its own, counted on the Dock too until he opens it.

use crate::{decode, encode, CallArrivalKind, StandingCall, StandingCallKind, StandingCalls};

fn call(key: &str, kind: &str, opened: bool) -> StandingCall {
    let review = if kind == "review_requested" {
        r#","review":{"manifest_id":"01MANIFEST","repository":"example-org/harbour","number":41,"title":"Moor the ferry","author":"someone","address":"https://example.invalid/41"}"#
    } else {
        ""
    };
    let json = format!(
        r#"{{"key":"{key}","owner":"{key}","kind":"{kind}","title":"t","text":"x","since":"2026-11-16T09:00:00.000Z","tone":"waiting","opened":{opened}{review}}}"#
    );
    decode::<StandingCall>("a call", json.as_bytes()).expect("decodes")
}

#[test]
fn a_review_asked_of_him_is_review_requested_on_both_kinds_and_names_its_pull_request() {
    assert_eq!(encode(&StandingCallKind::ReviewRequested).expect("encodes"), r#""review_requested""#);
    assert_eq!(encode(&CallArrivalKind::ReviewRequested).expect("encodes"), r#""review_requested""#);
    let asked = call("review:example-org/harbour#41", "review_requested", false);
    let review = asked.review.as_ref().expect("names its pull request");
    assert_eq!((review.repository.as_str(), review.number, review.author.as_str()), ("example-org/harbour", 41, "someone"));
    let main = encode(&call("main:/x", "main", false)).expect("encodes");
    assert!(!main.contains("review"), "absent on every other kind: {main}");
}

#[test]
fn the_reviews_count_is_his_unopened_requests_and_the_dock_counts_them_too() {
    let calls = StandingCalls {
        calls: vec![
            call("review:example-org/harbour#41", "review_requested", false),
            call("review:example-org/harbour#42", "review_requested", false),
            call("review:example-org/harbour#43", "review_requested", true),
            call("main:/x", "main", false),
        ],
    };
    assert_eq!(calls.reviews_unopened(), 2);
    let dock: Vec<&str> = calls.unopened().iter().map(|one| one.key.as_str()).collect();
    assert_eq!(dock, ["review:example-org/harbour#41", "review:example-org/harbour#42", "main:/x"], "an opened request drops off the Dock");
}
