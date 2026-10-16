//! The person giving back what a Session holds, or handing it to another, from Bridge.
//! `docs/concepts/session.md`, *Releasing what a Session holds*.

use api::HostedSessions;
use ipc::{ChosenHold, ReleaseSessionHolds, SessionId, SessionTurn};
use store::{AttachmentState, Holder, KeptAttachment};

use super::session_gate::{in_its_slot, standing};
use super::session_host::Rig;
use crate::tests::peer::Placing;

async fn release(rig: &Rig, id: &SessionId, to: Option<&SessionId>) -> Result<ipc::SessionHoldsReleased, api::Refusal> {
    chosen(rig, id, to, None).await
}

async fn chosen(
    rig: &Rig,
    id: &SessionId,
    to: Option<&SessionId>,
    rows: Option<&[(&str, &str)]>,
) -> Result<ipc::SessionHoldsReleased, api::Refusal> {
    let rows = rows.map(|rows| rows.iter().map(|(kind, target)| ChosenHold { kind: kind.to_string(), target: target.to_string() }).collect());
    rig.fleet
        .release_session_holds(ReleaseSessionHolds { session_id: id.clone(), to_session_id: to.cloned(), rows })
        .await
}

async fn opened(rig: &Rig, id: &SessionId, number: &str) {
    let manifest = rig.fleet.store().lock().await.session(id.as_str()).unwrap().unwrap().manifest_id.unwrap();
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(id.as_str()),
                kind: String::from("pr"),
                manifest_id: manifest,
                target: number.to_string(),
                state: AttachmentState::Standing,
                detail: Default::default(),
                since: String::from("2026-10-16T09:00:00Z"),
                changed_at: String::from("2026-10-16T09:00:00Z"),
            },
            false,
        )
        .unwrap();
}

#[tokio::test]
async fn a_working_session_is_stopped_and_gives_everything_back() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    opened(&rig, &id, "2430").await;
    assert_eq!(rig.turn(&id).await, SessionTurn::Working { woken_by: None });

    let released = release(&rig, &id, None).await.expect("released");

    assert_eq!(rig.turn(&id).await, SessionTurn::Idle);
    assert_eq!(rig.fleet.vcs().parked_slots(), vec![(1, id.as_str().to_string())]);
    let slot = released.moved.iter().find(|one| one.kind == "slot").expect("the slot");
    assert!(slot.released.is_some(), "what the slot held was committed to its branch");
    assert!(released.moved.iter().any(|one| one.kind == "pr" && one.target == "2430"));
    assert!(released.kept.is_empty(), "{:?}", released.kept);
    for kind in ["slot", "branch", "pr"] {
        assert!(standing(&rig, &id, kind).await.is_empty(), "{kind}");
    }
    let session = rig.fleet.store().lock().await.session(id.as_str()).unwrap().unwrap();
    assert_eq!(session.state, store::SessionState::Live, "the session is not ended");
}

#[tokio::test]
async fn a_hand_moves_the_lease_and_every_row_to_the_other_session() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    opened(&rig, &id, "2430").await;
    let other = rig.start_titled(Some("Takes it over")).await;

    let handed = release(&rig, &id, Some(&other)).await.expect("handed");

    assert_eq!(handed.to_session_id.as_ref(), Some(&other));
    assert!(rig.fleet.vcs().parked_slots().is_empty(), "nothing is committed on a hand");
    assert_eq!(rig.fleet.vcs().slot_holders(&rig.root).first().cloned().flatten().as_deref(), Some(other.as_str()));
    for kind in ["slot", "branch", "pr"] {
        assert!(standing(&rig, &id, kind).await.is_empty(), "{kind} left the source");
    }
    assert_eq!(standing(&rig, &other, "slot").await, vec!["1"]);
    assert_eq!(standing(&rig, &other, "pr").await, vec!["2430"]);
    let rows = rig.fleet.store().lock().await.attachments_of(&Holder::session(other.as_str())).unwrap();
    let pr = rows.iter().find(|one| one.kind == "pr").expect("the pr row");
    assert_eq!(pr.detail.get("handed").map(String::as_str), Some(format!("session {}", id.as_str()).as_str()));
    let hosting = rig.fleet.store().lock().await.hosting(other.as_str()).unwrap().unwrap();
    assert_eq!(hosting.lease_slot, Some(1), "the target works in the slot it was handed");
}

#[tokio::test]
async fn a_hand_to_itself_is_refused_and_nothing_moves() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;

    let refused = release(&rig, &id, Some(&id)).await.expect_err("itself");

    let api::Refusal::IllegalMove(error) = refused else { panic!("{refused:?}") };
    assert_eq!(error.code, "fleet.hand_to_itself");
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);
    assert!(rig.fleet.vcs().parked_slots().is_empty());
}

#[tokio::test]
async fn only_the_rows_ticked_move_and_a_working_turn_runs_on_without_a_slot_among_them() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    opened(&rig, &id, "2430").await;
    opened(&rig, &id, "2431").await;
    let other = rig.start_titled(Some("Takes it over")).await;

    let handed = chosen(&rig, &id, Some(&other), Some(&[("pr", "2430")])).await.expect("handed");

    assert_eq!(rig.turn(&id).await, SessionTurn::Working { woken_by: None }, "no slot was chosen, so the turn runs on");
    assert_eq!(handed.moved.iter().map(|one| (one.kind.as_str(), one.target.as_str())).collect::<Vec<_>>(), vec![("pr", "2430")]);
    assert_eq!(standing(&rig, &id, "pr").await, vec!["2431"]);
    assert_eq!(standing(&rig, &id, "slot").await, vec!["1"]);
    assert_eq!(standing(&rig, &other, "pr").await, vec!["2430"]);
    assert!(standing(&rig, &other, "slot").await.is_empty());
}

#[tokio::test]
async fn a_row_the_session_does_not_hold_is_refused_and_nothing_moves() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    opened(&rig, &id, "2430").await;

    let refused = chosen(&rig, &id, None, Some(&[("pr", "2430"), ("pr", "9999")])).await.expect_err("not held");

    let api::Refusal::IllegalMove(error) = refused else { panic!("{refused:?}") };
    assert_eq!(error.code, "fleet.hold_not_standing");
    assert_eq!(standing(&rig, &id, "pr").await, vec!["2430"]);
    assert_eq!(rig.turn(&id).await, SessionTurn::Working { woken_by: None });
}
