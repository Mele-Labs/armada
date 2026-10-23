//! A hosted session refused a slot because the pool is full waits in a line, and
//! is leased one and moved into it when one frees: in order, with the turn that
//! was refused already over, and not if it has ended meanwhile.

use std::sync::Arc;
use std::time::Duration;

use api::HostedSessions;
use ipc::{CloseSession, SessionId, SessionTurn};

use super::session_host::{denied, eventually, reason, rig_placing, Rig};
use crate::session_host::queueing::keep_granting_slots;

const OTHERS: &str = "an agent's session";

/// A rig whose pool is full but for slot 1, with a session standing in slot 1.
pub(super) async fn with_slot_one_held() -> (Rig, SessionId) {
    let rig = rig_placing(None);
    for n in 2..=8 {
        rig.fleet.vcs().hold_slot(&rig.root, n, OTHERS);
    }
    let holder = rig.start().await;
    rig.send(&holder, "hold a slot").await;
    let first = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(first);
    let held = rig.gate(&holder, "Edit", &write_to_main(&rig)).await;
    assert!(denied(&held) && reason(&held).contains("Slot 1"));
    (rig, holder)
}

pub(super) fn write_to_main(rig: &Rig) -> String {
    format!(r#"{{"file_path":"{}/src/lib.rs"}}"#, rig.root)
}

/// A session that has written, been queued, and stopped its turn as told.
pub(super) async fn queued_and_idle(rig: &Rig, text: &str) -> (SessionId, String) {
    let id = rig.start().await;
    rig.send(&id, text).await;
    let at = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(at);
    let answer = rig.gate(&id, "Edit", &write_to_main(rig)).await;
    assert!(denied(&answer));
    rig.stand_in.finishes(at, "waiting for a slot");
    eventually(|| async { rig.turn(&id).await == SessionTurn::Idle }).await;
    (id, reason(&answer))
}

pub(super) async fn close(rig: &Rig, id: &SessionId) {
    Arc::clone(&rig.fleet)
        .close_session(CloseSession {
            session_id: id.clone(),
        })
        .await
        .expect("closed");
}

/// The starts of `id` that resumed it, which is how a move into a slot shows.
pub(super) fn resumes(rig: &Rig, id: &SessionId) -> Vec<crate::session_host::Start> {
    rig.stand_in
        .starts()
        .into_iter()
        .filter(|start| start.session == id.as_str() && start.resuming)
        .collect()
}

pub(super) fn the_keeper(rig: &Rig) -> tokio::task::JoinHandle<()> {
    keep_granting_slots(Arc::clone(&rig.fleet), Duration::from_secs(3600))
}

#[tokio::test]
async fn a_full_pool_queues_the_session_and_a_freed_slot_moves_it_in() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let (waiting, told) = queued_and_idle(&rig, "change a file").await;
    assert!(told.contains("first in the queue"), "{told}");
    assert!(told.contains("Stop here"), "{told}");
    assert!(resumes(&rig, &waiting).is_empty(), "nothing to move into yet");

    close(&rig, &holder).await;

    eventually(|| async { resumes(&rig, &waiting).len() == 1 }).await;
    let moved = &resumes(&rig, &waiting)[0];
    assert_eq!(moved.directory, adapter_traits::slot_path(&rig.root, 1));
    let at = rig.stand_in.starts().len() - 1;
    let told = rig.stand_in.process(at).sent.lock().unwrap()[0].clone();
    assert!(told.contains("You are now in"), "{told}");
    let record = Arc::clone(&rig.fleet).get_session(waiting.clone()).await.unwrap();
    assert!(record
        .session
        .attachments
        .iter()
        .any(|one| one.kind == "slot" && one.target == "1"));
    keeper.abort();
}

#[tokio::test]
async fn two_queued_sessions_are_served_in_the_order_they_joined() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let (first, _) = queued_and_idle(&rig, "first").await;
    let (second, told) = queued_and_idle(&rig, "second").await;
    assert!(told.contains("1 ahead of it"), "{told}");
    let (third, told) = queued_and_idle(&rig, "third").await;
    assert!(told.contains("2 ahead of it"), "{told}");

    close(&rig, &holder).await;
    eventually(|| async { resumes(&rig, &first).len() == 1 }).await;
    assert!(resumes(&rig, &second).is_empty() && resumes(&rig, &third).is_empty());

    close(&rig, &first).await;
    eventually(|| async { resumes(&rig, &second).len() == 1 }).await;
    assert!(resumes(&rig, &third).is_empty());
    assert_eq!(
        resumes(&rig, &second)[0].directory,
        adapter_traits::slot_path(&rig.root, 1)
    );
    keeper.abort();
}

#[tokio::test]
async fn a_session_that_ends_while_queued_leaves_the_line() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let (gone, _) = queued_and_idle(&rig, "gone").await;
    let (next, _) = queued_and_idle(&rig, "next").await;
    close(&rig, &gone).await;

    // It was ahead of `next` and is not any more.
    let again = rig.gate(&next, "Edit", &write_to_main(&rig)).await;
    assert!(reason(&again).contains("first in the queue"), "{}", reason(&again));

    close(&rig, &holder).await;
    eventually(|| async { resumes(&rig, &next).len() == 1 }).await;
    assert!(resumes(&rig, &gone).is_empty(), "an ended session is not given a slot");
    keeper.abort();
}

/// The refused turn may not be over when the slot frees: the move then waits for
/// the turn's end, as a slot leased by the gate itself does.
#[tokio::test]
async fn a_session_still_finishing_its_turn_is_moved_when_it_ends() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let waiting = rig.start().await;
    rig.send(&waiting, "change a file").await;
    let at = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(at);
    assert!(denied(&rig.gate(&waiting, "Edit", &write_to_main(&rig)).await));

    close(&rig, &holder).await;
    eventually(|| async {
        let moving = rig.fleet.hosts().of(waiting.as_str()).state().moving.clone();
        moving.is_some()
    })
    .await;
    assert!(resumes(&rig, &waiting).is_empty(), "its turn is still running");

    rig.stand_in.finishes(at, "stopping");
    eventually(|| async { resumes(&rig, &waiting).len() == 1 }).await;
    keeper.abort();
}

/// `armada worktree release` frees a slot without Fleet being told.
#[tokio::test]
async fn a_slot_freed_where_fleet_did_not_see_it_is_found_on_the_timer() {
    let rig = rig_placing(None);
    for n in 1..=8 {
        rig.fleet.vcs().hold_slot(&rig.root, n, OTHERS);
    }
    let keeper = keep_granting_slots(Arc::clone(&rig.fleet), Duration::from_millis(50));
    let (waiting, _) = queued_and_idle(&rig, "change a file").await;
    assert!(resumes(&rig, &waiting).is_empty());

    rig.fleet.vcs().free_slot(&rig.root, 3);

    eventually(|| async { resumes(&rig, &waiting).len() == 1 }).await;
    assert_eq!(
        resumes(&rig, &waiting)[0].directory,
        adapter_traits::slot_path(&rig.root, 3)
    );
    keeper.abort();
}

#[tokio::test]
async fn a_waiting_session_is_not_moved_into_a_slot_until_the_closing_ones_park_has_finished() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let (waiting, _) = queued_and_idle(&rig, "change a file").await;
    let gate = rig.fleet.vcs().hold_parks();

    tokio::time::timeout(Duration::from_secs(2), close(&rig, &holder))
        .await
        .expect("the reply does not wait on the park");
    eventually(|| async { gate.arrived() == 1 }).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(resumes(&rig, &waiting).is_empty(), "the slot is not free while it is mid-park");

    gate.open();

    eventually(|| async { resumes(&rig, &waiting).len() == 1 }).await;
    keeper.abort();
}
