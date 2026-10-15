//! An empty optional field leaves no key, because Bridge reads a key as present.
//!
//! A flag with no line was sent as `null`, and drew `file:null`.

use crate::{decode, encode, CitedAt, DismissWaiting, Instant, ManifestEdited, SessionId};

#[test]
fn a_citation_about_the_whole_file_carries_no_line_key() {
    let at = CitedAt {
        file: "src/lib.rs".to_string(),
        line: None,
    };
    let written = encode(&at).expect("a citation encodes");
    assert_eq!(written, r#"{"file":"src/lib.rs"}"#);
    let read: CitedAt = decode("a citation", written.as_bytes()).expect("it reads");
    assert_eq!(read, at);
}

/// A removal with no reason leaves no `reason` key, and a Bridge that sends none reads as none.
#[test]
fn a_removal_without_a_reason_carries_no_reason_key_and_reads_without_one() {
    let bare = DismissWaiting { session_id: SessionId::carried("s1"), item_id: "a".into(), reason: None };
    let written = encode(&bare).expect("a removal encodes");
    assert_eq!(written, r#"{"session_id":"s1","item_id":"a"}"#);
    let read: DismissWaiting = decode("a removal", written.as_bytes()).expect("it reads");
    assert_eq!(read, bare);
    let why = DismissWaiting { reason: Some("done already".into()), ..bare };
    let written = encode(&why).expect("a removal encodes");
    assert_eq!(written, r#"{"session_id":"s1","item_id":"a","reason":"done already"}"#);
}

/// An edit's answer from a Fleet that sends no `declared` still reads, and one
/// with nothing to declare sends no key.
#[test]
fn an_edit_answer_with_nothing_declared_carries_no_declared_key() {
    let edited = ManifestEdited {
        path: "armada.yml".to_string(),
        at: Instant::carried("2026-09-12T09:00:00.000Z"),
        text: "version: 1\n".to_string(),
        declared: None,
    };
    let written = encode(&edited).expect("an edit's answer encodes");
    assert!(
        !written.contains("declared") && !written.contains("null"),
        "{written}"
    );
    let read: ManifestEdited = decode("an edit's answer", written.as_bytes()).expect("it reads");
    assert_eq!(read, edited);
}

/// **What a gate resolved, on the run that reached it** (21.9, #1683; `decided`
/// since 21.10). Built the way Fleet builds it, from `core_model`'s value, so
/// the words are the ones `armada.yml` writes, and compared byte for byte. A
/// run an earlier gate stopped says `decided: false`. A 21.9 Fleet sends no
/// `decided`, which reads `true`, and a Fleet older than 21.9 sends no
/// `resolved`, which reads absent, never as either policy's default.
#[test]
fn a_run_s_resolved_policies_round_trip_and_an_older_run_reads_absent() {
    let run = |outcome, decided| crate::StepAttempt {
        attempt: 1,
        outcome: crate::StepState::from(outcome),
        why: None,
        started_at: Instant::carried("2026-10-01T10:00:00.000Z"),
        ended_at: Some(Instant::carried("2026-10-01T10:05:00.000Z")),
        resolved: Some(
            core_model::ResolvedPolicies {
                auto_merge: core_model::AutoMerge::ChecksPass,
                review_gate: core_model::ReviewGate::HumanAlways,
                decided,
            }
            .into(),
        ),
    };
    for (outcome, decided, bytes) in [
        (
            core_model::StepState::AwaitingHuman,
            true,
            r#"{"attempt":1,"outcome":"awaiting_human","started_at":"2026-10-01T10:00:00.000Z","ended_at":"2026-10-01T10:05:00.000Z","resolved":{"auto_merge":"checks-pass","review_gate":"human_always","decided":true}}"#,
        ),
        (
            core_model::StepState::Stopped,
            false,
            r#"{"attempt":1,"outcome":"stopped","started_at":"2026-10-01T10:00:00.000Z","ended_at":"2026-10-01T10:05:00.000Z","resolved":{"auto_merge":"checks-pass","review_gate":"human_always","decided":false}}"#,
        ),
    ] {
        let run = run(outcome, decided);
        let written = encode(&run).expect("a run encodes");
        assert_eq!(written, bytes);
        let read: crate::StepAttempt = decode("a run", written.as_bytes()).expect("it reads");
        assert_eq!(read, run);
    }

    let from_21_9 = r#"{"attempt":1,"outcome":"awaiting_human","started_at":"2026-10-01T10:00:00.000Z","ended_at":"2026-10-01T10:05:00.000Z","resolved":{"auto_merge":"checks-pass","review_gate":"human_always"}}"#;
    let read: crate::StepAttempt = decode("a run", from_21_9.as_bytes()).expect("it reads");
    assert_eq!(
        read.resolved.map(|was| was.decided),
        Some(true),
        "21.9 kept only runs the rule decided"
    );

    let older = r#"{"attempt":1,"outcome":"advanced","started_at":"2026-10-01T10:00:00.000Z","ended_at":"2026-10-01T10:05:00.000Z"}"#;
    let read: crate::StepAttempt = decode("a run", older.as_bytes()).expect("it reads");
    assert_eq!(read.resolved, None);
    assert_eq!(
        encode(&read).expect("re-encodes"),
        older,
        "absent stays absent on the way back out"
    );
}
