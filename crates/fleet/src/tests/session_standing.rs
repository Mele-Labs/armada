//! What a Session is doing, read from what Fleet observed and never from a tool the agent called: a
//! finished turn is a reply, `done_since` is a done, and a running turn nothing is heard from is stuck.
//! `docs/concepts/session.md`, *What a Session is doing*.

use std::time::Duration;

use std::sync::Arc;

use adapter_traits::DroneEvent;
use api::{HostedSessions, Sessions};
use ipc::{CloseSession, DismissWaiting, Event, SessionFact, SessionId, SessionRecord, SessionReport, SessionStanding, WaitingSource};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::daemon::fitted_with;
use super::session_host::{eventually, rig_placing, Rig};
use super::tmp::TempDir;
use crate::daemon::Fleet;
use crate::session_host::standing::{derived, first_line, standing, Seen, STUCK_AFTER};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const ENDED: &str = "2026-10-14T10:00:00.000Z";
const LATER: &str = "2026-10-14T10:00:05.000Z";
const SAID: &str = "The cap is five.";

fn between() -> Seen<'static> {
    Seen { helm: false, running: None, held: false, turn_end: Some((ENDED, SAID)), spoke: None, done: None, dismissed: &[] }
}

fn kinds(seen: &Seen, others: bool) -> Vec<(String, WaitingSource, String)> {
    derived(seen, others).into_iter().map(|one| (one.id, one.source, one.text)).collect()
}

/// **Nothing here calls `waiting_for` or `all_done`**: a turn that ended on a message is a reply by
/// itself, with the agent's first line as its text and the turn's end as its id.
#[test]
fn a_finished_turn_that_ended_on_a_message_is_a_reply_without_any_tool_call() {
    let seen = between();
    assert_eq!(standing(&seen, STUCK_AFTER), SessionStanding::Replied);
    assert_eq!(kinds(&seen, false), [(format!("replied:{ENDED}"), WaitingSource::Replied, SAID.to_string())]);
    // A turn that said nothing to the person left nothing to answer.
    let silent = Seen { turn_end: Some((ENDED, "  ")), ..between() };
    assert_eq!(standing(&silent, STUCK_AFTER), SessionStanding::Idle);
    assert!(derived(&silent, false).is_empty());
}

/// The agent saying nothing is left is `done_since`, and it outranks the reply the same turn left.
#[test]
fn done_is_read_from_done_since_and_outranks_a_reply() {
    let seen = Seen { done: Some(LATER), ..between() };
    assert_eq!(standing(&seen, STUCK_AFTER), SessionStanding::Done);
    assert_eq!(kinds(&seen, false), [(format!("done:{LATER}"), WaitingSource::Done, "Done".to_string())]);
}

/// A reply goes when he writes after it. A message he wrote before the turn ended is what it answered.
#[test]
fn a_reply_goes_when_the_person_writes_after_it() {
    let before = Seen { spoke: Some("2026-10-14T09:59:00.000Z"), ..between() };
    assert_eq!(standing(&before, STUCK_AFTER), SessionStanding::Replied);
    let after = Seen { spoke: Some(LATER), ..between() };
    assert_eq!(standing(&after, STUCK_AFTER), SessionStanding::Idle);
    assert!(derived(&after, false).is_empty());
}

/// A new turn running is work: the reply and the done the turn before left are not standing.
#[test]
fn a_new_turn_ends_the_reply_and_the_done() {
    let running = Seen { running: Some(Duration::from_secs(1)), done: Some(LATER), ..between() };
    assert_eq!(standing(&running, STUCK_AFTER), SessionStanding::Working);
    assert!(derived(&running, false).is_empty());
}

/// A dismissed reply or done never comes back, and the Session stands as idle.
#[test]
fn a_dismissed_reply_and_a_dismissed_done_stay_gone() {
    let gone = [format!("replied:{ENDED}")];
    let replied = Seen { dismissed: &gone, ..between() };
    assert_eq!(standing(&replied, STUCK_AFTER), SessionStanding::Idle);
    assert!(derived(&replied, false).is_empty());
    let gone = [format!("done:{LATER}")];
    let done = Seen { done: Some(LATER), dismissed: &gone, turn_end: None, ..between() };
    assert_eq!(standing(&done, STUCK_AFTER), SessionStanding::Idle);
    assert!(derived(&done, false).is_empty());
}

/// **Stuck is a running turn nothing was heard from, and nothing else.** Not a Session between turns however
/// long ago it ended, not a turn that waits on his answer, and not one inside the bound.
#[test]
fn stuck_is_only_a_running_turn_that_went_quiet() {
    let quiet = Duration::from_secs(11 * 60);
    let running = |quiet, held| Seen { running: Some(quiet), held, ..between() };
    assert_eq!(standing(&running(quiet, false), STUCK_AFTER), SessionStanding::Stuck);
    assert_eq!(standing(&running(Duration::from_secs(9 * 60), false), STUCK_AFTER), SessionStanding::Working);
    assert_eq!(standing(&running(quiet, true), STUCK_AFTER), SessionStanding::Working, "held on his answer");
    // A turn that ended long ago is a reply, never stuck: the clock does not run between turns.
    assert_eq!(standing(&between(), Duration::from_secs(1)), SessionStanding::Replied);
}

/// Where something else is owed the agent said what it needs there, so the reply is not a second card.
#[test]
fn the_reply_stands_aside_where_something_else_is_owed() {
    assert!(derived(&between(), true).is_empty());
    let seen = Seen { done: Some(LATER), ..between() };
    assert_eq!(derived(&seen, true).len(), 1, "a done is his act to take, whatever else waits");
}

/// A Helm Session's replies are the dock's: they raise no call.
#[test]
fn a_helm_session_never_replies() {
    let seen = Seen { helm: true, done: Some(LATER), ..between() };
    assert_eq!(standing(&seen, STUCK_AFTER), SessionStanding::Idle);
    assert!(derived(&seen, false).is_empty());
}

#[test]
fn a_reply_reads_as_the_first_line_of_what_was_said() {
    assert_eq!(first_line("\n  The cap is five.\nAnd the backoff stops there."), "The cap is five.");
    assert_eq!(first_line(&"x".repeat(300)).chars().count(), 200);
}

fn session(id: &str) -> SessionId {
    SessionId::carried(id)
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

fn replied(record: &SessionRecord) -> Option<&ipc::WaitingItem> {
    record.waiting_for.iter().find(|one| one.source == WaitingSource::Replied)
}

/// A hosted turn that ends on a message is a reply on the row Fleet serves, with the agent having called nothing.
#[tokio::test]
async fn a_hosted_turn_that_ends_on_a_message_replies_with_no_waiting_for_call() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.\nThe backoff stops there.");
    eventually(|| async { replied(&record(&rig, &id).await).is_some() }).await;

    let row = record(&rig, &id).await;
    assert_eq!(row.standing, Some(SessionStanding::Replied));
    let item = replied(&row).expect("a reply");
    assert_eq!(item.text, "The cap is five.");
    assert!(item.id.starts_with("replied:"), "{}", item.id);
}

/// `all_done` is the agent adding detail: Fleet reads the done off `done_since` and serves it as an item
/// and a standing, and the person's message takes it away.
#[tokio::test]
async fn a_done_since_is_served_as_a_done_item_and_standing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let said = rig.fleet.all_done(None, ipc::AllDone { session_id: Some(id.clone()) }).await.unwrap();
    let since = said.done_since.expect("when").as_str().to_string();

    let row = record(&rig, &id).await;
    assert_eq!(row.standing, Some(SessionStanding::Done));
    let item = row.waiting_for.iter().find(|one| one.source == WaitingSource::Done).expect("a done item");
    assert_eq!(item.id, format!("done:{since}"));

    rig.send(&id, "one more thing").await;
    let after = record(&rig, &id).await;
    assert!(after.waiting_for.iter().all(|one| one.source != WaitingSource::Done));
    assert_ne!(after.standing, Some(SessionStanding::Done));
}

/// The person writing to the Session ends the reply, on the same read.
#[tokio::test]
async fn a_reply_goes_when_the_person_messages_the_session() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.");
    eventually(|| async { replied(&record(&rig, &id).await).is_some() }).await;

    rig.send(&id, "and log it").await;
    let row = record(&rig, &id).await;
    assert!(replied(&row).is_none(), "{:?}", row.waiting_for);
    assert_ne!(row.standing, Some(SessionStanding::Replied));
    // And it is forgotten, not only hidden: a clock that ran backwards cannot bring it back.
    assert_eq!(rig.fleet.store().lock().await.turn_end(id.as_str()).unwrap(), None);
}

/// A turn that begins by another route, a wake or the terminal, ends the reply too: nobody sent it from Bridge.
#[tokio::test]
async fn a_reply_goes_when_a_new_turn_starts_by_another_route() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.");
    eventually(|| async { replied(&record(&rig, &id).await).is_some() }).await;

    rig.stand_in.init(0);
    eventually(|| async { record(&rig, &id).await.standing == Some(SessionStanding::Working) }).await;
    assert!(replied(&record(&rig, &id).await).is_none());

    // That turn ends on tools alone, with nothing said: the earlier reply does not come back.
    rig.stand_in.says(0, vec![DroneEvent::Ended { turns: 1, cost_micros: 5_000, refusals: 0 }]);
    eventually(|| async { record(&rig, &id).await.standing == Some(SessionStanding::Idle) }).await;
    assert!(replied(&record(&rig, &id).await).is_none());
}

/// Dismissing the reply is for good: the Session stands as idle and a read does not bring it back.
#[tokio::test]
async fn a_dismissed_reply_does_not_return_on_the_row() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.");
    eventually(|| async { replied(&record(&rig, &id).await).is_some() }).await;
    let item = replied(&record(&rig, &id).await).unwrap().id.clone();

    Arc::clone(&rig.fleet).dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: item }).await.expect("dismissed");
    let row = record(&rig, &id).await;
    assert!(replied(&row).is_none());
    assert_eq!(row.standing, Some(SessionStanding::Idle));
}

/// An ended Session has no standing and owes nothing, whatever its last turn said.
#[tokio::test]
async fn an_ended_session_has_no_reply_and_no_standing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.");
    eventually(|| async { replied(&record(&rig, &id).await).is_some() }).await;

    rig.fleet.close_session(CloseSession { session_id: id.clone() }).await.expect("closed");
    let row = record(&rig, &id).await;
    assert!(replied(&row).is_none());
    assert_eq!(row.standing, None);
}

fn a_terminal_fleet(home: &TempDir) -> Arc<Fixture> {
    let fittings = fitted_with(home, FakeWorkProduct::changed(&[]), FakeHarness::that_listens());
    Arc::new(Fleet::assembled(fittings))
}

fn report(fact: SessionFact) -> SessionReport {
    SessionReport { harness: "a_harness".into(), session_id: session("t1"), fact }
}

fn started() -> SessionFact {
    SessionFact::Started { cwd: "/nowhere".into(), title: Some("cap the retries".into()), origin: ipc::SessionOrigin::Terminal, mod_version: None }
}

/// A terminal session is read from what its mod tells Fleet: a turn that ended on a message replies, the
/// next prompt ends it, and the Session ending leaves no standing.
#[tokio::test]
async fn a_terminal_turn_replies_from_the_mods_report_and_a_prompt_or_the_end_clears_it() {
    let home = TempDir::new();
    let fleet = a_terminal_fleet(&home);
    fleet.report_session(report(started())).await.unwrap();
    fleet.report_session(report(SessionFact::TurnStarted)).await.unwrap();
    let working = fleet.report_session(report(SessionFact::Wrote)).await.unwrap();
    assert_eq!(working.standing, Some(SessionStanding::Working));

    let done = fleet.report_session(report(SessionFact::TurnCompleted { said: Some("The cap is five.".into()) })).await.unwrap();
    assert_eq!(done.standing, Some(SessionStanding::Replied));
    assert_eq!(replied(&done).map(|one| one.text.as_str()), Some("The cap is five."));

    let again = fleet.report_session(report(SessionFact::TurnStarted)).await.unwrap();
    assert_eq!(again.standing, Some(SessionStanding::Working));
    assert!(again.waiting_for.is_empty());

    fleet.report_session(report(SessionFact::TurnCompleted { said: Some("Done with it.".into()) })).await.unwrap();
    let ended = fleet.report_session(report(SessionFact::Ended { reason: "exit".into() })).await.unwrap();
    assert_eq!(ended.standing, None);
    assert!(ended.waiting_for.is_empty());
}

/// **Stuck reaches Bridge by itself.** The Session goes stuck by time passing, which no event says, so the
/// tick publishes it; heard from again, the next tick publishes that.
#[tokio::test]
async fn a_running_turn_that_went_quiet_is_published_stuck_and_again_when_heard_from() {
    let home = TempDir::new();
    let fleet = a_terminal_fleet(&home);
    fleet.hosts().plant_stuck_after(Duration::from_millis(80));
    fleet.report_session(report(started())).await.unwrap();
    fleet.report_session(report(SessionFact::TurnStarted)).await.unwrap();
    let mut live = fleet.events().subscribe();

    tokio::time::sleep(Duration::from_millis(160)).await;
    fleet.standing_checked().await;
    assert_eq!(standing_published(&mut live).await, Some(SessionStanding::Stuck));
    assert_eq!(Arc::clone(&fleet).get_session(session("t1")).await.unwrap().session.standing, Some(SessionStanding::Stuck));

    fleet.report_session(report(SessionFact::Wrote)).await.unwrap();
    fleet.standing_checked().await;
    assert_eq!(standing_published(&mut live).await, Some(SessionStanding::Working));
}

/// The newest standing a published row carried, drained from what is waiting.
async fn standing_published(live: &mut api::Subscription) -> Option<SessionStanding> {
    let mut latest = None;
    while let Ok(Some(api::Next::Send(delivered))) = tokio::time::timeout(Duration::from_millis(100), live.next()).await {
        if let Event::SessionChanged(row) = delivered.event {
            latest = row.standing;
        }
    }
    latest
}
