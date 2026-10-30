//! A Job a watch or its own pull request holds open: its row names them only while they are all that keeps
//! the Job open, and is published again when a watch starts or ends.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions, Subscription, Watches};
use ipc::{
    AttachmentReport, Event, HeldOpenBy, PullRequestWait, SessionFact, SessionId, SessionRecord, SessionReport, SetWaitingFor, StartWatch, StopWatch, WaitingInput,
    WatchStarts,
};

use super::agent_starts::{rig, Rig};
use super::session_host::eventually;

fn named(name: &str) -> HeldOpenBy {
    HeldOpenBy::Watch { name: name.to_string() }
}

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

async fn held_open_by(rig: &Rig, job: &SessionId) -> Vec<HeldOpenBy> {
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
async fn a_job_between_turns_is_held_open_by_every_live_watch_in_the_order_they_started() {
    let rig = rig(None);
    let job = an_agent_between_turns(&rig).await;
    assert_eq!(held_open_by(&rig, &job).await, [], "no watch, nothing holds it");

    let first = rig.fleet.start_watch(None, watch(&job, "QA deploy")).await.expect("started").id;
    rig.fleet.start_watch(None, watch(&job, "Prod deploy")).await.expect("started");
    assert_eq!(held_open_by(&rig, &job).await, [named("QA deploy"), named("Prod deploy")]);

    rig.fleet.stop_watch(None, StopWatch { session_id: None, watch: first }).await.expect("stopped");
    assert_eq!(held_open_by(&rig, &job).await, [named("Prod deploy")], "the one still live");
}

#[tokio::test]
async fn a_turn_running_something_owed_or_a_command_left_running_each_say_more_than_the_watch() {
    let rig = rig(None);
    let job = rig.an_agent().await;
    rig.fleet.start_watch(None, watch(&job, "QA deploy")).await.expect("started");
    assert_eq!(held_open_by(&rig, &job).await, [], "its turn is still running");

    rig.stand_in.finishes(0, "Waiting on the deploy.");
    eventually(|| async { !rig.record(&job).await.held_open_by.is_empty() }).await;

    let background = SessionFact::Attached {
        attachment: AttachmentReport { kind: "background".into(), target: "task-1".into(), detail: BTreeMap::new() },
    };
    let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: job.clone(), fact: background };
    rig.fleet.report_session(report).await.expect("attached");
    assert_eq!(held_open_by(&rig, &job).await, [], "a command of its own is running");
}

#[tokio::test]
async fn something_owed_to_the_person_is_not_hidden_behind_the_watch() {
    let rig = rig(None);
    let job = an_agent_between_turns(&rig).await;
    rig.fleet.start_watch(None, watch(&job, "QA deploy")).await.expect("started");
    assert!(!held_open_by(&rig, &job).await.is_empty());

    let owed = WaitingInput { id: "q1".into(), text: "Run the migration.".into(), act: None, options: Vec::new(), steps: Vec::new() };
    rig.fleet.waiting_for(None, SetWaitingFor { items: vec![owed], session_id: Some(job.clone()) }).await.expect("set");
    assert_eq!(held_open_by(&rig, &job).await, []);
}

/// The `held_open_by` of every row of `job` published until the stream has been quiet a moment.
async fn published(watching: &mut Subscription, job: &SessionId) -> Vec<Vec<HeldOpenBy>> {
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
    assert_eq!(published(&mut watching, &job).await, [vec![named("QA deploy")]], "the start named the watch");
    rig.fleet.stop_watch(None, StopWatch { session_id: None, watch: id }).await.expect("stopped");
    assert_eq!(published(&mut watching, &job).await, [vec![]], "the stop named nothing");
}

async fn attach_pull_request(rig: &Rig, job: &SessionId, number: &str, detail: &[(&str, &str)]) {
    let detail = detail.iter().map(|(key, said)| (key.to_string(), said.to_string())).collect();
    let attached = SessionFact::Attached { attachment: AttachmentReport { kind: "pull_request".into(), target: number.into(), detail } };
    let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: job.clone(), fact: attached };
    rig.fleet.report_session(report).await.expect("attached");
}

#[tokio::test]
async fn a_job_between_turns_whose_own_pull_request_is_set_to_merge_on_pending_checks_is_held_open_by_it() {
    let rig = rig(None);
    let job = an_agent_between_turns(&rig).await;
    attach_pull_request(&rig, &job, "409", &[("state", "open"), ("auto_merge", "true"), ("checks", "pending")]).await;
    assert_eq!(
        held_open_by(&rig, &job).await,
        [HeldOpenBy::PullRequest { number: 409, on: PullRequestWait::Checks { auto_merge: true } }]
    );
}

#[tokio::test]
async fn a_pull_request_whose_checks_failed_does_not_hold_the_job_open() {
    let rig = rig(None);
    let job = an_agent_between_turns(&rig).await;
    attach_pull_request(&rig, &job, "409", &[("state", "open"), ("auto_merge", "true"), ("queued", "true"), ("checks", "failed")]).await;
    assert_eq!(held_open_by(&rig, &job).await, []);
}
