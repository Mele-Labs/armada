//! A Fleet notice the job pushed back on only in its replies, read off them once its wait is over.
//!
//! **The model is the fake and the process is the stand-in**, as the proposer's cases are: what
//! runs is which notices Fleet reads, when, what it asks, and what it keeps of the answer.

use std::sync::Arc;

use ipc::{DisputeSpotted, SessionId, SessionRow};
use testkit::FakeJudge;

use super::planted::Held;
use super::session_dispute::{others, reviews_switched, serving_fitted, told, told_answering, AFTER, NOTICE};
use super::session_host::Rig;
use crate::session_host::dispute_reading::PUSHBACK;

/// Past the shipped ten minutes.
const WAITED: u64 = 11 * 60;

/// A rig serving Armada, whose clock the case moves and whose model answers `answer`.
async fn reading(answer: &str) -> (Rig, Arc<Held>, Arc<FakeJudge>) {
    let clock = Arc::new(Held::started());
    let judge = Arc::new(FakeJudge::saying(answer));
    let (planted_clock, planted_judge) = (Arc::clone(&clock), Arc::clone(&judge));
    let rig = serving_fitted("armada", move |fittings| {
        fittings.clock = planted_clock;
        fittings.judge = planted_judge;
    })
    .await;
    (rig, clock, judge)
}

/// The questions put about a notice's replies, the proposer's apart.
fn read_asks(judge: &FakeJudge) -> Vec<String> {
    judge.asked().into_iter().filter(|asked| asked.contains(PUSHBACK)).collect()
}

async fn disputes(rig: &Rig, id: &SessionId) -> Vec<SessionRow> {
    rig.rows(id).await.into_iter().filter(|row| matches!(row, SessionRow::Disputed { .. })).collect()
}

#[tokio::test]
async fn pushback_said_only_in_prose_is_kept_once_as_a_dispute_read_off_the_replies() {
    let (rig, clock, judge) = reading("pushback: yes\nwhy: the readers are on sonnet already").await;
    let id = told(&rig, NOTICE).await;

    rig.fleet.notices_read().await;
    assert!(read_asks(&judge).is_empty(), "read before its wait was over");

    clock.on(WAITED);
    rig.fleet.notices_read().await;
    rig.fleet.notices_read().await;

    let asks = read_asks(&judge);
    assert_eq!(asks.len(), 1, "{asks:?}");
    assert!(asks[0].contains(NOTICE) && asks[0].contains(AFTER), "{}", asks[0]);
    let kept = disputes(&rig, &id).await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    match &kept[0] {
        SessionRow::Disputed { why, spotted, review, .. } => {
            assert_eq!(why, "the readers are on sonnet already");
            assert_eq!(*spotted, DisputeSpotted::Read);
            let started = others(&rig, &id).await;
            assert_eq!(review.as_deref(), started.first().map(|one| one.id.as_str()));
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn agreement_is_read_and_kept_as_nothing() {
    let (rig, clock, judge) = reading("pushback: no\nwhy:").await;
    let id = told_answering(&rig, NOTICE, Some("Good catch. I moved the readers to sonnet.")).await;

    clock.on(WAITED);
    rig.fleet.notices_read().await;

    assert_eq!(read_asks(&judge).len(), 1);
    assert!(disputes(&rig, &id).await.is_empty());
    assert!(others(&rig, &id).await.is_empty());
}

#[tokio::test]
async fn silence_is_not_read_and_waits_for_a_reply() {
    let (rig, clock, judge) = reading("pushback: yes\nwhy: anything").await;
    let id = told_answering(&rig, NOTICE, None).await;

    clock.on(WAITED);
    rig.fleet.notices_read().await;

    assert!(read_asks(&judge).is_empty());
    assert!(disputes(&rig, &id).await.is_empty());
    assert_eq!(rig.fleet.store().lock().await.unread_fleet_notices().unwrap().len(), 1);
}

#[tokio::test]
async fn news_is_not_kept_to_be_read() {
    let (rig, _, _) = reading("pushback: no").await;
    let id = rig.start_titled(Some("Build the dispute call")).await;
    rig.send(&id, "Build the dispute call").await;

    assert!(rig.fleet.told_news_by_fleet(id.as_str(), "Pull request #12 merged.").await.unwrap());

    assert!(rig.fleet.store().lock().await.unread_fleet_notices().unwrap().is_empty());
}

#[tokio::test]
async fn with_dispute_reviews_off_no_reply_is_read_and_no_notice_is_kept_to_be() {
    let (rig, clock, judge) = reading("pushback: yes\nwhy: the readers are on sonnet already").await;
    reviews_switched(&rig, false).await;
    let id = told(&rig, NOTICE).await;

    clock.on(WAITED);
    rig.fleet.notices_read().await;

    assert!(read_asks(&judge).is_empty(), "{:?}", read_asks(&judge));
    assert!(disputes(&rig, &id).await.is_empty());
    assert!(others(&rig, &id).await.is_empty());
    assert!(rig.fleet.store().lock().await.unread_fleet_notices().unwrap().is_empty());
}

#[tokio::test]
async fn a_notice_kept_while_reviews_were_on_waits_unread_until_they_are_on_again() {
    let (rig, clock, judge) = reading("pushback: yes\nwhy: the readers are on sonnet already").await;
    let id = told(&rig, NOTICE).await;
    reviews_switched(&rig, false).await;

    clock.on(WAITED);
    rig.fleet.notices_read().await;
    assert!(read_asks(&judge).is_empty(), "{:?}", read_asks(&judge));
    assert_eq!(rig.fleet.store().lock().await.unread_fleet_notices().unwrap().len(), 1);

    reviews_switched(&rig, true).await;
    rig.fleet.notices_read().await;
    assert_eq!(read_asks(&judge).len(), 1);
    assert_eq!(disputes(&rig, &id).await.len(), 1);
}
