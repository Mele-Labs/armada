//! A hosted job's MCP servers, read and acted on through control lines on its process's input.
//! **A stand-in process, as `session_host` has**: it records the lines Fleet wrote, and is told
//! the CLI's own answer lines, read as the real reader reads them.

use std::sync::Arc;

use adapters::McpState;

use super::session_host::{eventually, rig_placing, Rig, Standing};
use crate::session_host::mcp_servers::{McpAct, SESSION_MCP_REFUSED, SESSION_MCP_UNREACHABLE};
use crate::session_host::Heard;

/// What CLI 2.1.301 answered, with `{id}` for the request id and each `config` cut.
const FAILED: &str = r#"{"type":"control_response","response":{"subtype":"success","request_id":"{id}","response":{"mcpServers":[{"name":"broken","status":"failed","error":"ENOENT: no such file or directory, posix_spawn 'stdio'","config":{"type":"stdio"},"scope":"dynamic","source":"dynamic"}]}}}"#;
const CONNECTED: &str = r#"{"type":"control_response","response":{"subtype":"success","request_id":"{id}","response":{"mcpServers":[{"name":"broken","status":"connected","serverInfo":{"name":"broken","version":"0.1"},"config":{"type":"stdio"},"scope":"dynamic","source":"dynamic","tools":[{"name":"ping","annotations":{}}]}]}}}"#;
const SETTLED: &str = r#"{"type":"control_response","response":{"subtype":"success","request_id":"{id}"}}"#;
const NOBODY: &str = r#"{"type":"control_response","response":{"subtype":"error","request_id":"{id}","error":"Server not found: nobody"}}"#;

fn controls(process: &Standing) -> Vec<String> {
    let sent = process.sent.lock().unwrap();
    sent.iter().filter(|line| line.starts_with(r#"{"type":"control_request""#)).cloned().collect()
}

fn request_id(line: &str) -> String {
    let (_, after) = line.split_once(r#""request_id":""#).expect("a request id");
    after.split('"').next().unwrap_or_default().to_string()
}

/// Wait for the `nth` control line, answer it with `said`, and give back the line with its id
/// written as `{id}`.
async fn answered(rig: &Rig, nth: usize, said: &str) -> String {
    let process = rig.stand_in.process(0);
    eventually(|| async { controls(&process).len() >= nth }).await;
    let line = controls(&process)[nth - 1].clone();
    let id = request_id(&line);
    let answer = adapters::control_answer(&said.replace("{id}", &id)).expect("a control answer");
    rig.stand_in.hears(0, Heard::Answered(answer));
    line.replace(&id, "{id}")
}

fn code(refusal: api::Refusal) -> String {
    match refusal {
        api::Refusal::IllegalMove(why) | api::Refusal::Unacceptable(why) => why.code,
        other => panic!("not a refusal: {other:?}"),
    }
}

#[tokio::test]
async fn a_failed_server_reads_as_failed_and_a_reconnect_sends_mcp_reconnect_and_reads_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    rig.stand_in.init(0);
    let session = id.as_str().to_string();

    let reading = tokio::spawn({
        let (fleet, session) = (Arc::clone(&rig.fleet), session.clone());
        async move { fleet.session_mcp(&session).await }
    });
    let asked = answered(&rig, 1, FAILED).await;
    assert_eq!(asked, r#"{"type":"control_request","request_id":"{id}","request":{"subtype":"mcp_status"}}"#);
    let servers = reading.await.unwrap().expect("read");
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "broken");
    assert_eq!(servers[0].state, McpState::Failed);
    assert_eq!(servers[0].error.as_deref(), Some("ENOENT: no such file or directory, posix_spawn 'stdio'"));

    let acting = tokio::spawn({
        let (fleet, session) = (Arc::clone(&rig.fleet), session.clone());
        async move { fleet.session_mcp_acted(&session, "broken", McpAct::Reconnect).await }
    });
    let reconnect = answered(&rig, 2, SETTLED).await;
    let status = answered(&rig, 3, CONNECTED).await;
    assert_eq!(
        reconnect,
        r#"{"type":"control_request","request_id":"{id}","request":{"subtype":"mcp_reconnect","serverName":"broken"}}"#
    );
    assert_eq!(status, r#"{"type":"control_request","request_id":"{id}","request":{"subtype":"mcp_status"}}"#);
    let acted = acting.await.unwrap().expect("acted");
    assert_eq!(acted.servers[0].state, McpState::Connected);
    assert_eq!(acted.servers[0].tools, 1);
    assert_eq!(acted.sign_in, None);

    let runtime = rig.fleet.hosts().of(&session);
    let state = runtime.state();
    assert!(state.controls.is_empty(), "every waiter was answered");
    assert!(
        !state.unstarted.iter().any(|line| line.contains("control_request")),
        "a control line is not a turn's, so it is not kept to send again"
    );
}

#[tokio::test]
async fn a_job_with_no_process_or_a_refusing_cli_is_refused_plainly() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let refused = rig.fleet.session_mcp(id.as_str()).await.expect_err("no process yet");
    assert_eq!(code(refused), SESSION_MCP_UNREACHABLE);

    rig.send(&id, "hello").await;
    rig.stand_in.init(0);
    let acting = tokio::spawn({
        let (fleet, session) = (Arc::clone(&rig.fleet), id.as_str().to_string());
        async move { fleet.session_mcp_acted(&session, "nobody", McpAct::TurnOn).await }
    });
    let asked = answered(&rig, 1, NOBODY).await;
    assert!(asked.contains(r#""subtype":"mcp_toggle","serverName":"nobody","enabled":true"#), "{asked}");
    let refused = acting.await.unwrap().expect_err("the CLI refused");
    let api::Refusal::Unacceptable(why) = refused else {
        panic!("not a 422: {refused:?}");
    };
    assert_eq!(why.code, SESSION_MCP_REFUSED);
    assert!(why.message.contains("Server not found: nobody"), "{}", why.message);
}
