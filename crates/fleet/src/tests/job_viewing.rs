//! When the person was last in a Job: `view_session` keeps it, and the record carries it as
//! `viewed_at`. A stand-in process; no forge, tracker or terminal is reached.

use std::sync::{Arc, Mutex};

use api::{HostedSessions, Refusal};
use ipc::{Event, SessionId, SessionRecord, ViewSession};

use super::session_host::{rig_placing, Rig};

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
