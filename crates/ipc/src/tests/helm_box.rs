//! What rides with a message to Helm: attachments and tags, and the issue read. An absent field is
//! left out of the JSON, never written as `null`.

use crate::{
    decode, encode, AskHelm, HelmAsked, Instant, IssueComment, IssueRead, IssueState,
    LinkedPullRequest, LinkedPullRequestState, SentFile, SessionTag, SessionUpload, TagKind,
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
fn an_ask_with_attachments_and_mentions_round_trips() {
    let asked = AskHelm {
        text: "look at this".to_string(),
        attachments: vec![SessionUpload {
            name: "shot.png".to_string(),
            media_type: "image/png".to_string(),
            data: "aGk=".to_string(),
        }],
        mentions: vec![issue_tag()],
        model: None,
        effort: None,
        context: None,
    };
    let json = encode(&asked).expect("plain data");
    assert!(json.contains(r#""kind":"issue""#), "{json}");
    assert_eq!(
        decode::<AskHelm>("a message to Helm", json.as_bytes()).expect("round-trips"),
        asked
    );
}

#[test]
fn a_blank_text_with_an_attachment_decodes_and_a_words_only_ask_carries_neither_field() {
    let asked = decode::<AskHelm>(
        "a message to Helm",
        br#"{"text":"","attachments":[{"name":"a.txt","media_type":"text/plain","data":"aGk="}]}"#,
    )
    .expect("blank text is decoded; refusing it is Fleet's");
    assert_eq!(asked.text, "");
    assert_eq!(asked.attachments.len(), 1);

    let words = AskHelm {
        text: "hi".to_string(),
        attachments: Vec::new(),
        mentions: Vec::new(),
        model: None,
        effort: None,
        context: None,
    };
    assert_eq!(encode(&words).expect("plain data"), r#"{"text":"hi"}"#);
}

#[test]
fn an_asked_row_keeps_what_it_carried_and_leaves_out_what_it_did_not() {
    let asked = HelmAsked {
        ts: Instant::carried("2026-09-13T09:00:00.000Z"),
        text: "this".to_string(),
        files: vec![SentFile {
            id: "f1".to_string(),
            name: "shot.png".to_string(),
            media_type: "image/png".to_string(),
        }],
        tags: vec![issue_tag()],
    };
    let json = encode(&asked).expect("plain data");
    assert!(json.contains(r#""files":[{"id":"f1""#), "{json}");
    assert_eq!(decode::<HelmAsked>("an asked row", json.as_bytes()).expect("round-trips"), asked);

    let plain = encode(&HelmAsked {
        ts: Instant::carried("2026-09-13T09:00:00.000Z"),
        text: "this".to_string(),
        files: Vec::new(),
        tags: Vec::new(),
    })
    .expect("plain data");
    assert!(!plain.contains("files") && !plain.contains("tags"), "{plain}");
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
