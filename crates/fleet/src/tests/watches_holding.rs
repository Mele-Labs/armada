//! A Job a watch holds open: its row names the watch only while the watch is all that keeps the Job
//! open, and is published again when a watch starts or ends.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions, Subscription, Watches};
use ipc::{
    AttachmentReport, Event, SessionFact, SessionId, SessionRecord, SessionReport, SetWaitingFor, StartWatch, StopWatch, WaitingInput,
    WatchStarts,
};

use super::agent_starts::{rig, Rig};
use super::session_host::eventually;

fn watch(job: &SessionId, name: &str) -> StartWatch {
    StartWatch {
        session_id: Some(job.clone()),
        name: name.to_string(),
        starts: WatchStarts::Command {
            command: vec!["scripts/deploy-state".to_string()],
            every: "60s".to_string(),
            gives_up_after: None,
        },
        follows: None,
        set_aside: Vec::new(),
    }
}

async fn held_open_by(rig: &Rig, job: &SessionId) -> Option<String> {
    Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("read").session.held_open_by
}

/// An agent whose turn has ended and which owes nothing.
async fn an_agent_between_turns(rig: &Rig) -> SessionId {
    let job = rig.an_agent().await;
    rig.stand_in.finishes(0, "Waiting on the deploy.");
    let between = |record: SessionRecord| matches!(record.standing, Some(ipc::SessionStanding::Idle | ipc::SessionStanding::Replied));
    eventually(|| async { between(rig.record(&job).await) }).await;
    job
}

#[tokio::test]
async fn a_job_between_turns_is_held_open_by_the_watch_that_started_first_until_it_ends() {
    let rig = rig(None);
    let job = an_agent_between_turns(&rig).await;
    assert_eq!(held_open_by(&rig, &job).await, None, "no watch, nothing holds it");

    let first = rig.fleet.start_watch(None, watch(&job, "QA deploy")).await.expect("started").id;
    rig.fleet.start_watch(None, watch(&job, "Prod deploy")).await.expect("started");
    assert_eq!(held_open_by(&rig, &job).await.as_deref(), Some("QA deploy"));

    rig.fleet.stop_watch(None, StopWatch { session_id: None, watch: first }).await.expect("stopped");
    assert_eq!(held_open_by(&rig, &job).await.as_deref(), Some("Prod deploy"), "the next live one");
}

#[tokio::test]
async fn a_turn_running_something_owed_or_a_command_left_running_each_say_more_than_the_watch() {
    let rig = rig(None);
    let job = rig.an_agent().await;
    rig.fleet.start_watch(None, watch(&job, "QA deploy")).await.expect("started");
    assert_eq!(held_open_by(&rig, &job).await, None, "its turn is still running");

    rig.stand_in.finishes(0, "Waiting on the deploy.");
    eventually(|| async { rig.record(&job).await.held_open_by.is_some() }).await;

    let background = SessionFact::Attached {
        attachment: AttachmentReport { kind: "background".into(), target: "task-1".into(), detail: BTreeMap::new() },
    };
    let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: job.clone(), fact: background };
    rig.fleet.report_session(report).await.expect("attached");
    assert_eq!(held_open_by(&rig, &job).await, None, "a command of its own is running");
}

#[tokio::test]
async fn something_owed_to_the_person_is_not_hidden_behind_the_watch() {
    let rig = rig(None);
    let job = an_agent_between_turns(&rig).await;
    rig.fleet.start_watch(None, watch(&job, "QA deploy")).await.expect("started");
    assert!(held_open_by(&rig, &job).await.is_some());

    let owed = WaitingInput { id: "q1".into(), text: "Run the migration.".into(), act: None, options: Vec::new(), steps: Vec::new() };
    rig.fleet.waiting_for(None, SetWaitingFor { items: vec![owed], session_id: Some(job.clone()) }).await.expect("set");
    assert_eq!(held_open_by(&rig, &job).await, None);
}

/// The `held_open_by` of every row of `job` published until the stream has been quiet a moment.
async fn published(watching: &mut Subscription, job: &SessionId) -> Vec<Option<String>> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        if let Event::SessionChanged(record) = delivered.event {
            if record.id == *job {
                seen.push(record.held_open_by);
            }
        }
    }
    seen
}

#[tokio::test]
async fn starting_and_stopping_a_watch_publish_the_row_again() {
    let rig = rig(None);
    let job = an_agent_between_turns(&rig).await;
    let mut watching = rig.fleet.events().subscribe();
    published(&mut watching, &job).await;

    let id = rig.fleet.start_watch(None, watch(&job, "QA deploy")).await.expect("started").id;
    assert_eq!(published(&mut watching, &job).await, [Some("QA deploy".to_string())], "the start named the watch");
    rig.fleet.stop_watch(None, StopWatch { session_id: None, watch: id }).await.expect("stopped");
    assert_eq!(published(&mut watching, &job).await, [None], "the stop named nothing");
}
