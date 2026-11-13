//! What Fleet's memory of a pull request puts on the wire: the inbox state in Bridge's words, a
//! compare that lost its base as an outcome and not an error, and a row from before any of it.

use crate::{decode, encode, PullInbox, PullRequestCompare, PullReviewRow};

#[test]
fn an_inbox_state_is_written_in_snake_case_and_a_row_without_one_needs_a_first_look() {
    let text = encode(&PullInbox::ChangedSinceYouLooked).expect("encodes");
    assert_eq!(text, r#""changed_since_you_looked""#);
    let row = r#"{"manifest_id":"01MANIFEST","repository":"example-org/harbour","number":41,
        "title":"Moor the ferry","branch":"moor","address":"https://example.invalid/41","draft":false,
        "checks":"passed","review":"none","head":"bbb","author":"someone","updated_at":"2026-11-13T09:00:00Z"}"#;
    let row = decode::<PullReviewRow>("a row", row.as_bytes()).expect("a row from before decodes");
    assert_eq!((row.inbox, row.seen, row.my_review), (PullInbox::NeedsFirstLook, None, None));
}

#[test]
fn a_compare_whose_base_is_gone_is_an_outcome_bridge_can_match() {
    let gone = PullRequestCompare::BaseGone { since: "aaa".into(), head: "bbb".into() };
    let text = encode(&gone).expect("encodes");
    assert_eq!(text, r#"{"outcome":"base_gone","since":"aaa","head":"bbb"}"#);
    assert_eq!(decode::<PullRequestCompare>("a compare", text.as_bytes()).expect("decodes"), gone);
}
