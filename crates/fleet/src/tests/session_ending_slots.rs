//! A hosted session that ends leaves nothing held: its slot is parked, and where the pool refuses,
//! the ledger keeps saying the slot is held until a later pass can park it. The session of
//! 2026-10-13 ("Forking Sessions") was closed, read as given back, and held its slot.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::SlotParkRefused;
use api::{HostedSessions, Sessions};
use ipc::{ShowWindow, SessionId};

use super::session_gate::{in_its_slot, standing};
use super::session_host::eventually;
use crate::tests::peer::Placing;

async fn close(rig: &super::session_host::Rig, id: &SessionId) {
    let holding = Arc::strong_count(&rig.fleet);
    Arc::clone(&rig.fleet)
        .close_session(ipc::CloseSession { session_id: id.clone() })
        .await
        .expect("closed");
    // The park runs on a task that holds the Fleet until it is done.
    eventually(|| async { Arc::strong_count(&rig.fleet) == holding }).await;
}

#[tokio::test]
async fn a_close_the_pool_refuses_keeps_the_slot_on_the_ledger_and_the_rest_given_back() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);
    rig.fleet.vcs().refuse_next_park(SlotParkRefused::Vcs(String::from("git said no")));

    close(&rig, &id).await;

    assert!(rig.fleet.vcs().parked_slots().is_empty(), "the pool refused");
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"], "still held, and the ledger says so");
    let session = rig.fleet.store().lock().await.session(id.as_str()).unwrap().unwrap();
    assert_eq!(session.state, store::SessionState::Ended, "the session itself ended");
}

#[tokio::test]
async fn a_close_the_pool_accepts_gives_the_slot_back_on_the_ledger() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;

    close(&rig, &id).await;

    assert_eq!(rig.fleet.vcs().parked_slots(), vec![(1, id.as_str().to_string())]);
    assert!(standing(&rig, &id, "slot").await.is_empty());
}

#[tokio::test]
async fn the_boot_pass_parks_a_slot_an_ended_session_still_holds_and_no_other() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    rig.fleet.vcs().refuse_next_park(SlotParkRefused::Vcs(String::from("git said no")));
    close(&rig, &id).await;
    // A Job's slot, and a running session's: neither is an ended session's.
    rig.fleet.vcs().hold_slot(&rig.root, 2, "01JOBNOTASESSION");
    let running = rig.start().await;
    rig.fleet.vcs().hold_slot(&rig.root, 3, running.as_str());
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);

    let served = rig.fleet.served_named(None).expect("served");
    rig.fleet.freed_from_ended_sessions(&served).await;

    assert_eq!(rig.fleet.vcs().parked_slots(), vec![(1, id.as_str().to_string())]);
    assert!(standing(&rig, &id, "slot").await.is_empty(), "the ledger follows the pool");
}

#[tokio::test]
async fn the_boot_pass_leaves_a_slot_the_pool_still_refuses_for_the_next_one() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    rig.fleet.vcs().refuse_next_park(SlotParkRefused::Vcs(String::from("git said no")));
    close(&rig, &id).await;
    rig.fleet.vcs().refuse_next_park(SlotParkRefused::Vcs(String::from("git said no")));

    let served = rig.fleet.served_named(None).expect("served");
    rig.fleet.freed_from_ended_sessions(&served).await;

    assert!(rig.fleet.vcs().parked_slots().is_empty());
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);
}

#[tokio::test]
async fn a_walk_the_session_had_open_ends_with_it() {
    let rig = super::session_host::rig_placing(None);
    let id = rig.start().await;
    rig.fleet
        .show_window(None, ShowWindow {
            url: "http://localhost:5173/one".into(),
            title: None,
            session_id: Some(id.clone()),
            variations: Vec::new(),
            ..Default::default()
        })
        .await
        .expect("shown");
    assert!(rig.fleet.walk_open(id.as_str()).await.is_some());

    close(&rig, &id).await;

    assert_eq!(rig.fleet.walk_open(id.as_str()).await, None);
}

#[tokio::test]
async fn a_close_replies_ended_while_the_park_is_held_and_the_slot_stays_held_until_it_finishes() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    let gate = rig.fleet.vcs().hold_parks();

    let (fleet, closed) = (Arc::clone(&rig.fleet), ipc::CloseSession { session_id: id.clone() });
    let closing = async move { fleet.close_session(closed).await };
    let record = tokio::time::timeout(Duration::from_secs(2), closing)
        .await
        .expect("the reply does not wait on the park")
        .expect("closed");

    assert_eq!(record.state, ipc::SessionState::Ended);
    assert_eq!(record.end_reason.as_deref(), Some("closed"));
    eventually(|| async { gate.arrived() == 1 }).await;
    assert!(rig.fleet.vcs().parked_slots().is_empty(), "the park is still held");
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"], "the slot is still the ledger's until it is parked");

    gate.open();

    eventually(|| async { standing(&rig, &id, "slot").await.is_empty() }).await;
    assert_eq!(rig.fleet.vcs().parked_slots(), vec![(1, id.as_str().to_string())]);
}
