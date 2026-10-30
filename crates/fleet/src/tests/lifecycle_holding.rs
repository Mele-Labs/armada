//! A live instance holds its Job open, and ending the Job ends it. **The agent's `all_done` and
//! `finish_up` are refused while the Job holds one no end state has taken**, each named as
//! `Deploy is Rolling out`; the agent refused is told once when the last one is over; ending the
//! Job keeps what it still held as ended, stops its watches and raises `job_lifecycles.changed`.
//! A Job that holds nothing is finished and ended as it always was. **No forge, tracker or process
//! is reached.**

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Lifecycles, Next, Sessions, Watches};
use core_model::{ForgeFact, Observed, ReadingState};
use ipc::{AllDone, CloseSession, Event, FinishUp, SessionId, StartWatch, WatchStarts};

use super::rule_acting::notices_in;
use super::session_host::{rig_placing, Rig};
use crate::lifecycles::ending::IN_FLIGHT as CODE;

async fn take_on(rig: &Rig, id: &SessionId, lifecycle: &str, key: &str) {
    let mut store = rig.fleet.store().lock().await;
    store.take_on_lifecycle_instance(id.as_str(), lifecycle, key, &[], "t0").expect("taken on");
}

/// A deploy rolling out and a pull request approved, the Job holding both.
async fn holding_a_deploy_and_a_pull_request(rig: &Rig, id: &SessionId) {
    take_on(rig, id, "deploy", "Deploy").await;
    take_on(rig, id, "pr", "812").await;
    rig.fleet.lifecycle_seen(id.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Running)).await;
    rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Approved)).await;
}

fn done(id: &SessionId) -> AllDone {
    AllDone { session_id: Some(id.clone()) }
}

fn finish(id: &SessionId) -> FinishUp {
    FinishUp { session_id: Some(id.clone()) }
}

async fn told(rig: &Rig, id: &SessionId) -> Vec<String> {
    notices_in(rig, id).await.into_iter().filter(|(notice, _)| notice.as_deref() == Some("lifecycles_clear")).map(|(_, text)| text).collect()
}

async fn ended_at(rig: &Rig, id: &SessionId, lifecycle: &str, key: &str) -> String {
    let store = rig.fleet.store().lock().await;
    let kept = store.lifecycle_instances_of(id.as_str()).expect("reads");
    kept.into_iter().find(|one| one.lifecycle == lifecycle && one.key == key).expect("held").ended_at
}

#[tokio::test]
async fn all_done_and_finish_up_are_refused_while_an_instance_is_in_flight_and_each_one_is_named() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holding_a_deploy_and_a_pull_request(&rig, &id).await;

    let refused = rig.fleet.all_done(None, done(&id)).await.unwrap_err();
    assert_eq!((refused.status(), refused.error().code.as_str()), (409, CODE));
    let said = refused.error().message.clone();
    assert!(said.starts_with("all_done refused. Still in flight:"), "{said}");
    assert!(said.contains("- Deploy is Rolling out\n"), "{said}");
    assert!(said.contains("- PR #812 is Approved\n"), "{said}");
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session;
    assert_eq!(record.done_since, None, "nothing is stamped");

    let kept = rig.fleet.finish_up(None, finish(&id)).await.unwrap_err();
    assert_eq!((kept.status(), kept.error().code.as_str()), (409, CODE));
    assert!(kept.error().message.starts_with("finish_up refused. Still in flight:"), "{}", kept.error().message);
}

#[tokio::test]
async fn a_status_nobody_could_place_and_an_instance_not_read_yet_hold_the_job_too() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    take_on(&rig, &id, "ticket", "ARM-42").await;
    take_on(&rig, &id, "ticket", "ARM-43").await;
    rig.fleet.lifecycle_seen(id.as_str(), "ticket", "ARM-42", Observed::Tracker("Awaiting QA")).await;

    let said = rig.fleet.all_done(None, done(&id)).await.unwrap_err().error().message.clone();
    assert!(said.contains("- ARM-42 is Awaiting QA\n"), "the word its tracker used: {said}");
    assert!(said.contains("- ARM-43 is not read\n"), "{said}");
}

#[tokio::test]
async fn the_calls_go_through_once_every_instance_is_in_an_end_state() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holding_a_deploy_and_a_pull_request(&rig, &id).await;
    rig.fleet.lifecycle_seen(id.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Passed)).await;
    assert!(rig.fleet.all_done(None, done(&id)).await.is_err(), "the pull request is only approved");

    rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Merged)).await;
    assert!(rig.fleet.finish_up(None, finish(&id)).await.is_ok());
    assert!(rig.fleet.all_done(None, done(&id)).await.expect("nothing is in flight").done_since.is_some());
}

#[tokio::test]
async fn a_job_that_holds_nothing_is_finished_as_it_always_was() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert!(rig.fleet.finish_up(None, finish(&id)).await.is_ok());
    assert!(rig.fleet.all_done(None, done(&id)).await.expect("nothing is held").done_since.is_some());
}

/// A reader hands `fact` to an instance, and what the tap does about the move is done.
async fn seen(rig: &Rig, id: &SessionId, lifecycle: &str, key: &str, fact: Observed<'_>) {
    rig.fleet.lifecycle_seen(id.as_str(), lifecycle, key, fact).await;
    rig.fleet.lifecycles_clear_told(id.as_str()).await;
}

#[tokio::test]
async fn the_agent_that_was_refused_is_told_once_when_the_last_instance_is_over_and_one_never_refused_is_not() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holding_a_deploy_and_a_pull_request(&rig, &id).await;
    rig.fleet.all_done(None, done(&id)).await.unwrap_err();

    seen(&rig, &id, "deploy", "Deploy", Observed::Watch(ReadingState::Passed)).await;
    assert!(told(&rig, &id).await.is_empty(), "the pull request still holds it");
    seen(&rig, &id, "pr", "812", Observed::Forge(ForgeFact::Merged)).await;
    assert_eq!(told(&rig, &id).await, ["PR #812 is Merged. Nothing of this Job is in flight."]);

    // Held again and cleared again, and no call was refused in between.
    take_on(&rig, &id, "deploy", "Staging").await;
    seen(&rig, &id, "deploy", "Staging", Observed::Watch(ReadingState::Running)).await;
    seen(&rig, &id, "deploy", "Staging", Observed::Watch(ReadingState::Passed)).await;
    assert_eq!(told(&rig, &id).await.len(), 1, "told once for the one refusal");

    let other = rig.start().await;
    take_on(&rig, &other, "deploy", "Deploy").await;
    seen(&rig, &other, "deploy", "Deploy", Observed::Watch(ReadingState::Running)).await;
    seen(&rig, &other, "deploy", "Deploy", Observed::Watch(ReadingState::Passed)).await;
    assert!(told(&rig, &other).await.is_empty(), "the agent never asked");
}

#[tokio::test]
async fn the_tap_that_hears_what_a_job_holds_tells_the_agent_when_the_last_instance_is_over() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let _tap = crate::lifecycles::keep_the_lifecycles(Arc::clone(&rig.fleet));
    holding_a_deploy_and_a_pull_request(&rig, &id).await;
    rig.fleet.all_done(None, done(&id)).await.unwrap_err();

    rig.fleet.lifecycle_seen(id.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Passed)).await;
    rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Merged)).await;

    super::session_host::eventually(|| async { !told(&rig, &id).await.is_empty() }).await;
    assert_eq!(told(&rig, &id).await.len(), 1);
}

async fn lifecycle_events(watching: &mut api::Subscription) -> Vec<ipc::JobLifecycles> {
    let mut found = Vec::new();
    while let Ok(next) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        match next {
            Some(Next::Send(delivered)) => {
                if let Event::JobLifecyclesChanged(held) = delivered.event {
                    found.push(held);
                }
            }
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    found
}

#[tokio::test]
async fn ending_the_job_ends_what_is_in_flight_stops_its_watches_and_tells_bridge() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holding_a_deploy_and_a_pull_request(&rig, &id).await;
    rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Merged)).await;
    let watch = StartWatch {
        session_id: Some(id.clone()),
        name: "Deploy".to_string(),
        starts: WatchStarts::Command { command: vec!["/bin/cat".into(), "absent.json".into()], every: "30s".into(), gives_up_after: None },
        follows: None,
        set_aside: Vec::new(),
    };
    let watch_id = rig.fleet.start_watch(None, watch).await.expect("started").id;
    rig.fleet.store().lock().await.follow_lifecycle_instance_with_watch(id.as_str(), "deploy", "Deploy", &watch_id).expect("followed");
    assert_eq!(ended_at(&rig, &id, "deploy", "Deploy").await, "", "live before it ends");
    let mut watching = rig.fleet.events().subscribe();

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("ended");

    assert_ne!(ended_at(&rig, &id, "deploy", "Deploy").await, "", "the deploy was rolling out");
    assert_eq!(ended_at(&rig, &id, "pr", "812").await, "", "the pull request reached an end state of its own");
    let live = rig.fleet.store().lock().await.watches_of(id.as_str()).expect("reads").iter().filter(|one| one.watch.live).count();
    assert_eq!(live, 0, "the watch the deploy followed stopped with it");

    let raised = lifecycle_events(&mut watching).await;
    let last = raised.last().expect("Bridge is told");
    let wire = |key: &str| last.instances.iter().find(|one| one.key == key).expect("held");
    assert!(wire("Deploy").ended_at.is_some() && wire("812").ended_at.is_none(), "{last:?}");
    assert_eq!(wire("Deploy").standing, ipc::InstanceStanding::In { state: "rolling_out".to_string() }, "it stands where it was");

    // What its source says of it now moves nothing: the Job is over.
    rig.fleet.lifecycle_seen(id.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Passed)).await;
    let held = rig.fleet.get_job_lifecycles(None, Some(id.clone())).await.expect("read");
    let deploy = held.instances.iter().find(|one| one.key == "Deploy").expect("held");
    assert_eq!(deploy.standing, ipc::InstanceStanding::In { state: "rolling_out".to_string() });
    assert!(deploy.ended_at.is_some());
}

#[tokio::test]
async fn ending_a_job_that_holds_nothing_raises_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut watching = rig.fleet.events().subscribe();

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id }).await.expect("ended");

    assert!(lifecycle_events(&mut watching).await.is_empty());
}
