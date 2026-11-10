//! Waiting to start: a person's drag moves a Job and is kept as a correction, and with the Operator
//! off the line stays first come, first served. **The Operator orders nothing**: no ruling places a Job.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{LinePlace, MoveInLine, SessionId};

use super::operator_acting::operator_on;
use super::session_host::Rig;
use super::session_queue::{queued_and_idle, with_slot_one_held};

/// The line for a slot, in its order.
async fn line(rig: &Rig) -> Vec<String> {
    rig.fleet.store().lock().await.slot_lines().expect("read").into_iter().map(|one| one.session_id).collect()
}

fn ids(of: &[&SessionId]) -> Vec<String> {
    of.iter().map(|one| one.as_str().to_string()).collect()
}

fn dragged(id: &SessionId, place: LinePlace) -> MoveInLine {
    MoveInLine { session_id: id.clone(), place }
}

#[tokio::test]
async fn a_drag_moves_a_job_in_waiting_to_start_and_is_kept_as_a_correction() {
    let (rig, _holder) = with_slot_one_held().await;
    operator_on(&rig).await;
    let (first, _) = queued_and_idle(&rig, "first").await;
    let (second, _) = queued_and_idle(&rig, "second").await;
    let (third, _) = queued_and_idle(&rig, "third").await;
    assert_eq!(line(&rig).await, ids(&[&first, &second, &third]), "first come, first served: the Operator orders nothing");

    // The person drags the second ahead of the first.
    Arc::clone(&rig.fleet).move_in_line(dragged(&second, LinePlace::Ahead { session_id: first.clone() })).await.expect("moved");
    assert_eq!(line(&rig).await, ids(&[&second, &first, &third]));
    let manifest = rig.manifest.as_str().to_string();
    assert_eq!(rig.fleet.hosts().slot_queue().ahead_of(&manifest, first.as_str()), 1, "served in the new order");
    let log = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    let corrected = log.iter().find(|row| row.outcome == "corrected").expect("the drag is in the Log");
    assert_eq!(
        (corrected.session_id.as_str(), corrected.by.as_str(), corrected.source.as_str(), corrected.place.clone()),
        (second.as_str(), "human", "person", Some(format!("ahead:{}", first.as_str())))
    );

    // A place in no line is refused, and nothing moves.
    let refused = Arc::clone(&rig.fleet).move_in_line(dragged(&first, LinePlace::Behind { session_id: SessionId::carried("01GONE") })).await;
    assert!(refused.is_err());
    assert_eq!(line(&rig).await, ids(&[&second, &first, &third]));
}

#[tokio::test]
async fn with_the_operator_off_waiting_to_start_is_first_come_first_served() {
    let (rig, _holder) = with_slot_one_held().await;
    let (first, _) = queued_and_idle(&rig, "first").await;
    let (second, _) = queued_and_idle(&rig, "second").await;

    let refused = Arc::clone(&rig.fleet).move_in_line(dragged(&second, LinePlace::Ahead { session_id: first.clone() })).await;

    let refused = refused.expect_err("no drag while the Operator is off");
    assert!(format!("{refused:?}").contains("fleet.line_unordered"), "{refused:?}");
    assert_eq!(line(&rig).await, ids(&[&first, &second]), "in the order they came");
}
