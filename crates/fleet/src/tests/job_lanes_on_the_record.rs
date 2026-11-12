//! The lanes a Job's record carries: what `crate::lane_keeping` hands `crate::job_lanes` and where it sets the
//! field. The lanes themselves are `job_lanes`'s cases; these are the Job and the record around them.

use std::sync::Arc;

use api::{HostedSessions, Next};
use core_model::{ForgeFact, Observed};
use ipc::{CloseSession, Event, JobLanesView, SessionId};

use super::session_host::{rig_placing, Rig};

async fn take_on(rig: &Rig, id: &SessionId, lifecycle: &str, key: &str) {
    let mut store = rig.fleet.store().lock().await;
    store.take_on_lifecycle_instance(id.as_str(), lifecycle, key, &[], "t0").expect("taken on");
}

async fn lanes(rig: &Rig, id: &SessionId) -> Option<JobLanesView> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session.lanes
}

#[tokio::test]
async fn a_job_that_holds_nothing_and_follows_no_workflow_has_no_lanes() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert_eq!(lanes(&rig, &id).await, None);
}

#[tokio::test]
async fn a_job_that_holds_instances_has_a_lane_for_each_and_the_state_each_stands_in() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    take_on(&rig, &id, "ticket", "ARM-41").await;
    take_on(&rig, &id, "pr", "812").await;
    rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Draft)).await;

    let drawn = lanes(&rig, &id).await.expect("lanes");
    assert_eq!(drawn.lanes.iter().map(|lane| lane.title.as_str()).collect::<Vec<_>>(), ["ARM-41", "PR #812"]);
    assert_eq!((drawn.workflow.as_str(), drawn.at, drawn.working), ("", -1, false));
    let pull = drawn.job.instances.iter().find(|one| one.lifecycle == "pr").expect("the pull request");
    assert_eq!((pull.label.as_str(), pull.state.as_str()), ("PR #812", "draft"));
    assert!(drawn.lanes[1].nodes.iter().any(|node| node.here && node.label == "Draft"));
}

#[tokio::test]
async fn a_job_that_ended_has_no_lanes_though_it_held_instances() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    take_on(&rig, &id, "ticket", "ARM-41").await;
    assert!(lanes(&rig, &id).await.is_some());

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("ended");
    assert_eq!(lanes(&rig, &id).await, None);
}

#[tokio::test]
async fn a_move_of_what_the_job_holds_publishes_its_record_with_the_lanes_as_they_now_stand() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    take_on(&rig, &id, "pr", "812").await;
    let mut watching = rig.fleet.events().subscribe();

    rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Draft)).await;
    rig.fleet.job_lifecycles_published(id.as_str()).await;
    rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Approved)).await;
    rig.fleet.job_lifecycles_published(id.as_str()).await;

    let mut heard: Vec<String> = Vec::new();
    while let Ok(next) = tokio::time::timeout(std::time::Duration::from_millis(300), watching.next()).await {
        match next {
            Some(Next::Send(delivered)) => {
                if let Event::SessionChanged(record) = delivered.event {
                    if record.id == id {
                        let stood = record.lanes.and_then(|one| one.job.instances.first().map(|one| one.state.clone()));
                        heard.push(stood.unwrap_or_default());
                    }
                }
            }
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    assert_eq!(heard.first().map(String::as_str), Some("draft"), "{heard:?}");
    assert_eq!(heard.last().map(String::as_str), Some("approved"), "{heard:?}");
}

#[tokio::test]
async fn a_move_in_a_started_job_publishes_the_record_of_the_job_that_started_it_with_the_ticket_it_handed_over_as_that_job_reads_it() {
    let rig = rig_placing(None);
    let parent = rig.start().await;
    let child = rig.start().await;
    super::lifecycle_keeping::holds(&rig, &child, "started_by", parent.as_str(), store::AttachmentState::Spent, &[]).await;
    {
        let mut store = rig.fleet.store().lock().await;
        store.take_on_lifecycle_instance(parent.as_str(), "ticket", "ARM-52", &[], "t0").expect("taken on");
        store.take_on_lifecycle_instance(parent.as_str(), "job", child.as_str(), &[("ticket", "ARM-52")], "t0").expect("taken on");
        // Handing it away ended it on the parent, and the started Job holds it from then on.
        store.end_lifecycle_instances(parent.as_str(), &[("ticket", "ARM-52")], "t1").expect("ended");
        store.take_on_lifecycle_instance(child.as_str(), "ticket", "ARM-52", &[], "t1").expect("taken on");
    }
    let mut watching = rig.fleet.events().subscribe();

    rig.fleet.lifecycle_seen(child.as_str(), "ticket", "ARM-52", Observed::Tracker("Awaiting QA")).await;
    rig.fleet.job_lifecycles_published(child.as_str()).await;

    let mut carried = Vec::new();
    while let Ok(next) = tokio::time::timeout(std::time::Duration::from_millis(300), watching.next()).await {
        match next {
            Some(Next::Send(delivered)) => {
                if let Event::SessionChanged(record) = delivered.event {
                    if record.id == parent {
                        let lanes = record.lanes.expect("the parent holds instances");
                        assert_eq!(lanes.job.instances.iter().map(|one| one.lifecycle.as_str()).collect::<Vec<_>>(), ["job"], "the handed ticket has no lane of its own");
                        carried.push(lanes.job.instances[0].ticket.clone().map(|ticket| ticket.state));
                    }
                }
            }
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    assert_eq!(carried, [Some("Awaiting QA".to_string())]);
}
