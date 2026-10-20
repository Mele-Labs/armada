//! A hosted job saying a Fleet notice was wrong, and the review job in Armada that settles it.
//!
//! **The acceptance case is the real one**: the subagent-spend notice fired on a job whose reading
//! agents were already on sonnet, and the agent said so.

use std::sync::Arc;

use api::Sessions;
use config::Manifest;
use ipc::{DisputeFleet, DisputeSpotted, SessionId, SessionRow, SessionVoice};

use super::daemon::workflow_named;
use super::session_host::{eventually, rig_fitted, Rig};

const NOTICE: &str = "Subagents burned 5.0 units in your last hour, 87% of it, on opus. Pass model sonnet on reading and searching agents.";
const WHY: &str = "My reading agents already run on sonnet; the opus spend is the two building agents, which the rules say run on the job's own model.";
const BEFORE: &str = "Dispatched two sonnet readers and two opus builders.";
const AFTER: &str = "That notice is wrong: the readers are on sonnet already.";

/// A rig serving Armada's own repository, holding the `bug` workflow.
pub(super) fn serving_armada() -> Rig {
    serving("armada")
}

pub(super) fn serving(manifest: &str) -> Rig {
    let manifest = Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: {manifest}\n"),
    )
    .expect("a manifest");
    rig_fitted(|fittings| {
        let starting = fittings.starting();
        starting.manifest = manifest;
        starting.workflows = ["bug", "feature"]
            .into_iter()
            .map(workflow_named)
            .map(|workflow| (workflow.id().clone(), workflow))
            .collect();
    })
}

/// The agent's text, as a row of its thread.
async fn agent_said(rig: &Rig, id: &SessionId, process: usize, text: &str) {
    rig.stand_in.finishes(process, text);
    eventually(|| async {
        rig.rows(id).await.iter().any(|row| {
            matches!(row, SessionRow::Message { from: SessionVoice::Agent, text: said, .. } if said == text)
        })
    })
    .await;
}

/// A job that has worked a turn, been sent `notice` by Fleet, and answered it.
pub(super) async fn told(rig: &Rig, notice: &str) -> SessionId {
    let id = rig.start_titled(Some("Build the dispute call")).await;
    rig.send(&id, "Build the dispute call").await;
    rig.stand_in.init(0);
    agent_said(rig, &id, 0, BEFORE).await;
    assert!(rig.fleet.told_by_fleet(id.as_str(), notice).await.expect("told"));
    agent_said(rig, &id, 0, AFTER).await;
    id
}

pub(super) fn disputing(id: &SessionId, why: &str) -> DisputeFleet {
    DisputeFleet { why: why.to_string(), session_id: Some(id.clone()) }
}

/// Every job on the ledger but `but`.
pub(super) async fn others(rig: &Rig, but: &SessionId) -> Vec<store::KeptSession> {
    let store = rig.fleet.store().lock().await;
    let all = store.find_sessions(&store::SessionSearch::default()).expect("the ledger");
    all.into_iter().filter(|one| one.id != but.as_str()).collect()
}

/// The first message of `review`'s thread.
pub(super) async fn brief_of(rig: &Rig, review: &str) -> String {
    let rows = rig.rows(&SessionId::carried(review)).await;
    rows.iter()
        .find_map(|row| match row {
            SessionRow::Message { text, .. } => Some(text.clone()),
            _ => None,
        })
        .expect("a first message")
}

#[tokio::test]
async fn a_disputed_subagent_notice_starts_one_review_in_armada_carrying_the_notice_the_reason_and_the_thread() {
    let rig = serving_armada();
    let id = told(&rig, NOTICE).await;

    let disputed = Arc::clone(&rig.fleet).dispute_fleet(None, disputing(&id, WHY)).await.expect("disputed");

    let review = disputed.review_session_id.clone().expect("a review job");
    let started = others(&rig, &id).await;
    assert_eq!(started.len(), 1, "{started:?}");
    assert_eq!(started[0].id, review);
    assert_eq!(started[0].manifest_id.as_deref(), Some("armada"));
    assert_eq!(started[0].title.as_deref(), Some("Review a disputed Fleet notice"));
    let hosting = rig.fleet.store().lock().await.hosting(&review).unwrap().expect("hosted");
    assert_eq!(hosting.workflow.as_deref(), Some("bug"));

    let brief = brief_of(&rig, &review).await;
    for carried in [NOTICE, WHY, BEFORE, AFTER] {
        assert!(brief.contains(carried), "the brief lacks {carried:?}:\n{brief}");
    }

    let rows = rig.rows(&id).await;
    let notice = rows
        .iter()
        .find_map(|row| match row {
            SessionRow::Message { id, text, .. } if text == NOTICE => Some(id.clone()),
            _ => None,
        })
        .expect("the notice row");
    let row = rows.iter().find(|row| matches!(row, SessionRow::Disputed { .. })).expect("a disputed row");
    match row {
        SessionRow::Disputed { notice: named, why, spotted, review: reviewed, .. } => {
            assert_eq!(named, &notice);
            assert_eq!(why, WHY);
            assert_eq!(*spotted, DisputeSpotted::Called);
            assert_eq!(reviewed.as_deref(), Some(review.as_str()));
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn a_job_fleet_told_nothing_is_refused_and_nothing_is_started() {
    let rig = serving_armada();
    let id = rig.start_titled(Some("Build the dispute call")).await;
    rig.send(&id, "Build the dispute call").await;

    let refused = Arc::clone(&rig.fleet).dispute_fleet(None, disputing(&id, WHY)).await.unwrap_err();

    assert!(refused.error().message.contains("no notice"), "{refused:?}");
    assert!(others(&rig, &id).await.is_empty());
    assert!(!rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Disputed { .. })));
}

#[tokio::test]
async fn a_dispute_with_no_reason_is_refused() {
    let rig = serving_armada();
    let id = told(&rig, NOTICE).await;

    let refused = Arc::clone(&rig.fleet).dispute_fleet(None, disputing(&id, "  ")).await.unwrap_err();

    assert!(refused.error().message.contains("`why`"), "{refused:?}");
    assert!(others(&rig, &id).await.is_empty());
}
