//! The model a hosted session's agent resolved, read off its `init` line. Since 23.85. **A stand-in
//! process, as `session_host` has**: what is read here is what Fleet carries on the row.

use std::sync::Arc;

use adapter_traits::DroneEvent;
use api::HostedSessions;
use ipc::{SessionId, StopSession};

use super::session_host::{eventually, rig_placing, whole, Rig};

async fn resolved(rig: &Rig, id: &SessionId) -> Option<String> {
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    record.hosted.expect("hosted").resolved_model
}

fn init(model: &str) -> Vec<DroneEvent> {
    vec![DroneEvent::Started {
        session: String::new(),
        model: model.into(),
        mcp_servers: 1,
        harness: whole(),
    }]
}

#[tokio::test]
async fn the_model_the_init_line_names_is_on_the_row_and_none_is_guessed_before_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert_eq!(resolved(&rig, &id).await, None, "no process has started, so none is said");

    rig.send(&id, "hello").await;
    rig.stand_in.says(0, init("vendor-opus-5-5"));
    eventually(|| async { resolved(&rig, &id).await.is_some() }).await;
    assert_eq!(resolved(&rig, &id).await.as_deref(), Some("vendor-opus-5-5"));

    // The next turn's init is the one that stands.
    rig.stand_in.says(0, init("vendor-sonnet-5-5"));
    eventually(|| async { resolved(&rig, &id).await.as_deref() == Some("vendor-sonnet-5-5") }).await;
}

#[tokio::test]
async fn an_init_that_names_no_model_leaves_none() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    rig.stand_in.says(0, init("vendor-opus-5-5"));
    eventually(|| async { resolved(&rig, &id).await.is_some() }).await;

    rig.stand_in.says(0, init(""));
    eventually(|| async { resolved(&rig, &id).await.is_none() }).await;
}

#[tokio::test]
async fn a_process_that_is_let_go_takes_its_model_with_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    rig.stand_in.says(0, init("vendor-opus-5-5"));
    eventually(|| async { resolved(&rig, &id).await.is_some() }).await;

    rig.fleet
        .stop_session(StopSession {
            session_id: id.clone(),
        })
        .await
        .expect("stopped");
    assert_eq!(resolved(&rig, &id).await, None, "the next process has not said yet");
}
