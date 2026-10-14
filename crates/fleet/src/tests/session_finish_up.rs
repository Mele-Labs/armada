//! A hosted session giving back what it is finished with in one call: the slots the pool holds for
//! it, and nothing of anyone else's. `docs/concepts/session.md`, *Finishing up*.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{FinishUp, SessionId};

use super::session_host::rig_placing;

const OTHERS: &str = "0b0b0b0b-0000-0000-0000-000000000000";

fn finish(id: &SessionId) -> FinishUp {
    FinishUp { session_id: Some(id.clone()) }
}

#[tokio::test]
async fn every_slot_held_for_the_session_is_released_and_nobody_elses() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.vcs().hold_slot(&rig.root, 2, id.as_str());
    rig.fleet.vcs().hold_slot(&rig.root, 3, OTHERS);
    rig.fleet.vcs().hold_slot(&rig.root, 4, id.as_str());

    let done = rig.fleet.finish_up(None, finish(&id)).await.unwrap();

    assert_eq!(done.released, vec![2, 4]);
    assert!(done.kept.is_empty(), "{:?}", done.kept);
    let held = rig.fleet.vcs().slot_holders(&rig.root);
    assert_eq!(held[1], None);
    assert_eq!(held[2].as_deref(), Some(OTHERS));
    assert_eq!(held[3], None);
}

#[tokio::test]
async fn a_slot_the_pool_refuses_stays_held_and_is_named() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.vcs().hold_slot(&rig.root, 2, id.as_str());
    rig.fleet.vcs().keep_next_release("2 files are uncommitted");

    let done = rig.fleet.finish_up(None, finish(&id)).await.unwrap();

    assert!(done.released.is_empty());
    assert_eq!(done.kept, vec!["slot 2: 2 files are uncommitted".to_string()]);
    assert_eq!(rig.fleet.vcs().slot_holders(&rig.root)[1].as_deref(), Some(id.as_str()));
}

#[tokio::test]
async fn a_session_with_nothing_held_has_nothing_to_give_back() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let done = rig.fleet.finish_up(None, finish(&id)).await.unwrap();
    assert_eq!(done, ipc::FinishedUp::default());
}

#[tokio::test]
async fn an_ended_session_and_a_call_that_places_none_are_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert!(rig.fleet.finish_up(None, FinishUp { session_id: None }).await.is_err());
    Arc::clone(&rig.fleet)
        .close_session(ipc::CloseSession { session_id: id.clone() })
        .await
        .unwrap();
    assert!(rig.fleet.finish_up(None, finish(&id)).await.is_err());
}
