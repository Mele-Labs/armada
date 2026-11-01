//! What a Judge puts on the wire: the kind is a snake_case string, an empty field leaves no key, a note's
//! `where` crosses under that name, and a reading reads back as it was written.

use crate::{
    decode, encode, AskJudge, Instant, JudgeArtifact, JudgeKind, JudgeNote, JudgeNoteHow, JudgeNoteSettled,
    JudgeReading, JudgeState, SessionId,
};

fn artifact() -> JudgeArtifact {
    JudgeArtifact { kind: JudgeKind::PullRequest, id: "412".into(), title: "Move sign-in to session tokens".into() }
}

fn a_reading() -> JudgeReading {
    JudgeReading {
        id: "r1".into(),
        artifact: artifact(),
        prompt: String::new(),
        state: JudgeState::Reading,
        asked_at: Instant::carried("2026-10-31T09:00:00.000Z"),
        finished_at: None,
        failed: None,
        read: false,
        notes: Vec::new(),
        cost_micros: None,
    }
}

fn a_note(id: &str) -> JudgeNote {
    JudgeNote {
        id: id.into(),
        about: None,
        claim: "A token never expires when its lifetime is zero".into(),
        r#where: None,
        expected: None,
        found: None,
        impact: None,
        cites: Vec::new(),
        settled: None,
    }
}

#[test]
fn an_artifact_kind_crosses_as_its_snake_case_name() {
    let text = encode(&artifact()).expect("encodes");
    assert_eq!(text, r#"{"kind":"pull_request","id":"412","title":"Move sign-in to session tokens"}"#);
    assert_eq!(decode::<JudgeArtifact>("an artifact", text.as_bytes()).expect("decodes"), artifact());
}

#[test]
fn a_reading_with_nothing_to_add_leaves_no_null_keys_but_keeps_what_the_card_always_reads() {
    let text = encode(&a_reading()).expect("encodes");
    for absent in ["finished_at", "failed", "cost_micros", "null"] {
        assert!(!text.contains(absent), "{absent} in {text}");
    }
    for kept in [r#""prompt":"""#, r#""state":"reading""#, r#""read":false"#, r#""notes":[]"#] {
        assert!(text.contains(kept), "{kept} in {text}");
    }
}

#[test]
fn a_failed_reading_keeps_fleets_reason_and_is_not_a_result() {
    let mut reading = a_reading();
    reading.state = JudgeState::Failed;
    reading.failed = Some("The model call timed out".into());
    reading.finished_at = Some(Instant::carried("2026-10-31T09:01:00.000Z"));
    let text = encode(&reading).expect("encodes");
    assert!(text.contains(r#""state":"failed""#) && text.contains(r#""failed":"The model call timed out""#), "{text}");
    assert_eq!(decode::<JudgeReading>("a reading", text.as_bytes()).expect("decodes"), reading);
}

#[test]
fn a_note_carries_where_under_that_name_and_reads_back_whole() {
    let mut reading = a_reading();
    reading.state = JudgeState::Done;
    reading.read = true;
    reading.cost_micros = Some(90_000);
    reading.notes = vec![
        a_note("n1"),
        JudgeNote {
            about: Some("crates/api/src/auth/tokens.rs:41".into()),
            r#where: Some("crates/api/src/auth/tokens.rs:41".into()),
            expected: Some("A lifetime of zero is refused".into()),
            found: Some("Zero means no expiry".into()),
            impact: Some("A misconfigured deploy signs people in for good".into()),
            cites: vec!["crates/api/src/auth/tokens.rs:41".into()],
            settled: Some(JudgeNoteSettled { how: JudgeNoteHow::Dismissed, at: Instant::carried("2026-10-31T09:05:00.000Z") }),
            ..a_note("n2")
        },
    ];
    let text = encode(&reading).expect("encodes");
    assert!(text.contains(r#""where":"crates/api/src/auth/tokens.rs:41""#), "{text}");
    assert!(!text.contains("r#where"), "{text}");
    assert!(text.contains(r#""settled":{"how":"dismissed","at":"2026-10-31T09:05:00.000Z"}"#), "{text}");
    assert_eq!(text.matches("cites").count(), 1, "an empty list leaves no key: {text}");
    assert_eq!(text.matches("impact").count(), 1, "an absent impact leaves no key: {text}");
    assert_eq!(decode::<JudgeReading>("a reading", text.as_bytes()).expect("decodes"), reading);
}

#[test]
fn a_note_that_is_only_a_claim_decodes() {
    let note: JudgeNote = decode("a note", br#"{"id":"n9","claim":"No task adds a test"}"#).expect("decodes");
    assert_eq!(note, JudgeNote { claim: "No task adds a test".into(), ..a_note("n9") });
}

#[test]
fn an_ask_without_a_prompt_asks_what_was_missed() {
    let ask: AskJudge = decode(
        "an ask",
        br#"{"session_id":"session-1","artifact":{"kind":"plan","id":"3","title":"The plan"}}"#,
    )
    .expect("decodes");
    assert_eq!(ask.prompt, "");
    assert_eq!(ask.session_id, SessionId::carried("session-1"));
    assert_eq!(ask.artifact.kind, JudgeKind::Plan);
}

#[test]
fn a_job_with_no_readings_leaves_no_key_and_one_with_readings_carries_them() {
    let mut record = super::session_activity::row(None);
    let text = encode(&record).expect("encodes");
    assert!(!text.contains("judge_readings"), "{text}");
    record.judge_readings = vec![a_reading()];
    let text = encode(&record).expect("encodes");
    assert!(text.contains(r#""judge_readings":[{"id":"r1""#), "{text}");
}
