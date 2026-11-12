//! When the person was last in a Job: `view_session` keeps it, and the record carries it as
//! `viewed_at`. A stand-in process; no forge, tracker or terminal is reached.

use std::sync::{Arc, Mutex};

use api::{HostedSessions, Refusal};
use ipc::{AnswerWaiting, Event, SendSessionMessage, SessionId, SessionRecord, ViewSession, WaitingSource};

use super::session_host::{eventually, rig_placing, Rig};

fn published(rig: &Rig, id: &SessionId) -> Arc<Mutex<Vec<SessionRecord>>> {
    let published = Arc::new(Mutex::new(Vec::new()));
    let (seen, id) = (Arc::clone(&published), id.clone());
    rig.fleet.events().tap(move |event| {
        if let Event::SessionChanged(record) = event {
            if record.id == id {
                seen.lock().unwrap().push(record.clone());
            }
        }
    });
    published
}

/// Each visit is published with the instant on the record, and the store keeps it past a restart.
#[tokio::test]
async fn a_visit_is_kept_and_published_on_the_record() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert_eq!(Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.viewed_at, None, "never in it");
    let seen = published(&rig, &id);

    let came = rig.fleet.view_session(ViewSession { session_id: id.clone(), on: true }).await.expect("on screen");
    let left = rig.fleet.view_session(ViewSession { session_id: id.clone(), on: false }).await.expect("off screen");

    let (came, left) = (came.viewed_at.expect("kept on arriving"), left.viewed_at.expect("kept on leaving"));
    assert!(left.as_str() > came.as_str(), "leaving moves it on: {came:?} then {left:?}");
    let seen: Vec<_> = seen.lock().unwrap().iter().map(|one| one.viewed_at.clone()).collect();
    assert_eq!(seen, [Some(came), Some(left.clone())]);
    let rig = rig.restarted(|_| {});
    assert_eq!(Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.viewed_at, Some(left));
}

#[tokio::test]
async fn a_job_fleet_does_not_know_is_refused_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let gone = SessionId::carried("01NOSUCHJOB0000000000000000");
    let refused = rig.fleet.view_session(ViewSession { session_id: gone, on: true }).await;
    assert!(matches!(refused, Err(Refusal::NotFound(_))), "{refused:?}");
}

async fn viewed_at(rig: &Rig, id: &SessionId) -> Option<ipc::Instant> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.viewed_at
}

/// He is in the Job when he writes to it, through whichever client: the message's own route keeps
/// it. A message Fleet sends a Job itself, as Autopilot's or the Operator's do, is not him.
#[tokio::test]
async fn a_message_he_sends_is_a_visit_and_one_fleet_sends_is_not() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "go").await;
    assert_eq!(viewed_at(&rig, &id).await, None, "Fleet's own send is no visit");

    let sent = SendSessionMessage { session_id: id.clone(), text: "and the floor?".into(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() };
    let answered = Arc::clone(&rig.fleet).send_session_message_aiming(sent, None).await.expect("sent");
    assert!(answered.viewed_at.is_some(), "the answer carries the visit");
    assert_eq!(viewed_at(&rig, &id).await, answered.viewed_at);
}

const ASKS: &str = "The cap is five.\n\nWant me to land it?";

/// Answering what a Job waits on, from a card anywhere, is being in the Job.
#[tokio::test]
async fn answering_what_a_job_waits_on_is_a_visit() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, ASKS);
    let replied = |record: &SessionRecord| record.waiting_for.iter().find(|one| one.source == WaitingSource::Replied).map(|one| one.id.clone());
    eventually(|| async { replied(&Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session).is_some() }).await;
    let item_id = replied(&Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session).expect("the item");
    assert_eq!(viewed_at(&rig, &id).await, None);

    let said = AnswerWaiting { session_id: id.clone(), item_id, choice: None, text: Some("yes, land it".into()), mode: None, fixed: None };
    let after = Arc::clone(&rig.fleet).answer_waiting(said).await.expect("answered");
    assert!(after.viewed_at.is_some(), "the answer carries the visit");
}
