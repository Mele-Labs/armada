//! The claim, written before the code: **a Job an agent started waits in its starter's Needs you
//! until the person presses Start on it.** The starter lists `Press Start on "<title>"` and its agent's
//! own item for the same Job is left off; a press while the brief is still read is queued and begins
//! with the reading's landing; and the starter's agent is told once that it started.
//! `docs/concepts/session.md`, *A Job an agent started*. Split from `agent_starts.rs` at its size.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{
    BeginSession, CloseSession, DeliveryChoice, Event, LinkedEvent, SessionId, SessionRecord, SessionRow, SetWaitingFor,
    WaitingAct, WaitingActKind, WaitingInput, WaitingItem, WaitingSource,
};
use testkit::FakeJudge;

use crate::tests::agent_starts::{begins, caller, eventually_read, handing, rig, rigged, Rig};
use crate::tests::session_host::eventually;

/// The starter's items for Jobs it started that wait on Start.
fn held_items(record: &SessionRecord) -> Vec<&WaitingItem> {
    record.waiting_for.iter().filter(|one| one.source == WaitingSource::HeldChild).collect()
}

impl Rig {
    /// The agent starts "Check the reader" with `message`, and the Job is answered.
    async fn started_with(&self, message: &str) -> SessionId {
        Arc::clone(&self.fleet).start_another_session(caller(), begins(self, "Check the reader", message)).await.expect("held").session_id
    }

    /// Every record of `id` Fleet publishes from now on.
    fn published(&self, id: &SessionId) -> Arc<Mutex<Vec<SessionRecord>>> {
        let published = Arc::new(Mutex::new(Vec::new()));
        let (seen, id) = (Arc::clone(&published), id.clone());
        self.fleet.events().tap(move |event| {
            if let Event::SessionChanged(record) = event {
                if record.id == id {
                    seen.lock().unwrap().push(record.clone());
                }
            }
        });
        published
    }
}

const FIRST: &str = "Read crates/reader and say why it drops lines.";

#[tokio::test]
async fn the_parent_waits_on_a_held_child_until_it_starts() {
    let rig = rig(None);
    let agent = rig.an_agent().await;
    let held = rig.started_with(FIRST).await;
    eventually_read(&rig, &held).await;

    let parent = rig.record(&agent).await;
    let [item] = &held_items(&parent)[..] else { panic!("one held child waits: {:?}", parent.waiting_for) };
    let title = rig.record(&held).await.title.expect("titled");
    assert_eq!(item.id, format!("start:{}", held.as_str()));
    assert_eq!(item.text, format!("Press Start on \"{title}\""));
    let act = item.act.as_ref().expect("an act");
    assert_eq!((act.kind, act.target.as_str()), (WaitingActKind::Message, held.as_str()), "its press opens the child");

    let published = rig.published(&agent);
    rig.begin(&held, None).await;
    eventually(|| async { published.lock().unwrap().iter().any(|one| held_items(one).is_empty()) }).await;
    assert!(held_items(&rig.record(&agent).await).is_empty(), "Start settles it");
}

#[tokio::test]
async fn the_parent_stops_waiting_on_a_held_child_that_ends() {
    let rig = rig(None);
    let agent = rig.an_agent().await;
    let held = rig.started_with(FIRST).await;
    assert_eq!(held_items(&rig.record(&agent).await).len(), 1);

    let published = rig.published(&agent);
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: held.clone() }).await.expect("closed");
    eventually(|| async { published.lock().unwrap().iter().any(|one| held_items(one).is_empty()) }).await;
    assert!(held_items(&rig.record(&agent).await).is_empty());
}

#[tokio::test]
async fn an_agents_own_item_for_a_held_child_is_left_off_the_parents_list() {
    let rig = rig(None);
    let agent = rig.an_agent().await;
    let held = rig.started_with(FIRST).await;
    let item = |id: &str, text: &str, target: &str| WaitingInput {
        id: id.into(),
        text: text.into(),
        act: Some(WaitingAct { kind: WaitingActKind::Message, target: target.into() }),
        options: Vec::new(),
        steps: Vec::new(),
    };
    let items = vec![
        item("start-it", "Press Start on the reader Job so it runs", held.as_str()),
        item("branch", "Tell me which branch to use", agent.as_str()),
    ];
    rig.fleet.waiting_for(None, SetWaitingFor { items, session_id: Some(agent.clone()) }).await.expect("set");

    let waiting = rig.record(&agent).await.waiting_for;
    let ids: Vec<&str> = waiting.iter().map(|one| one.id.as_str()).collect();
    assert!(!ids.contains(&"start-it"), "Fleet's item already says it: {ids:?}");
    assert!(ids.contains(&"branch"), "an item about anything else stands: {ids:?}");
    assert!(ids.contains(&format!("start:{}", held.as_str()).as_str()), "{ids:?}");
}

#[tokio::test]
async fn a_start_pressed_while_the_brief_is_read_is_queued_then_begins_with_the_readings_landing() {
    let judge = FakeJudge::answering_in_turn(&["workflow: none\ntitle: The agent", "workflow: gate\ntitle: Why the reader drops lines"]);
    let rig = rigged(None, Some(Arc::new(judge.taking(Duration::from_millis(500)))), Some(super::delivery_gate::workflow()));
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));
    let agent = rig.an_agent().await;
    eventually_read(&rig, &agent).await;
    let held = rig.started_with("Read crates/reader and say why it drops lines, then keep it local.").await;
    assert!(rig.record(&held).await.proposal.is_some_and(|one| one.reading), "the brief is still read");

    // Bridge names no landing while the reading is out.
    let press = BeginSession { session_id: held.clone(), delivery: None, message: None };
    let pressed = Arc::clone(&rig.fleet).begin_session(press.clone()).await.expect("queued");
    let start = pressed.hosted.as_ref().and_then(|hosted| hosted.start.as_ref()).expect("still on its card");
    assert!(start.queued, "the press waits on the reading");
    assert_eq!(held_items(&rig.record(&agent).await).len(), 1, "the parent still waits on it");
    let again = Arc::clone(&rig.fleet).begin_session(press).await.expect_err("one press begins it");
    assert!(format!("{again:?}").contains("fleet.session_waiting_unheld"), "{again:?}");
    assert_eq!(rig.stand_in.starts().len(), 1, "nothing runs while it is read");

    eventually(|| async { rig.stand_in.starts().len() == 2 && !rig.sent_to(1).is_empty() }).await;
    rig.rail_has_acted().await;
    let landed = rig.rows(&held).await.into_iter().find_map(|row| match row {
        SessionRow::Delivery { choice: Some(choice), .. } => Some(choice),
        _ => None,
    });
    assert_eq!(landed, Some(DeliveryChoice::KeepLocal), "the landing the card picked once the reading put it on the workflow");
    let sent = rig.sent_to(1);
    let [first] = &sent[..] else { panic!("one turn, not {}: {sent:?}", sent.len()) };
    assert!(first.contains("follows the Gate workflow") && !first.contains("choosing how this work is delivered"), "{first}");
    assert!(held_items(&rig.record(&agent).await).is_empty());
}

#[tokio::test]
async fn the_parent_is_told_once_when_a_child_it_started_starts() {
    let rig = rig(None);
    let agent = rig.a_parent_with_a_plan().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g1")).await.expect("held").session_id;
    eventually_read(&rig, &held).await;

    let begun = rig.begin(&held, None).await;
    assert_eq!(rig.linked_rows(&agent, 1).await, [(held.as_str().to_string(), LinkedEvent::Started)]);
    let said = format!("({}) started. Plan group: g1.", held.as_str());
    eventually(|| async { rig.told(&said) == 1 }).await;

    // Told again, as a press and a queued one landing together would, it is not news.
    rig.fleet.linked_started(&begun).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(rig.linked_rows(&agent, 1).await.len(), 1);
    assert_eq!(rig.told(&said), 1);
}
