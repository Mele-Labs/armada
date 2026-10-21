//! The control lines a hosted job's servers are read and acted on through.
//!
//! The answers below are what CLI 2.1.301 sent on 21 Oct 2026, run as a hosted
//! job is, with each `config` cut and a sign-in's address shortened.

use crate::mcp_control::{control_answer, mcp_request, Answered, ControlAnswer, McpRequest, McpServer, McpSignIn, McpState};

const STATUS: &str = concat!(
    r#"{"type":"control_response","response":{"subtype":"success","request_id":"s1","response":{"mcpServers":["#,
    r#"{"name":"spike","status":"connected","serverInfo":{"name":"spike","version":"0.1"},"config":{"type":"stdio"},"scope":"dynamic","source":"dynamic","tools":[{"name":"ping","annotations":{}}]},"#,
    r#"{"name":"broken","status":"failed","error":"ENOENT: no such file or directory, posix_spawn 'stdio'","config":{"type":"stdio"},"scope":"dynamic","source":"dynamic"}"#,
    r#"]}}}"#,
);
const TURNED_OFF: &str = r#"{"type":"control_response","response":{"subtype":"success","request_id":"s2","response":{"mcpServers":[{"name":"spike","status":"disabled","config":{"type":"stdio"},"scope":"dynamic","source":"dynamic"}]}}}"#;
const NEEDS_SIGN_IN: &str = r#"{"type":"control_response","response":{"subtype":"success","request_id":"s3","response":{"mcpServers":[{"name":"tracker","status":"needs-auth","config":{"type":"http","url":"https://mcp.example.test/mcp"},"scope":"dynamic","source":"dynamic"}]}}}"#;
const RECONNECTED: &str = r#"{"type":"control_response","response":{"subtype":"success","request_id":"r1"}}"#;
const RECONNECT_REFUSED: &str = r#"{"type":"control_response","response":{"subtype":"error","request_id":"r2","error":"ENOENT: no such file or directory, posix_spawn 'stdio'"}}"#;
const NOBODY: &str = r#"{"type":"control_response","response":{"subtype":"error","request_id":"t3","error":"Server not found: nobody"}}"#;
const SIGN_IN: &str = r#"{"type":"control_response","response":{"subtype":"success","request_id":"a1","response":{"authUrl":"https://mcp.example.test/authorize?response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A64820%2Fcallback","requiresUserAction":true,"callbackExpected":true,"redirectScheme":"localhost","state":"u05u","callbackPort":64820}}}"#;
const SIGN_IN_REFUSED: &str = r#"{"type":"control_response","response":{"subtype":"error","request_id":"a2","error":"Server type \"stdio\" does not support OAuth authentication"}}"#;

fn read(line: &str) -> ControlAnswer {
    control_answer(line).expect("a control answer")
}

fn servers(line: &str) -> Vec<McpServer> {
    match read(line).outcome {
        Ok(Answered::Servers(servers)) => servers,
        other => panic!("not a list of servers: {other:?}"),
    }
}

#[test]
fn each_request_is_the_line_the_cli_takes() {
    assert_eq!(
        mcp_request("s1", McpRequest::Status),
        r#"{"type":"control_request","request_id":"s1","request":{"subtype":"mcp_status"}}"#
    );
    assert_eq!(
        mcp_request("r1", McpRequest::Reconnect("spike")),
        r#"{"type":"control_request","request_id":"r1","request":{"subtype":"mcp_reconnect","serverName":"spike"}}"#
    );
    assert_eq!(
        mcp_request("t1", McpRequest::TurnOff("spike")),
        r#"{"type":"control_request","request_id":"t1","request":{"subtype":"mcp_toggle","serverName":"spike","enabled":false}}"#
    );
    assert_eq!(
        mcp_request("t2", McpRequest::TurnOn("spike")),
        r#"{"type":"control_request","request_id":"t2","request":{"subtype":"mcp_toggle","serverName":"spike","enabled":true}}"#
    );
    assert_eq!(
        mcp_request("a1", McpRequest::SignIn("tracker")),
        r#"{"type":"control_request","request_id":"a1","request":{"subtype":"mcp_authenticate","serverName":"tracker"}}"#
    );
}

#[test]
fn a_status_answer_reads_as_each_server_and_where_it_stands() {
    assert_eq!(read(STATUS).request_id, "s1");
    assert_eq!(
        servers(STATUS),
        [
            McpServer {
                name: "spike".into(),
                state: McpState::Connected,
                error: None,
                scope: Some("dynamic".into()),
                tools: 1,
            },
            McpServer {
                name: "broken".into(),
                state: McpState::Failed,
                error: Some("ENOENT: no such file or directory, posix_spawn 'stdio'".into()),
                scope: Some("dynamic".into()),
                tools: 0,
            },
        ]
    );
    assert_eq!(servers(TURNED_OFF)[0].state, McpState::Off);
    assert_eq!(servers(NEEDS_SIGN_IN)[0].state, McpState::NeedsSignIn);
    let pending = STATUS.replace(r#""status":"connected""#, r#""status":"pending""#);
    assert_eq!(servers(&pending)[0].state, McpState::Connecting);
}

#[test]
fn a_state_word_armada_does_not_know_reads_as_failed_with_the_word_kept() {
    let unknown = STATUS.replace(r#""status":"connected""#, r#""status":"sleeping""#);
    let first = servers(&unknown).remove(0);
    assert_eq!(first.state, McpState::Failed);
    let why = first.error.expect("the word is kept");
    assert!(why.contains("`sleeping`"), "{why}");
}

#[test]
fn a_reconnect_or_toggle_that_settled_reads_as_done_and_a_refusal_keeps_the_clis_words() {
    assert_eq!(read(RECONNECTED), ControlAnswer { request_id: "r1".into(), outcome: Ok(Answered::Done) });
    assert_eq!(
        read(RECONNECT_REFUSED).outcome,
        Err(String::from("ENOENT: no such file or directory, posix_spawn 'stdio'"))
    );
    assert_eq!(read(NOBODY), ControlAnswer { request_id: "t3".into(), outcome: Err("Server not found: nobody".into()) });
}

#[test]
fn a_sign_in_answer_is_the_page_and_whether_the_process_waits_for_it() {
    let Ok(Answered::SignIn(Some(McpSignIn { url, callback_expected }))) = read(SIGN_IN).outcome else {
        panic!("not a sign-in");
    };
    assert!(url.starts_with("https://mcp.example.test/authorize?"), "{url}");
    assert!(callback_expected);
    let signed_in = r#"{"type":"control_response","response":{"subtype":"success","request_id":"a3","response":{"requiresUserAction":false}}}"#;
    assert_eq!(read(signed_in).outcome, Ok(Answered::SignIn(None)));
    assert!(read(SIGN_IN_REFUSED).outcome.is_err_and(|why| why.contains("does not support OAuth")));
}

#[test]
fn a_line_that_is_not_a_control_answer_is_passed_over() {
    for line in [
        r#"{"type":"system","subtype":"init","mcp_servers":[]}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"control_response"}]}}"#,
        "not json at all \"control_response\"",
    ] {
        assert!(control_answer(line).is_none(), "{line}");
    }
}
