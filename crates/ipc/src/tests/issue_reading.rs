//! What rides with a message: a tag, and the issue read. An absent field is left out of the JSON,
//! never written as `null`.

use crate::{
    decode, encode, Instant, IssueComment, IssueRead, IssueState, LinkedPullRequest, LinkedPullRequestState,
    SessionTag, TagKind,
};

fn issue_tag() -> SessionTag {
    SessionTag {
        kind: TagKind::Issue,
        id: "214".to_string(),
        title: "#214 The title".to_string(),
        url: Some("https://forge.invalid/o/r/issues/214".to_string()),
    }
}

#[test]
fn a_reported_bug_rides_a_message_as_a_tag_of_its_own_kind() {
    let tag = SessionTag {
        kind: TagKind::Report,
        id: "01jreport".to_string(),
        title: "Preview restart drops the lease".to_string(),
        url: None,
    };
    let json = encode(&tag).expect("plain data");
    assert!(json.contains(r#""kind":"report""#), "{json}");
    assert!(!json.contains("url"), "{json}");
    assert_eq!(decode::<SessionTag>("a tag", json.as_bytes()).expect("round-trips"), tag);
}

#[test]
fn an_issue_read_spells_its_state_in_snake_case() {
    let read = IssueRead {
        reference: "#214".to_string(),
        title: "The title".to_string(),
        url: "https://forge.invalid/o/r/issues/214".to_string(),
        state: IssueState::Closed,
        body: "The body".to_string(),
        author: "nick".to_string(),
        opened_at: Instant::carried("2026-10-01T09:00:00Z"),
        labels: vec!["bug".to_string()],
        assignees: vec!["nick".to_string()],
        comments: vec![IssueComment {
            author: "sam".to_string(),
            at: Instant::carried("2026-10-02T10:00:00Z"),
            body: "Seen here too".to_string(),
        }],
        linked_pull_requests: vec![LinkedPullRequest {
            number: 230,
            title: "Fix it".to_string(),
            url: "https://forge.invalid/o/r/pull/230".to_string(),
            state: LinkedPullRequestState::Merged,
        }],
        fields: vec![crate::IssueField { name: "Labels".to_string(), values: vec!["bug".to_string()] }],
        holders: vec!["job-1".to_string()],
    };
    let json = encode(&read).expect("plain data");
    assert!(json.contains(r#""fields":[{"name":"Labels","values":["bug"]}]"#), "{json}");
    assert!(json.contains(r#""holders":["job-1"]"#), "{json}");
    assert!(json.contains(r#""state":"closed""#), "{json}");
    assert!(json.contains(r#""state":"merged""#), "{json}");
    assert!(json.contains(r#""opened_at":"2026-10-01T09:00:00Z""#), "{json}");
    assert_eq!(
        decode::<IssueRead>("an issue", json.as_bytes()).expect("round-trips"),
        read
    );
}

#[test]
fn a_tag_without_a_url_carries_none_and_one_with_it_does() {
    let mut tag = issue_tag();
    assert!(encode(&tag).expect("plain data").contains(r#""url":"https"#));
    tag.url = None;
    let json = encode(&tag).expect("plain data");
    assert!(!json.contains("url") && !json.contains("null"), "{json}");
}
