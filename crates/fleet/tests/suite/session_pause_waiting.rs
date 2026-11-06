//! A paused Job raises nothing on the person's Needs you list, and what it owed is back after Resume.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{PauseSession, ResumeSession, SessionId, SessionRecord, SessionStanding, SetWaitingFor, WaitingInput};

use super::session_host::{denied, rig_placing, Rig};

const CALL: &str = "slot-3 is 2 commits behind main.";

async fn row(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session
}

/// A Job holding slot one that owes the person an item and a call.
async fn owing(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    rig.stand_in.init(0);
    let path = format!("{}/src/lib.rs", rig.root);
    assert!(denied(&rig.gate(&id, "Edit", &format!(r#"{{"file_path":"{path}"}}"#)).await));
    let items = vec![WaitingInput { id: "pick".into(), text: "Pick a name".into(), act: None, options: Vec::new(), steps: Vec::new() }];
    rig.fleet.waiting_for(None, SetWaitingFor { session_id: Some(id.clone()), items }).await.expect("kept");
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet.store().lock().await.keep_call(id.as_str(), ipc::Notice::BehindBase.word(), CALL, &at).expect("a call");
    id
}

fn owed(record: &SessionRecord) -> Vec<String> {
    record.waiting_for.iter().map(|one| one.text.clone()).collect()
}

#[tokio::test]
async fn a_paused_job_raises_no_item_or_call_and_they_return_after_the_resume() {
    let rig = rig_placing(None);
    let id = owing(&rig).await;
    let before = owed(&row(&rig, &id).await);
    assert!(before.iter().any(|one| one == "Pick a name") && before.iter().any(|one| one.contains(CALL)), "{before:?}");

    rig.fleet.pause_session(PauseSession { session_id: id.clone() }).await.expect("paused");
    let paused = row(&rig, &id).await;
    assert!(paused.waiting_for.is_empty(), "{:?}", owed(&paused));
    assert_eq!(paused.standing, Some(SessionStanding::Idle));

    Arc::clone(&rig.fleet).resume_session(ResumeSession { session_id: id.clone() }).await.expect("resumed");
    assert_eq!(owed(&row(&rig, &id).await), before, "still owed, so back");
}
