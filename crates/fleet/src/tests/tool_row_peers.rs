//! The claim: **a tool call's row carries the picture it looked at and the message it sent**, and a
//! message names the Job it went to where Fleet can place it. A hosted Job's rows are written by
//! `row_put`, which is where the name is resolved, so a stored row and its event agree.
//!
//! Driven by stream lines sent as the reader sends them, and read off the thread.

use adapter_traits::AgentHarness;
use adapters::HeadlessAgent;
use ipc::{MessagePeer, SessionId, SessionRow, SessionTurn, ToolMessage};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_plan::running;
use crate::session_host::{address_of, Heard};

fn call(id: &str, name: &str, input: &str) -> String {
    format!(r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"{id}","name":"{name}","input":{input}}}]}}}}"#)
}

async fn tool_rows(rig: &Rig, id: &SessionId) -> Vec<SessionRow> {
    rig.rows(id).await.into_iter().filter(|row| matches!(row, SessionRow::Tool { .. })).collect()
}

/// Two Jobs with a process each: `Sender`, whose agent has pid 1000, and `Reader`, pid 1001.
async fn two_jobs(rig: &Rig) -> (SessionId, SessionId) {
    let sender = rig.start_titled(Some("Sender")).await;
    let reader = rig.start_titled(Some("Reader")).await;
    rig.send(&sender, "go").await;
    rig.send(&reader, "go").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "ready");
    rig.stand_in.init(1);
    rig.stand_in.finishes(1, "ready");
    eventually(|| async { rig.turn(&reader).await == SessionTurn::Idle }).await;
    (sender, reader)
}

/// What the sender sent to `to`, once its row is on the thread.
async fn sent_to(rig: &Rig, sender: &SessionId, to: &str, input: &str) -> ToolMessage {
    let line = call("toolu_msg", "SendMessage", &format!(r#"{{"to":"{to}",{input}}}"#));
    rig.stand_in.hears(0, Heard::Events(HeadlessAgent::at("agent").read(&line)));
    let find = || async {
        tool_rows(rig, sender).await.into_iter().find_map(|row| match row {
            SessionRow::Tool { message: Some(message), .. } if message.to.name == to => Some(message),
            _ => None,
        })
    };
    eventually(|| async { find().await.is_some() }).await;
    find().await.expect("the row is on the thread")
}

#[tokio::test]
async fn an_image_read_carries_its_path() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    for (call_id, path) in [("toolu_70", "/tmp/shot.png"), ("toolu_71", "/tmp/main.rs")] {
        let line = call(call_id, "Read", &format!(r#"{{"file_path":"{path}"}}"#));
        rig.stand_in.hears(0, Heard::Events(HeadlessAgent::at("agent").read(&line)));
    }
    eventually(|| async { tool_rows(&rig, &id).await.len() == 2 }).await;

    let images: Vec<Option<String>> = tool_rows(&rig, &id)
        .await
        .into_iter()
        .map(|row| match row {
            SessionRow::Tool { image, .. } => image,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(images, [Some("/tmp/shot.png".to_string()), None]);
}

#[tokio::test]
async fn a_sent_message_names_the_job() {
    let rig = rig_placing(None);
    let (sender, reader) = two_jobs(&rig).await;

    let known = sent_to(&rig, &sender, &address_of(reader.as_str()), r#""message":"please review""#).await;
    assert_eq!(
        known.to,
        MessagePeer { name: address_of(reader.as_str()), session_id: Some(reader.clone()), title: Some("Reader".into()) }
    );
    assert_eq!(known.text, "please review");

    let unknown = sent_to(&rig, &sender, "s-00000000", r#""message":"anyone""#).await;
    assert_eq!(unknown.to, MessagePeer { name: "s-00000000".into(), session_id: None, title: None });
    let bare = sent_to(&rig, &sender, "a2f35e976899d156f", r#""message":"anyone""#).await;
    assert_eq!(bare.to.session_id, None, "a bare name is no Job's address");
}

/// **The socket's pid is the hosted agent's**: Fleet keeps a pid for the agent it hosts, so a
/// message to that socket names the Job, and one to a pid no Job's agent has stays as written.
#[tokio::test]
async fn a_sent_message_resolves_a_socket_by_pid() {
    let rig = rig_placing(None);
    let (sender, reader) = two_jobs(&rig).await;

    let by_pid = sent_to(&rig, &sender, "uds:/tmp/cc-socks/1001.sock", r#""message":"over here""#).await;
    assert_eq!(by_pid.to.session_id, Some(reader));
    assert_eq!(by_pid.to.title.as_deref(), Some("Reader"));
    assert_eq!(by_pid.to.name, "uds:/tmp/cc-socks/1001.sock");

    let gone = sent_to(&rig, &sender, "uds:/tmp/cc-socks/4242.sock", r#""message":"hello""#).await;
    assert_eq!(gone.to.session_id, None);
}

#[tokio::test]
async fn a_sent_message_carries_its_summary() {
    let rig = rig_placing(None);
    let (sender, reader) = two_jobs(&rig).await;

    let said = sent_to(&rig, &sender, &address_of(reader.as_str()), r#""summary":"  Heads up  ","message":"the build is red""#).await;
    assert_eq!(said.summary.as_deref(), Some("Heads up"));
    assert_eq!(said.text, "the build is red");

    let bare = sent_to(&rig, &sender, "s-00000000", r#""message":"no summary""#).await;
    assert_eq!(bare.summary, None);
}
