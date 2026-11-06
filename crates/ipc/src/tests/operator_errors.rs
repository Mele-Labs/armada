//! An error Bridge showed the person, on the wire.

use crate::operator_errors::{BridgeErrorField, BridgeErrorTaken, ReportBridgeError};

#[test]
fn an_error_card_round_trips_with_its_fields_chain_logs_commit_and_job() {
    let sent = ReportBridgeError {
        code: "bridge.fleet.build_failed".into(),
        message: "Build failed: restart exited 1".into(),
        fields: vec![BridgeErrorField { key: "exit".into(), value: "1".into() }],
        chain: vec!["restart exited 1".into(), "no such file".into()],
        logs: vec!["/tmp/restart.log".into()],
        commit: Some("e1e0644c02".into()),
        job_id: Some("01JOB".into()),
        at: "2026-11-06T09:00:00.000Z".into(),
    };
    let wire = serde_json::to_value(&sent).expect("written");
    assert_eq!(wire["fields"], serde_json::json!([{ "key": "exit", "value": "1" }]));
    assert_eq!(serde_json::from_value::<ReportBridgeError>(wire).expect("read"), sent);
}

#[test]
fn a_card_with_only_a_code_a_message_and_a_time_reads_with_the_rest_absent() {
    let read: ReportBridgeError = serde_json::from_str(r#"{"code":"bridge.not_sent","message":"No handler registered","at":"2026-11-06T09:00:00.000Z"}"#).expect("read");
    assert!(read.fields.is_empty() && read.chain.is_empty() && read.logs.is_empty());
    assert_eq!((read.commit, read.job_id), (None, None));
    let wire = serde_json::to_value(BridgeErrorTaken { id: "error:abc:2026-11-06".into(), repeated: true }).expect("written");
    assert_eq!(wire, serde_json::json!({ "id": "error:abc:2026-11-06", "repeated": true }));
}
