//! A session the plan's limit stopped is held, and woken with `Continue` when the limit resets.
//! `docs/concepts/session.md`, *A session that hit the plan's limit*.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use api::{HostedSessions, Sessions};
use core_model::Timestamp;
use ipc::{SessionFact, SessionId, SessionRecord, SessionReport, TakeHeld};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::tests::daemon::fitted_with;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// 05:00 on the clock's day.
const RESETS: &str = "2026-09-02T05:00:00.000Z";

/// A clock that ticks a second per reading, and jumps when a test says so.
struct Held {
    ticks: AtomicU64,
    pushed: AtomicU64,
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let at = self.ticks.fetch_add(1, Ordering::SeqCst) + self.pushed.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-09-02T{:02}:{:02}:{:02}.000Z",
            (at / 3_600) % 24,
            (at / 60) % 60,
            at % 60
        ))
    }
}

fn report(fact: SessionFact) -> SessionReport {
    SessionReport {
        harness: "a_harness".into(),
        session_id: SessionId::carried("s1"),
        fact,
    }
}

async fn a_terminal_session(home: &TempDir) -> (Arc<Fixture>, Arc<Held>) {
    let clock = Arc::new(Held {
        ticks: AtomicU64::new(0),
        pushed: AtomicU64::new(0),
    });
    let mut fittings = fitted_with(home, FakeWorkProduct::changed(&[]), FakeHarness::that_listens());
    fittings.clock = Arc::clone(&clock) as Arc<dyn Clock>;
    let fleet = Arc::new(Fleet::assembled(fittings));
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

async fn limited(fleet: &Fixture) -> SessionRecord {
    fleet
        .report_session(report(SessionFact::Limited {
            resets_at: ipc::Instant::carried(RESETS),
        }))
        .await
        .unwrap()
}

/// What the terminal's mod would be handed if it asked now. **Asking is also what makes the session
/// listening**, which a message to a terminal needs.
async fn handed_to_the_mod(fleet: &Fixture) -> Vec<String> {
    fleet
        .take_held_messages(TakeHeld {
            session_id: "s1".into(),
            wait_ms: None,
        })
        .await
        .unwrap()
        .messages
}

#[tokio::test]
async fn a_held_session_says_when_and_is_woken_once_after_the_reset() {
    let home = TempDir::new();
    let (fleet, clock) = a_terminal_session(&home).await;
    handed_to_the_mod(&fleet).await;

    let record = limited(&fleet).await;
    assert_eq!(record.held_until.as_ref().map(|at| at.as_str()), Some(RESETS));

    // Before the reset, and in the minute after it, nobody is woken.
    fleet.limits_lifted().await;
    clock.pushed.fetch_add(5 * 3_600 + 30, Ordering::SeqCst);
    fleet.limits_lifted().await;
    assert!(handed_to_the_mod(&fleet).await.is_empty());

    clock.pushed.fetch_add(120, Ordering::SeqCst);
    fleet.limits_lifted().await;
    assert_eq!(handed_to_the_mod(&fleet).await, vec!["Fleet: Continue".to_string()]);

    fleet.limits_lifted().await;
    assert!(handed_to_the_mod(&fleet).await.is_empty(), "woken once");
    let after = fleet.list_sessions(None, None, None).await.unwrap();
    assert_eq!(after.sessions[0].held_until, None, "the hold is let go with the wake");
}

#[tokio::test]
async fn a_session_that_took_a_turn_meanwhile_is_not_woken() {
    let home = TempDir::new();
    let (fleet, clock) = a_terminal_session(&home).await;
    handed_to_the_mod(&fleet).await;
    limited(&fleet).await;

    let record = fleet.report_session(report(SessionFact::TurnCompleted)).await.unwrap();
    assert_eq!(record.held_until, None);

    clock.pushed.fetch_add(6 * 3_600, Ordering::SeqCst);
    fleet.limits_lifted().await;
    assert!(handed_to_the_mod(&fleet).await.is_empty());
}

#[tokio::test]
async fn a_terminal_that_is_not_listening_is_tried_again_and_an_ended_session_is_let_go() {
    let home = TempDir::new();
    let (fleet, clock) = a_terminal_session(&home).await;
    limited(&fleet).await;
    clock.pushed.fetch_add(5 * 3_600 + 300, Ordering::SeqCst);

    // Nobody has asked, so nothing can be handed over yet, and the hold stands.
    fleet.limits_lifted().await;
    assert!(fleet.store().lock().await.session_limit("s1").unwrap().is_some());

    // Its mod asks, and the next tick wakes it.
    handed_to_the_mod(&fleet).await;
    fleet.limits_lifted().await;
    assert_eq!(handed_to_the_mod(&fleet).await, vec!["Fleet: Continue".to_string()]);
}

#[tokio::test]
async fn a_terminal_that_never_listens_is_given_up_on_long_after_the_reset() {
    let home = TempDir::new();
    let (fleet, clock) = a_terminal_session(&home).await;
    limited(&fleet).await;
    clock.pushed.fetch_add(6 * 3_600, Ordering::SeqCst);
    fleet.limits_lifted().await;
    assert!(fleet.store().lock().await.session_limit("s1").unwrap().is_none());
}

#[tokio::test]
async fn a_hold_survives_what_the_store_keeps_and_ends_with_the_session() {
    let home = TempDir::new();
    let (fleet, _clock) = a_terminal_session(&home).await;
    limited(&fleet).await;
    assert_eq!(
        fleet.store().lock().await.sessions_limited().unwrap(),
        vec![("s1".to_string(), RESETS.to_string())]
    );
    fleet
        .report_session(report(SessionFact::Ended { reason: "other".into() }))
        .await
        .unwrap();
    assert!(fleet.store().lock().await.sessions_limited().unwrap().is_empty());
}
