//! A person freeing the bay a hosted session holds, from Cleanup, with the session's own id as the
//! holder. `docs/concepts/fleet.md`, *Worktree slots*.

use api::Commands;
use ipc::{ChangeSlotPool, SessionId, SessionTurn, SlotAct, SlotPoolChanged};

use super::session_gate::{in_its_slot, standing};
use super::session_host::{eventually, Rig};
use crate::tests::peer::Placing;

async fn release(rig: &Rig, id: &SessionId) -> Result<SlotPoolChanged, api::Refusal> {
    Commands::change_slot_pool(
        &*rig.fleet,
        ChangeSlotPool {
            act: SlotAct::Release,
            slot: Some(1),
            holder: Some(id.as_str().to_string()),
        },
        None,
    )
    .await
}

async fn idle(rig: &Rig, id: &SessionId) {
    rig.stand_in.finishes(1, "done");
    eventually(|| async { rig.turn(id).await == SessionTurn::Idle }).await;
}

fn refused_with(refused: api::Refusal, code: &str) -> ipc::WireError {
    let api::Refusal::IllegalMove(error) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(error.code, code);
    error
}

#[tokio::test]
async fn an_idle_sessions_slot_is_parked_and_its_rows_given_back() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    idle(&rig, &id).await;
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);

    let released = release(&rig, &id).await.expect("released");

    assert_eq!(released.slot, 1);
    assert!(released.released.is_some());
    assert_eq!(rig.fleet.vcs().parked_slots(), vec![(1, id.as_str().to_string())]);
    assert!(standing(&rig, &id, "slot").await.is_empty());
    assert!(standing(&rig, &id, "branch").await.is_empty());
    let hosting = rig.fleet.store().lock().await.hosting(id.as_str()).unwrap().unwrap();
    assert_eq!(hosting.lease_slot, None, "its next message does not resume into a free bay");
}

#[tokio::test]
async fn a_working_sessions_slot_is_refused_and_its_rows_stand() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    assert_eq!(rig.turn(&id).await, SessionTurn::Working { woken_by: None });

    let refused = release(&rig, &id).await.expect_err("it is working");

    let error = refused_with(refused, "fleet.slot_session_working");
    assert!(error.message.ends_with(" is working"), "{}", error.message);
    assert!(rig.fleet.vcs().parked_slots().is_empty());
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);
}

#[tokio::test]
async fn a_slot_another_holds_since_is_refused_and_the_rows_stand() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    idle(&rig, &id).await;
    rig.fleet.vcs().hold_slot(&rig.root, 1, "01ANOTHERSESSION");

    let refused = release(&rig, &id).await.expect_err("re-leased");

    refused_with(refused, "fleet.slot_holder_changed");
    assert!(rig.fleet.vcs().parked_slots().is_empty());
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);
}

#[tokio::test]
async fn a_released_slot_leaves_a_release_row_in_the_thread() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    idle(&rig, &id).await;

    release(&rig, &id).await.expect("released");

    assert!(rig
        .rows(&id)
        .await
        .iter()
        .any(|row| matches!(row, ipc::SessionRow::Release { slot: 1, .. })));
}
