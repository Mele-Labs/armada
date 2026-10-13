//! Stopping the turn a hosted session is running. Since 23.81. **A stand-in process, as
//! `session_host` has**: the stop is Fleet ending it and setting the turn, which is what is
//! read here.

use std::sync::atomic::Ordering;

use api::{HostedSessions, Sessions};
use ipc::{
    AttachmentState, SessionFact, SessionId, SessionOrigin, SessionReport, SessionState,
    SessionTurn, StopSession,
};

use super::session_host::{denied, eventually, rig_placing};

#[tokio::test]
async fn a_stop_ends_the_turn_and_keeps_the_row_and_the_slot_and_the_next_message_resumes() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    rig.stand_in.init(0);
    // A first write leases a slot, so there is something for a stop to leave held.
    let path = format!("{}/src/lib.rs", rig.root);
    let held = rig
        .gate(&id, "Edit", &format!(r#"{{"file_path":"{path}"}}"#))
        .await;
    assert!(denied(&held));
    assert!(matches!(rig.turn(&id).await, SessionTurn::Working { .. }));

    let stopped = rig
        .fleet
        .stop_session(StopSession {
            session_id: id.clone(),
        })
        .await
        .expect("stopped");

    assert_eq!(stopped.hosted.expect("hosted").turn, SessionTurn::Idle);
    assert_eq!(stopped.state, SessionState::Live);
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst));
    assert!(
        stopped
            .attachments
            .iter()
            .any(|one| one.kind == "slot" && one.target == "1" && one.state == AttachmentState::Standing),
        "the slot is still held"
    );
    assert_eq!(rig.turn(&id).await, SessionTurn::Idle);

    rig.send(&id, "carry on").await;
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    let resumed = &rig.stand_in.starts()[1];
    assert_eq!(resumed.session, id.as_str());
    assert!(resumed.resuming, "the same conversation, resumed");
    assert!(rig.stand_in.process(1).sent.lock().unwrap()[0].contains("carry on"));
}

#[tokio::test]
async fn a_terminal_session_is_not_stopped_from_here() {
    let rig = rig_placing(None);
    const ID: &str = "bbbb-2222";
    rig.fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried(ID),
            fact: SessionFact::Started {
                cwd: rig.root.clone(),
                title: None,
                origin: SessionOrigin::Terminal,
                mod_version: None,
            },
        })
        .await
        .unwrap();

    let refused = rig
        .fleet
        .stop_session(StopSession {
            session_id: SessionId::carried(ID),
        })
        .await
        .expect_err("a terminal session is refused");
    assert!(format!("{refused:?}").contains("fleet.session_command_refused"), "{refused:?}");
}
