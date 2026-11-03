//! A thing said on a pull request, as the spellings `packages/protocol/src/pull-requests.ts` and
//! `pull-reviews.ts` list: its id always, an edit and a bot only where there is one.

use crate::{decode, encode, PullRequestEntry, PullRequestVerdict, PullReviewThreadComment};

/// A comment never edited, by a person, leaves out `edited_at` and `bot` rather than sending `null`
/// and `false`; an edited one by an app carries both.
#[test]
fn an_entry_carries_its_id_and_an_edit_and_a_bot_only_where_there_is_one() {
    let plain = PullRequestEntry::Comment {
        id: "IC_1".into(),
        author: "ada".into(),
        bot: false,
        at: "2026-10-15T10:00:00Z".into(),
        edited_at: None,
        body: "why?".into(),
    };
    assert_eq!(
        encode(&plain).expect("plain data"),
        r#"{"kind":"comment","id":"IC_1","author":"ada","at":"2026-10-15T10:00:00Z","body":"why?"}"#
    );
    let edited = PullRequestEntry::Review {
        id: "PRR_1".into(),
        author: "lint-app".into(),
        bot: true,
        at: "2026-10-15T10:00:00Z".into(),
        edited_at: Some("2026-10-15T10:30:00Z".into()),
        verdict: PullRequestVerdict::Commented,
        body: "one".into(),
    };
    let written = encode(&edited).expect("plain data");
    assert_eq!(
        written,
        r#"{"kind":"review","id":"PRR_1","author":"lint-app","bot":true,"at":"2026-10-15T10:00:00Z","edited_at":"2026-10-15T10:30:00Z","verdict":"commented","body":"one"}"#
    );
    assert_eq!(decode::<PullRequestEntry>("an entry", written.as_bytes()).expect("it reads"), edited);
}

#[test]
fn a_thread_comment_carries_an_edit_and_a_bot_only_where_there_is_one() {
    let plain = PullReviewThreadComment {
        id: "PRRC_1".into(),
        author: "ada".into(),
        bot: false,
        at: "2026-10-15T10:00:00Z".into(),
        edited_at: None,
        body: "why?".into(),
    };
    assert_eq!(
        encode(&plain).expect("plain data"),
        r#"{"id":"PRRC_1","author":"ada","at":"2026-10-15T10:00:00Z","body":"why?"}"#
    );
    let edited = PullReviewThreadComment { bot: true, edited_at: Some("2026-10-15T10:30:00Z".into()), ..plain };
    let written = encode(&edited).expect("plain data");
    assert!(written.contains(r#""bot":true"#) && written.contains(r#""edited_at":"2026-10-15T10:30:00Z""#), "{written}");
    assert_eq!(decode::<PullReviewThreadComment>("a comment", written.as_bytes()).expect("it reads"), edited);
}
