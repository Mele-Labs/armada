//! The boxes a person checks before Autopilot starts: what goes on it, and which kinds of decision
//! a helper may answer. The set is closed and has no `destructive`: that always waits for a person,
//! so there is no word to send for it.

use crate::{decode_body, encode, AutopilotDecision, AutopilotSettings, AutopilotWork};

const BEFORE: &[u8] = br#"{"instruction":"fix what is open","at_once":2}"#;

#[test]
fn a_body_from_before_the_boxes_reads_as_backlog_on_running_off_and_no_decisions() {
    let read: AutopilotSettings = decode_body("settings", BEFORE).expect("reads");
    assert_eq!(read.on_autopilot, AutopilotWork { backlog: true, running: false });
    assert!(read.decisions.is_empty());
}

#[test]
fn every_kind_of_decision_round_trips_under_its_own_word() {
    let words = ["recommended", "open", "plan", "draft", "walk", "land", "unsure", "red", "reads", "writes"];
    let body = format!(
        r#"{{"instruction":"x","at_once":1,"on_autopilot":{{"backlog":false,"running":true}},"decisions":{}}}"#,
        serde_json::to_string(&words).expect("words")
    );
    let read: AutopilotSettings = decode_body("settings", body.as_bytes()).expect("reads");
    assert_eq!(read.decisions.len(), words.len());
    assert_eq!(read.on_autopilot, AutopilotWork { backlog: false, running: true });
    let sent = encode(&read).expect("writes");
    for word in words {
        assert!(sent.contains(&format!("\"{word}\"")), "{word}");
    }
    assert!(read.decisions.contains(&AutopilotDecision::Land));
}

#[test]
fn a_body_naming_a_destructive_decision_is_refused() {
    for word in ["destructive", "Destructive", "everything", ""] {
        let body = format!(r#"{{"instruction":"x","at_once":1,"decisions":["{word}"]}}"#);
        assert!(decode_body::<AutopilotSettings>("settings", body.as_bytes()).is_err(), "{word:?}");
    }
}

fn an_answer() -> crate::AutopilotDecided {
    crate::AutopilotDecided {
        id: "S1:a".into(),
        manifest_id: crate::ManifestId::carried("M1"),
        session_id: None,
        who: "Fix the clock".into(),
        kind: AutopilotDecision::Recommended,
        asked: "Which clock?".into(),
        chose: "Frozen (Recommended)".into(),
        because: "it is the one the agent marked".into(),
        at: crate::Instant::carried("2026-11-02T01:00:00.000Z"),
    }
}

/// The event carries the whole list beside its own `kind`, and a row with no Job leaves
/// `session_id` out rather than sending `null`.
#[test]
fn the_decisions_event_carries_the_list_whole_and_leaves_an_empty_job_out() {
    let list = crate::AutopilotDecisions { to_review: vec![an_answer()] };
    let spelled = encode(&crate::StreamMessage::Event(crate::Delivered {
        cursor: crate::Cursor::at(4),
        event: crate::Event::AutopilotDecisionsChanged(list.clone()),
    }))
    .expect("plain data");
    assert!(spelled.contains(r#""kind":"autopilot.decisions_changed""#), "{spelled}");
    assert!(spelled.contains(r#""kind":"recommended""#) && !spelled.contains("null"), "{spelled}");
    let back: crate::StreamMessage = crate::decode("a stream message", spelled.as_bytes()).expect("reads");
    assert!(matches!(
        back,
        crate::StreamMessage::Event(crate::Delivered { event: crate::Event::AutopilotDecisionsChanged(held), .. }) if held == list
    ));
}
