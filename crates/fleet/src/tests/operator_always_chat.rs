//! The Operator page is always a chat while its switch is on: **a send with no session running is kept, starts one at
//! once and is told to it once it is up**, the switch starts the session with no tick, and the page names the newest
//! session, ended ones too, so its thread still draws the talk.

use std::sync::Arc;

use ipc::CloseSession;

use super::operator_cards::said;
use super::operator_session::{live, switched};
use super::peer::Placing;
use super::session_host::{eventually, rig_placing, Rig};
use crate::operator::keep_the_operator_up;

/// The first process the stand-in starts is pid 1000, and this connection is the one it holds.
fn rig() -> Rig {
    let peers = Placing::nothing();
    peers.holding(1000, 50999, 12345);
    rig_placing(Some(peers))
}

fn heard(rig: &Rig, containing: &str) -> usize {
    rig.stand_in.starts().first().map_or(0, |_| rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count())
}

#[tokio::test]
async fn switching_on_starts_the_operators_session_with_no_tick() {
    let rig = rig();
    let _up = keep_the_operator_up(Arc::clone(&rig.fleet));
    switched(&rig, true).await;
    eventually(|| async { live(&rig).await.is_some() }).await;

    switched(&rig, false).await;
    eventually(|| async { live(&rig).await.is_none() }).await;
}

#[tokio::test]
async fn a_send_with_no_session_running_is_kept_starts_one_and_is_told_to_it() {
    let rig = rig();
    // One project, so his words have an Operator to be kept for before any session is up.
    super::operator_projects::project_named(&rig, "Shop", &rig.manifest).await;
    switched(&rig, true).await;
    assert!(live(&rig).await.is_none(), "no tick has run, so none is up");
    rig.fleet.operator_talked(said("Are you there?", None)).await.expect("kept, not refused");

    let _up = keep_the_operator_up(Arc::clone(&rig.fleet));
    eventually(|| async { live(&rig).await.is_some() }).await;
    eventually(|| async { heard(&rig, "Are you there?") == 1 }).await;
    rig.fleet.operator_session_kept().await;
    assert_eq!(heard(&rig, "Are you there?"), 1, "told once, not again on the next tick");
}

#[tokio::test]
async fn a_send_with_the_switch_off_is_refused() {
    let rig = rig();
    let refused = rig.fleet.operator_talked(said("hello", None)).await.expect_err("off");
    assert!(format!("{refused:?}").contains("fleet.operator_session_not_running"), "{refused:?}");
}

#[tokio::test]
async fn the_page_names_the_newest_operator_session_after_it_ended() {
    let rig = rig();
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let id = live(&rig).await.expect("started");
    assert_eq!(rig.fleet.operator_page_wire().await.expect("read").talk_session_id.as_ref().map(|one| one.as_str().to_string()), Some(id.clone()));

    rig.fleet.closed_and_parked(CloseSession { session_id: ipc::SessionId::carried(&id) }).await.expect("closed");
    let page = rig.fleet.operator_page_wire().await.expect("read");
    assert!(page.session_id.is_none(), "none runs");
    assert_eq!(page.talk_session_id.map(|one| one.as_str().to_string()), Some(id), "its thread still holds the talk");
}
