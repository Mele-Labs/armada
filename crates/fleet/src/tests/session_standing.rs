//! What a Session is doing, read from what Fleet observed and never from a tool the agent called: a
//! finished turn is a reply, `done_since` is a done, and a running turn nothing is heard from is stuck.
//! `docs/concepts/session.md`, *What a Session is doing*.

use std::time::Duration;

use std::sync::Arc;

use adapter_traits::DroneEvent;
use api::{HostedSessions, Sessions};
use ipc::{CloseSession, Event, SessionFact, SessionId, SessionRecord, SessionReport, SessionStanding, WaitingSource};
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

fn kinds(seen: &Seen) -> Vec<(String, WaitingSource, String)> {
    derived(seen).into_iter().map(|one| (one.id, one.source, one.text)).collect()
}

/// **Nothing here calls `waiting_for` or `all_done`**: a turn that ended on a message is a reply by
/// itself. A reply asks nothing of the person, so it is a standing and raises no waiting item.
#[test]
fn a_finished_turn_that_ended_on_a_message_is_a_reply_with_no_waiting_item() {
    let seen = between();
    assert_eq!(standing(&seen, STUCK_AFTER), SessionStanding::Replied);
    assert!(derived(&seen).is_empty(), "{:?}", kinds(&seen));
    // A turn that said nothing to the person left nothing to answer.
    let silent = Seen { turn_end: Some((ENDED, "  ")), ..between() };
    assert_eq!(standing(&silent, STUCK_AFTER), SessionStanding::Idle);
    assert!(derived(&silent).is_empty());
}

/// The agent saying nothing is left is `done_since`, and it outranks the reply the same turn left.
#[test]
fn done_is_read_from_done_since_and_outranks_a_reply() {
    let seen = Seen { done: Some(LATER), ..between() };
    assert_eq!(standing(&seen, STUCK_AFTER), SessionStanding::Done);
    assert_eq!(kinds(&seen), [(format!("done:{LATER}"), WaitingSource::Done, "Done".to_string())]);
}

/// A reply goes when he writes after it. A message he wrote before the turn ended is what it answered.
#[test]
fn a_reply_goes_when_the_person_writes_after_it() {
    let before = Seen { spoke: Some("2026-10-14T09:59:00.000Z"), ..between() };
    assert_eq!(standing(&before, STUCK_AFTER), SessionStanding::Replied);
    let after = Seen { spoke: Some(LATER), ..between() };
    assert_eq!(standing(&after, STUCK_AFTER), SessionStanding::Idle);
    assert!(derived(&after).is_empty());
}

/// A new turn running is work: the reply and the done the turn before left are not standing.
#[test]
fn a_new_turn_ends_the_reply_and_the_done() {
    let running = Seen { running: Some(Duration::from_secs(1)), done: Some(LATER), ..between() };
    assert_eq!(standing(&running, STUCK_AFTER), SessionStanding::Working);
    assert!(derived(&running).is_empty());
}

/// A dismissed done never comes back, and the Session stands as idle.
#[test]
fn a_dismissed_done_stays_gone() {
    let gone = [format!("done:{LATER}")];
    let done = Seen { done: Some(LATER), dismissed: &gone, turn_end: None, ..between() };
    assert_eq!(standing(&done, STUCK_AFTER), SessionStanding::Idle);
    assert!(derived(&done).is_empty());
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

/// A Helm Session's replies are the dock's: they raise no call.
#[test]
fn a_helm_session_never_replies() {
    let seen = Seen { helm: true, done: Some(LATER), ..between() };
    assert_eq!(standing(&seen, STUCK_AFTER), SessionStanding::Idle);
    assert!(derived(&seen).is_empty());
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

fn replied(record: &SessionRecord) -> bool {
    record.standing == Some(SessionStanding::Replied)
}

/// A hosted turn that ends on a message is a reply on the row Fleet serves, with the agent having called
/// nothing, and the row lists nothing for the person to do.
#[tokio::test]
async fn a_hosted_turn_that_ends_on_a_message_replies_with_no_waiting_item() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.\nThe backoff stops there.");
    eventually(|| async { replied(&record(&rig, &id).await) }).await;

    let row = record(&rig, &id).await;
    assert_eq!(row.standing, Some(SessionStanding::Replied));
    assert!(row.waiting_for.is_empty(), "{:?}", row.waiting_for);
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
    eventually(|| async { replied(&record(&rig, &id).await) }).await;

    rig.send(&id, "and log it").await;
    let row = record(&rig, &id).await;
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
    eventually(|| async { replied(&record(&rig, &id).await) }).await;

    rig.stand_in.init(0);
    eventually(|| async { record(&rig, &id).await.standing == Some(SessionStanding::Working) }).await;

    // That turn ends on tools alone, with nothing said: the earlier reply does not come back.
    rig.stand_in.says(0, vec![DroneEvent::Ended { turns: 1, cost_micros: 5_000, refusals: 0 }]);
    eventually(|| async { record(&rig, &id).await.standing == Some(SessionStanding::Idle) }).await;
}

/// An ended Session has no standing and owes nothing, whatever its last turn said.
#[tokio::test]
async fn an_ended_session_has_no_reply_and_no_standing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.");
    eventually(|| async { replied(&record(&rig, &id).await) }).await;

    rig.fleet.close_session(CloseSession { session_id: id.clone() }).await.expect("closed");
    assert_eq!(record(&rig, &id).await.standing, None);
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
    assert!(done.waiting_for.is_empty(), "{:?}", done.waiting_for);

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

/// **A terminal session's permission prompt raises a call and clears when the session carries on.** The
/// prompt is the harness's own fact, told by the mod, and the agent calls nothing. While it is up the
/// turn waits on the person, so it is never stuck; nothing in Bridge can answer it, so its one act opens
/// the Session.
#[tokio::test]
async fn a_terminal_permission_prompt_raises_a_call_and_clears_when_the_session_carries_on() {
    let home = TempDir::new();
    let fleet = a_terminal_fleet(&home);
    fleet.hosts().plant_stuck_after(Duration::from_millis(80));
    fleet.report_session(report(started())).await.unwrap();
    fleet.report_session(report(SessionFact::TurnStarted)).await.unwrap();
    let asked = SessionFact::PermissionAsked { tool: "Bash".into(), detail: "cargo build".into() };
    let held = fleet.report_session(report(asked.clone())).await.unwrap();
    let [call] = held.waiting_for.as_slice() else { panic!("one call: {:?}", held.waiting_for) };
    assert_eq!(call.source, WaitingSource::Permission);
    assert_eq!(call.text, "Bash cargo build");
    assert!(call.options.is_empty(), "Bridge cannot answer a terminal's prompt");
    assert_eq!(call.act.as_ref().map(|act| (act.kind, act.target.as_str())), Some((ipc::WaitingActKind::Message, "t1")));

    tokio::time::sleep(Duration::from_millis(160)).await;
    fleet.standing_checked().await;
    let waiting = Arc::clone(&fleet).get_session(session("t1")).await.unwrap().session;
    assert_eq!(waiting.standing, Some(SessionStanding::Working), "waiting on the person is not stuck");

    let settled = fleet.report_session(report(SessionFact::PermissionSettled)).await.unwrap();
    assert!(settled.waiting_for.is_empty(), "{:?}", settled.waiting_for);

    // The turn ending, or the next one beginning, takes a prompt the mod never settled with it.
    fleet.report_session(report(asked.clone())).await.unwrap();
    let over = fleet.report_session(report(SessionFact::TurnCompleted { said: None })).await.unwrap();
    assert!(over.waiting_for.is_empty(), "{:?}", over.waiting_for);
    fleet.report_session(report(asked)).await.unwrap();
    let again = fleet.report_session(report(SessionFact::TurnStarted)).await.unwrap();
    assert!(again.waiting_for.is_empty(), "{:?}", again.waiting_for);
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
