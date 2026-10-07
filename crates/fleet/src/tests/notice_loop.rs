//! The pull request and issue rotations are read beside the turn: with the
//! roster held, so no turn can pass its first line, each is still noticed.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::Landing;
use core_model::Timestamp;
use testkit::{FakeJudge, FakeLinkLookup, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::fittings;
use crate::tests::noticing::{a_finished_job, a_fleet_asking_every_turn};
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

#[tokio::test]
async fn a_merge_is_noticed_while_a_turn_cannot_run() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_asking_every_turn(&home));
    let job_id = a_finished_job(&fleet, &home).await;
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from("https://forge.invalid/armada/pull/1"),
    });

    let held = fleet.slots().lock().await;
    let turn = {
        let fleet = Arc::clone(&fleet);
        tokio::spawn(async move { fleet.turn().await })
    };
    let noticing = crate::notice_loop::keep_noticing(
        Arc::clone(&fleet),
        Duration::from_millis(5),
        |why| panic!("not read: {why}"),
    );
    let mut landed = false;
    for _ in 0..200 {
        let record = fleet.store().lock().await.landed_by_job().unwrap();
        landed = matches!(record.get(&job_id), Some(Landing::Merged { .. }));
        if landed {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!turn.is_finished(), "the turn is the one that is stuck");
    noticing.abort();
    drop(held);
    turn.await.unwrap().unwrap();
    assert!(landed, "the merge was recorded without a turn");
}

#[tokio::test]
async fn an_edited_issue_is_noticed_while_a_turn_cannot_run() {
    let home = TempDir::new();
    let links = Arc::new(
        FakeLinkLookup::resolving("example.test/issues/9", "the reader drops the last row")
            .linking("y#9", "https://example.test/x/y/issues/9"),
    );
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = Arc::new(FakeJudge::saying(
        "workflow: bug\ntitle: The reader drops the last row\ndone_when: the last row is read",
    ));
    fittings.links = links.clone();
    fittings.noticing = Noticing::every(Duration::ZERO);
    let fleet = Arc::new(Fleet::assembled(fittings));
    let made = fleet
        .propose_from("https://example.test/issues/9", None)
        .await
        .expect("a proposal");
    let job = made[0].id().clone();
    links.edit_issue_at("2099-01-01T00:00:00Z");

    let held = fleet.slots().lock().await;
    let turn = {
        let fleet = Arc::clone(&fleet);
        tokio::spawn(async move { fleet.turn().await })
    };
    let noticing = crate::notice_loop::keep_noticing(
        Arc::clone(&fleet),
        Duration::from_millis(5),
        |why| panic!("not read: {why}"),
    );
    let mut moved = None;
    for _ in 0..200 {
        moved = fleet
            .store()
            .lock()
            .await
            .issue_source(&job)
            .unwrap()
            .and_then(|source| source.moved_at);
        if moved.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!turn.is_finished(), "the turn is the one that is stuck");
    noticing.abort();
    drop(held);
    turn.await.unwrap().unwrap();
    assert_eq!(
        moved,
        Some(Timestamp::from_rfc3339("2099-01-01T00:00:00Z"))
    );
}
