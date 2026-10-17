//! A person's act from the phone keeps the phone with it, where the act leaves
//! no move to hang a door on: a Job dispatched, a Session ask answered, a
//! Drone redirected mid-step. A Bridge act keeps nothing.

use api::Queries;
use testkit::{FakeJudge, FakeWorkProduct};

use crate::tests::daemon::a_fleet_proposing_through;
use crate::tests::planning::A_PLAN;
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

async fn dispatched_through(door: &str) -> Option<ipc::Via> {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_PLAN),
    );
    let made = api::carrying(ipc::Via::from_wire(door), async {
        fleet.propose_from("two coupled changes", None).await
    })
    .await
    .expect("a plan");
    fleet
        .get_job(ipc::JobId::from(made[0].id()))
        .await
        .expect("the Job reads")
        .created_via
}

#[tokio::test]
async fn a_job_dispatched_from_the_phone_names_the_phone() {
    assert_eq!(dispatched_through("phone").await, ipc::Via::from_wire("phone"));
}

#[tokio::test]
async fn a_job_dispatched_in_bridge_names_no_door() {
    assert_eq!(dispatched_through("bridge").await, None);
}

/// The `via` of the one ask row on a hosted session's thread, after the person
/// answered it through `door`.
async fn ask_answered_through(door: &str) -> Option<ipc::Via> {
    use crate::tests::session_host::{eventually, rig_placing};
    use api::HostedSessions;

    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet
        .tune_session(ipc::TuneSession {
            session_id: id.clone(),
            model: None,
            effort: None,
            mode: ipc::SessionMode::Ask,
        })
        .await
        .unwrap();
    let fleet = std::sync::Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let waiting = tokio::spawn(async move {
        let body = r#"{"tool_name":"Bash","input":{"command":"rm -rf build"}}"#;
        let asking: ipc::AskingToRun = ipc::decode("an ask", body.as_bytes()).expect("an ask");
        fleet.session_permission(&session, asking).await
    });
    let call = std::sync::Mutex::new(String::new());
    eventually(|| async {
        let asked = std::sync::Arc::clone(&rig.fleet)
            .get_session(id.clone())
            .await
            .unwrap()
            .session
            .hosted
            .unwrap()
            .asked;
        match asked {
            Some(asked) => {
                *call.lock().unwrap() = asked.call;
                true
            }
            None => false,
        }
    })
    .await;
    let call = call.into_inner().unwrap();
    api::carrying(ipc::Via::from_wire(door), async {
        rig.fleet
            .answer_session_ask(ipc::AnswerSessionAsk {
                session_id: id.clone(),
                call,
                answer: ipc::HelmCallAnswer::AllowOnce,
                note: None,
                answers: Vec::new(),
                folder: None,
            })
            .await
            .unwrap()
    })
    .await;
    waiting.await.unwrap().unwrap();
    match &rig.rows(&id).await[..] {
        [ipc::SessionRow::Ask { via, .. }] => *via,
        other => panic!("one ask row was expected: {other:?}"),
    }
}

#[tokio::test]
async fn a_session_ask_answered_from_the_phone_names_the_phone() {
    assert_eq!(ask_answered_through("phone").await, ipc::Via::from_wire("phone"));
}

#[tokio::test]
async fn a_session_ask_answered_in_bridge_is_not_marked_as_the_phone() {
    assert_ne!(ask_answered_through("bridge").await, ipc::Via::from_wire("phone"));
}
