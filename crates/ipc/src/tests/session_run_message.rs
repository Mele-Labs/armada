//! How a run ended, as the socket says it: a code, or a null code and whether Stop was the cause.

use crate::SessionRunMessage;

#[test]
fn exited_says_whether_stop_ended_a_run_that_has_no_code() {
    let stopped = SessionRunMessage::Exited { code: None, stopped: true };
    assert_eq!(serde_json::to_string(&stopped).unwrap(), r#"{"kind":"exited","code":null,"stopped":true}"#);
    let signalled: SessionRunMessage = serde_json::from_str(r#"{"kind":"exited","code":null,"stopped":false}"#).unwrap();
    assert_eq!(signalled, SessionRunMessage::Exited { code: None, stopped: false });
    assert_eq!(
        serde_json::to_string(&SessionRunMessage::Exited { code: Some(1), stopped: false }).unwrap(),
        r#"{"kind":"exited","code":1,"stopped":false}"#
    );
}
