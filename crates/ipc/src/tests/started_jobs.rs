//! What the two acts carry on the wire: a child's line is its summary with the extra fields beside it,
//! not under a key, and an act is a word. `docs/concepts/session/a-job-that-started-jobs.md`.

use crate::{decode, encode, SessionState, StartedAct, StartedCommands, StartedJob, StartedResult};

#[test]
fn a_childs_line_flattens_its_summary_and_reads_back_the_same() {
    let mut record = super::session_wire::a_record();
    record.state = SessionState::Ended;
    record.end_reason = Some("closed".into());
    let line = StartedJob::of(&record, None, Some("Moved the reads.".into()));

    let json = encode(&line).expect("plain data");
    assert!(json.contains(r#""id":"01RECORD""#) && json.contains(r#""held":false"#), "{json}");
    assert!(!json.contains(r#""summary":{"id""#), "the summary is not nested: {json}");
    assert_eq!(decode::<StartedJob>("a child", json.as_bytes()).expect("round-trips"), line);
    assert_eq!(line.result, Some(StartedResult { end_reason: Some("closed".into()), summary: Some("Moved the reads.".into()) }));
}

#[test]
fn a_live_child_has_no_result_and_an_act_is_its_snake_case_word() {
    let live = StartedJob::of(&super::session_wire::a_record(), None, Some("not yet".into()));
    assert!(live.result.is_none(), "only an ended Job has one");

    let body = r#"{"commands":[{"session_id":"01KID","act":"withdraw"},{"session_id":"01KID","act":"drop"}]}"#;
    let commands = decode::<StartedCommands>("commands", body.as_bytes()).expect("decodes");
    let acts: Vec<&str> = commands.commands.iter().map(|one| one.act.word()).collect();
    assert_eq!(acts, ["withdraw", "drop"]);
    assert!(decode::<StartedCommands>("commands", br#"{"commands":[{"session_id":"x","act":"Withdraw"}]}"#).is_err());
    assert_eq!(serde_json::to_string(&StartedAct::Resume).unwrap(), r#""resume""#);
}
