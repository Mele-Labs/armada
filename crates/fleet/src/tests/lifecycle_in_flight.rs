//! The in-flight list a Job's record carries: what End job lists, read by Fleet so a client does not
//! work it out. **The record is the one place a client reads it**, and a move of what the Job holds
//! publishes the record again. No forge, tracker or process is reached.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next};
use core_model::{ForgeFact, Observed, ReadingState};
use ipc::{CloseSession, Event, InFlight, SessionId};

use super::session_host::{rig_placing, Rig};

async fn take_on(rig: &Rig, id: &SessionId, lifecycle: &str, key: &str) {
    let mut store = rig.fleet.store().lock().await;
    store.take_on_lifecycle_instance(id.as_str(), lifecycle, key, &[], "t0").expect("taken on");
}

async fn in_flight(rig: &Rig, id: &SessionId) -> Vec<InFlight> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session.in_flight
}

fn flying(lifecycle: &str, key: &str, label: &str, state: &str) -> InFlight {
    InFlight { lifecycle: lifecycle.into(), key: key.into(), label: label.into(), state: state.into() }
}

#[tokio::test]
async fn the_record_lists_what_is_neither_ended_nor_in_an_end_state_and_says_where_each_stands() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    take_on(&rig, &id, "ticket", "ARM-41").await;
    take_on(&rig, &id, "pr", "812").await;
    take_on(&rig, &id, "ticket", "ARM-43").await;
    take_on(&rig, &id, "ticket", "ARM-42").await;
    take_on(&rig, &id, "deploy", "Deploy").await;
    rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Merged)).await;
    rig.fleet.lifecycle_seen(id.as_str(), "ticket", "ARM-42", Observed::Tracker("Awaiting QA")).await;
    rig.fleet.lifecycle_seen(id.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Running)).await;
    // Handing it away ended the ticket where it stood.
    rig.fleet.store().lock().await.end_lifecycle_instances(id.as_str(), &[("ticket", "ARM-41")], "t1").expect("ended");

    assert_eq!(
        in_flight(&rig, &id).await,
        [
            flying("ticket", "ARM-43", "ARM-43", ""),
            flying("ticket", "ARM-42", "ARM-42", "Awaiting QA"),
            flying("deploy", "Deploy", "Deploy", "Rolling out"),
        ],
        "ARM-41 was ended, #812 merged; ARM-43 is unread, ARM-42 unmapped and the deploy in a state"
    );
}

#[tokio::test]
async fn a_job_that_holds_nothing_and_one_that_has_ended_list_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert_eq!(in_flight(&rig, &id).await, Vec::<InFlight>::new());

    take_on(&rig, &id, "deploy", "Deploy").await;
    rig.fleet.lifecycle_seen(id.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Running)).await;
    assert_eq!(in_flight(&rig, &id).await.len(), 1);

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("ended");
    assert_eq!(in_flight(&rig, &id).await, Vec::<InFlight>::new(), "a Job that ended holds nothing in flight");
}

#[tokio::test]
async fn a_move_of_what_the_job_holds_publishes_its_record_with_the_list_as_it_now_stands() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    take_on(&rig, &id, "deploy", "Deploy").await;
    let mut watching = rig.fleet.events().subscribe();

    rig.fleet.lifecycle_seen(id.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Running)).await;
    rig.fleet.job_lifecycles_published(id.as_str()).await;
    rig.fleet.lifecycle_seen(id.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Passed)).await;
    rig.fleet.job_lifecycles_published(id.as_str()).await;

    let mut lists = Vec::new();
    while let Ok(next) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        match next {
            Some(Next::Send(delivered)) => {
                if let Event::SessionChanged(record) = delivered.event {
                    if record.id == id {
                        lists.push(record.in_flight);
                    }
                }
            }
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    assert_eq!(lists.first().map(Vec::as_slice), Some([flying("deploy", "Deploy", "Deploy", "Rolling out")].as_slice()), "{lists:?}");
    assert_eq!(lists.last().map(Vec::len), Some(0), "the deploy passed, so the last record lists nothing: {lists:?}");
}
