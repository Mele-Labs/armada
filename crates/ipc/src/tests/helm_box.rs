//! What rides with a message to Helm and what the message box runs on: attachments, tags, files,
//! the facts `opened` carries, `facts` itself, `TuneHelm`, and the issue read. An absent field is
//! left out of the JSON, never written as `null`.

use crate::{
    decode, encode, AskHelm, HelmAsked, HelmBoxFacts, HelmMessage, HelmOpened, Instant, IssueRead,
    IssueState, ManifestId, ProtocolId, SentFile, SessionTag, SessionUpload, TagKind, TuneHelm,
};

fn issue_tag() -> SessionTag {
    SessionTag {
        kind: TagKind::Issue,
        id: "214".to_string(),
        title: "#214 The title".to_string(),
        job: None,
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
    let json = encode(&HelmMessage::Asked(asked.clone())).expect("plain data");
    assert!(json.contains(r#""files":[{"id":"f1""#), "{json}");
    assert_eq!(
        decode::<HelmMessage>("a Helm message", json.as_bytes()).expect("round-trips"),
        HelmMessage::Asked(asked)
    );

    let plain = encode(&HelmMessage::Asked(HelmAsked {
        ts: Instant::carried("2026-09-13T09:00:00.000Z"),
        text: "this".to_string(),
        files: Vec::new(),
        tags: Vec::new(),
    }))
    .expect("plain data");
    assert!(!plain.contains("files") && !plain.contains("tags"), "{plain}");
}

#[test]
fn opened_always_carries_facts_and_facts_is_its_own_message() {
    let opened = HelmMessage::Opened(HelmOpened {
        protocol_id: ProtocolId::current(),
        manifest_id: ManifestId::carried("armada"),
        replying: false,
        skipped: 0,
        facts: HelmBoxFacts::default(),
    });
    let json = encode(&opened).expect("plain data");
    assert!(json.contains(r#""facts":{}"#), "{json}");
    assert_eq!(
        decode::<HelmMessage>("a Helm message", json.as_bytes()).expect("round-trips"),
        opened
    );

    let facts = HelmMessage::Facts(HelmBoxFacts {
        model: Some("opus".to_string()),
        effort: None,
        commands: vec!["/help".to_string()],
    });
    let json = encode(&facts).expect("plain data");
    assert_eq!(
        json,
        r#"{"message":"facts","model":"opus","commands":["/help"]}"#
    );
    assert_eq!(
        decode::<HelmMessage>("a Helm message", json.as_bytes()).expect("round-trips"),
        facts
    );
}

#[test]
fn a_tune_leaves_out_what_it_does_not_set() {
    let tuned = TuneHelm {
        model: None,
        effort: Some("high".to_string()),
    };
    let json = encode(&tuned).expect("plain data");
    assert_eq!(json, r#"{"effort":"high"}"#);
    assert_eq!(
        decode::<TuneHelm>("a tune", br#"{}"#).expect("empty"),
        TuneHelm {
            model: None,
            effort: None
        }
    );
    assert_eq!(
        decode::<TuneHelm>("a tune", json.as_bytes()).expect("round-trips"),
        tuned
    );
}

#[test]
fn an_issue_read_spells_its_state_in_snake_case() {
    let read = IssueRead {
        number: 214,
        title: "The title".to_string(),
        url: "https://forge.invalid/o/r/issues/214".to_string(),
        state: IssueState::Closed,
    };
    let json = encode(&read).expect("plain data");
    assert!(json.contains(r#""state":"closed""#), "{json}");
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
