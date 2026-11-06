//! The Operator's order of Waiting to start, and its holds. A place it ruled is taken as the Job
//! joins the line for a slot; a person's drag moves a Job over it and is kept as a correction; with
//! the Operator off the line stays first come, first served. Of two Jobs kept in touch, the one
//! that has not begun is held with a `StartAfter` until the first pushes, a working one is not, and
//! the hold lifts, logged, when the first ends without pushing.

use std::sync::Arc;
use std::time::Duration;

use api::HostedSessions;
use ipc::{LinePlace, MoveInLine, SessionId};

use super::operator_acting::{asked, dispatch, shared};
use super::session_host::{denied, eventually, rig_placing, Rig};
use super::session_queue::{close, queued_and_idle, with_slot_one_held, write_to_main};
use fleet::operator::{Acting, FleetActing, Place};

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
async fn the_operators_place_orders_waiting_to_start_and_a_drag_moves_it_again_as_a_correction() {
    let (rig, _holder) = with_slot_one_held().await;
    dispatch(&rig, true).await;
    let (first, _) = queued_and_idle(&rig, "first").await;
    let (second, _) = queued_and_idle(&rig, "second").await;

    // Ruled ahead of the first before it wrote, it takes that place as it joins.
    let third = rig.start().await;
    rig.send(&third, "third").await;
    FleetActing::on(&rig.fleet).place(third.as_str(), &Place::Ahead(first.as_str().into()));
    tokio::time::sleep(Duration::from_millis(200)).await;
    let at = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(at);
    assert!(denied(&rig.gate(&third, "Edit", &write_to_main(&rig)).await), "queued");
    assert_eq!(line(&rig).await, ids(&[&third, &first, &second]), "the Operator's order, kept");

    // Ruled again while it waits, it moves at once.
    FleetActing::on(&rig.fleet).place(third.as_str(), &Place::Last);
    eventually(|| async { line(&rig).await == ids(&[&first, &second, &third]) }).await;

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

#[tokio::test]
async fn a_start_held_until_the_first_pushes_lifts_when_it_ends_without_pushing_and_a_working_job_is_not_held() {
    let rig = rig_placing(None);
    dispatch(&rig, true).await;
    let first = asked(&rig, "Build the tab host the terminal opens in").await;
    let held = rig.start_held().await;
    let working = asked(&rig, "Open terminals in tabs").await;
    let acting = FleetActing::on(&rig.fleet);

    acting.hold(held.as_str(), first.as_str(), &shared());
    acting.hold(working.as_str(), first.as_str(), &shared());

    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let (fleet, id) = (Arc::clone(&fleet), held.as_str().to_string());
        async move { fleet.store().lock().await.wait_of(&id).ok().flatten().is_some() }
    })
    .await;
    let wait = rig.fleet.store().lock().await.wait_of(held.as_str()).expect("read").expect("held");
    assert_eq!((wait.on_session.as_str(), wait.condition.as_str(), wait.by.as_str()), (first.as_str(), "pr_pushed", "operator"));
    assert!(wait.held, "once the wait ends it is on his start card, as it was");
    let rows = || async { rig.fleet.store().lock().await.operator_rulings(10).expect("read") };
    eventually(|| async { rows().await.iter().any(|row| row.outcome == "not_held") }).await;
    let not_held = rows().await.into_iter().find(|row| row.outcome == "not_held").expect("logged");
    assert_eq!((not_held.session_id.as_str(), not_held.by.as_str()), (working.as_str(), "fleet"));
    assert!(not_held.because.contains("already working"), "{}", not_held.because);
    assert!(rig.fleet.store().lock().await.wait_of(working.as_str()).expect("read").is_none(), "a working Job is not paused");

    // The first ends with no pull request: the hold lifts, and the Log says so.
    close(&rig, &first).await;
    rig.fleet.waits_judged(first.as_str()).await;

    let lifted = rows().await.into_iter().find(|row| row.outcome == "hold_lifted").expect("the Log says the hold lifted");
    assert_eq!((lifted.session_id.as_str(), lifted.first_session.as_deref()), (held.as_str(), Some(first.as_str())));
    assert!(lifted.because.contains("no pull request"), "{}", lifted.because);
    let wait = rig.fleet.store().lock().await.wait_of(held.as_str()).expect("read").expect("kept");
    assert!(wait.started_at.is_some(), "it waits on nothing now");
}
