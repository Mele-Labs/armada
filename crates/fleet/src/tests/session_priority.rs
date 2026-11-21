//! One Job at most is the person's priority: `prioritise_session` moves the mark to a Job, clears it
//! from the Job that holds it, and refuses an ended Job. Both rows it moves between are published.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions};
use ipc::{Event, PrioritiseSession, SessionFact, SessionId, SessionRecord, SessionReport};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fitted_with;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn report(id: &str, fact: SessionFact) -> SessionReport {
    SessionReport { harness: "a_harness".into(), session_id: SessionId::carried(id), fact }
}

fn fleet_on(home: &TempDir) -> Arc<Fixture> {
    Arc::new(Fleet::assembled(fitted_with(home, FakeWorkProduct::changed(&[]), FakeHarness::that_listens())))
}

async fn started(fleet: &Fixture, id: &str) {
    let fact = SessionFact::Started {
        cwd: "/nowhere".into(),
        title: Some(format!("job {id}")),
        origin: ipc::SessionOrigin::Terminal,
        mod_version: None,
    };
    fleet.report_session(report(id, fact)).await.unwrap();
}

async fn two_jobs(home: &TempDir) -> Arc<Fixture> {
    let fleet = fleet_on(home);
    started(&fleet, "a").await;
    started(&fleet, "b").await;
    fleet
}

fn prioritise(id: &str) -> PrioritiseSession {
    PrioritiseSession { session_id: SessionId::carried(id) }
}

async fn row(fleet: &Fixture, id: &str) -> SessionRecord {
    let all = fleet.list_sessions(None, None, None).await.unwrap().sessions;
    all.into_iter().find(|one| one.id.as_str() == id).expect("the job is listed")
}

async fn marked(fleet: &Fixture) -> (bool, bool) {
    (row(fleet, "a").await.prioritised, row(fleet, "b").await.prioritised)
}

#[tokio::test]
async fn prioritising_moves_the_one_mark_and_prioritising_it_again_clears_it() {
    let home = TempDir::new();
    let fleet = two_jobs(&home).await;

    let answered = fleet.prioritise_session(prioritise("a")).await.unwrap();
    assert!(answered.prioritised, "the answer carries the mark");
    assert_eq!(marked(&fleet).await, (true, false));
    assert_eq!(fleet.prioritised_session_id().await.as_deref(), Some("a"));

    fleet.prioritise_session(prioritise("b")).await.unwrap();
    assert_eq!(marked(&fleet).await, (false, true), "the mark moved off a");
    assert_eq!(fleet.prioritised_session_id().await.as_deref(), Some("b"));

    let cleared = fleet.prioritise_session(prioritise("b")).await.unwrap();
    assert!(!cleared.prioritised);
    assert_eq!(marked(&fleet).await, (false, false));
    assert_eq!(fleet.prioritised_session_id().await, None);
}

#[tokio::test]
async fn moving_the_mark_publishes_both_rows() {
    let home = TempDir::new();
    let fleet = two_jobs(&home).await;
    fleet.prioritise_session(prioritise("a")).await.unwrap();

    let mut watching = fleet.events().subscribe();
    fleet.prioritise_session(prioritise("b")).await.unwrap();
    let mut told = tokio::time::timeout(Duration::from_secs(5), async {
        let mut told = Vec::new();
        while told.len() < 2 {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::SessionChanged(record) = delivered.event {
                        told.push((record.id.as_str().to_string(), record.prioritised));
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
        told
    })
    .await
    .expect("two session.changed arrive");
    told.sort();
    assert_eq!(told, vec![("a".to_string(), false), ("b".to_string(), true)]);
}

#[tokio::test]
async fn an_ended_job_is_refused_and_a_prioritised_job_that_ends_is_no_longer_prioritised() {
    let home = TempDir::new();
    let fleet = two_jobs(&home).await;
    fleet.report_session(report("b", SessionFact::Ended { reason: "closed".into() })).await.unwrap();

    let refused = fleet.prioritise_session(prioritise("b")).await.unwrap_err();
    assert_eq!(refused.status(), 409);
    assert_eq!(refused.error().code, "fleet.session_not_prioritisable");
    assert_eq!(refused.error().message, "This Job has ended, so it cannot be prioritised.");

    fleet.prioritise_session(prioritise("a")).await.unwrap();
    fleet.report_session(report("a", SessionFact::Ended { reason: "closed".into() })).await.unwrap();
    assert!(!row(&fleet, "a").await.prioritised, "an ended Job reads as not prioritised");
    assert_eq!(fleet.prioritised_session_id().await, None);
}

#[tokio::test]
async fn the_mark_outlasts_a_fleet_restart() {
    let home = TempDir::new();
    let fleet = two_jobs(&home).await;
    fleet.prioritise_session(prioritise("b")).await.unwrap();
    drop(fleet);

    let again = fleet_on(&home);
    assert_eq!(marked(&again).await, (false, true));
    assert_eq!(again.prioritised_session_id().await.as_deref(), Some("b"));
}
