//! A snoozed Job is kept quiet until its time, then let back with `session.changed`.
//! `docs/concepts/session/snoozing-a-job.md`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions};
use core_model::Timestamp;
use ipc::{Event, SessionFact, SessionId, SessionRecord, SessionReport, SnoozeSession};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::tests::daemon::fitted_with;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// 03:00 on the clock's day.
const UNTIL: &str = "2026-09-02T03:00:00.000Z";

/// A clock that ticks a second per reading, and jumps when a test says so.
struct Moved {
    ticks: AtomicU64,
    pushed: AtomicU64,
}

impl Clock for Moved {
    fn now(&self) -> Timestamp {
        let at = self.ticks.fetch_add(1, Ordering::SeqCst) + self.pushed.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!("2026-09-02T{:02}:{:02}:{:02}.000Z", (at / 3_600) % 24, (at / 60) % 60, at % 60))
    }
}

fn report(fact: SessionFact) -> SessionReport {
    SessionReport { harness: "a_harness".into(), session_id: SessionId::carried("s1"), fact }
}

fn fleet_on(home: &TempDir, clock: &Arc<Moved>) -> Arc<Fixture> {
    let mut fittings = fitted_with(home, FakeWorkProduct::changed(&[]), FakeHarness::that_listens());
    fittings.clock = Arc::clone(clock) as Arc<dyn Clock>;
    Arc::new(Fleet::assembled(fittings))
}

async fn a_job(home: &TempDir) -> (Arc<Fixture>, Arc<Moved>) {
    let clock = Arc::new(Moved { ticks: AtomicU64::new(0), pushed: AtomicU64::new(0) });
    let fleet = fleet_on(home, &clock);
    fleet
        .report_session(report(SessionFact::Started {
            cwd: "/nowhere".into(),
            title: Some("fix the ledger".into()),
            origin: ipc::SessionOrigin::Terminal,
            mod_version: None,
        }))
        .await
        .unwrap();
    (fleet, clock)
}

fn snooze(until: Option<&str>) -> SnoozeSession {
    SnoozeSession { session_id: SessionId::carried("s1"), until: until.map(ipc::Instant::carried) }
}

async fn the_job(fleet: &Fixture) -> SessionRecord {
    fleet.list_sessions(None, None, None).await.unwrap().sessions.remove(0)
}

#[tokio::test]
async fn a_snoozed_job_says_until_when_and_is_let_back_once_the_time_comes() {
    let home = TempDir::new();
    let (fleet, clock) = a_job(&home).await;

    let record = fleet.snooze_session(snooze(Some(UNTIL))).await.unwrap();
    assert_eq!(record.snoozed_until.as_ref().map(|at| at.as_str()), Some(UNTIL));

    // Before the time, the tick leaves it snoozed.
    fleet.snoozes_ended().await;
    clock.pushed.fetch_add(2 * 3_600, Ordering::SeqCst);
    fleet.snoozes_ended().await;
    assert_eq!(the_job(&fleet).await.snoozed_until.as_ref().map(|at| at.as_str()), Some(UNTIL));

    let mut watching = fleet.events().subscribe();
    clock.pushed.fetch_add(3_600, Ordering::SeqCst);
    fleet.snoozes_ended().await;
    let told = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::SessionChanged(record) = delivered.event {
                        return record;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("a session.changed arrives");
    assert_eq!(told.id.as_str(), "s1");
    assert_eq!(told.snoozed_until, None, "published without the snooze");
    assert_eq!(the_job(&fleet).await.snoozed_until, None);
}

#[tokio::test]
async fn a_wake_lets_the_job_back_now() {
    let home = TempDir::new();
    let (fleet, _clock) = a_job(&home).await;
    fleet.snooze_session(snooze(Some(UNTIL))).await.unwrap();
    let woken = fleet.snooze_session(snooze(None)).await.unwrap();
    assert_eq!(woken.snoozed_until, None);
    assert_eq!(the_job(&fleet).await.snoozed_until, None);
}

#[tokio::test]
async fn a_snooze_outlasts_a_fleet_restart_and_the_job_ending() {
    let home = TempDir::new();
    let (fleet, clock) = a_job(&home).await;
    fleet.snooze_session(snooze(Some(UNTIL))).await.unwrap();
    fleet.report_session(report(SessionFact::Ended { reason: "closed".into() })).await.unwrap();
    assert_eq!(the_job(&fleet).await.snoozed_until.as_ref().map(|at| at.as_str()), Some(UNTIL), "ending keeps it");
    drop(fleet);

    let again = fleet_on(&home, &clock);
    assert_eq!(the_job(&again).await.snoozed_until.as_ref().map(|at| at.as_str()), Some(UNTIL), "a restart keeps it");
}

#[tokio::test]
async fn an_ended_job_is_refused_a_snooze_in_a_plain_sentence() {
    let home = TempDir::new();
    let (fleet, _clock) = a_job(&home).await;
    fleet.report_session(report(SessionFact::Ended { reason: "closed".into() })).await.unwrap();
    let refused = fleet.snooze_session(snooze(Some(UNTIL))).await.unwrap_err();
    assert_eq!(refused.status(), 409);
    assert_eq!(refused.error().code, "fleet.session_not_snoozable");
    assert_eq!(refused.error().message, "This Job has ended, so it cannot be snoozed.");
    assert_eq!(the_job(&fleet).await.snoozed_until, None);
}

#[tokio::test]
async fn a_time_that_is_not_one_is_refused() {
    let home = TempDir::new();
    let (fleet, _clock) = a_job(&home).await;
    let refused = fleet.snooze_session(snooze(Some("tomorrow"))).await.unwrap_err();
    assert_eq!(refused.error().code, "fleet.snooze_unreadable");
}
