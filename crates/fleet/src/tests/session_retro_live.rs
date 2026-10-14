//! A Session's retro drawn in its thread while it is written: the stages in order, the items as
//! they settle, a refused press adding no row, a failed call, a row a restart left, and the
//! throttle. Since 23.78. `docs/concepts/retro.md`, *A Session's retro*.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{Ask, CallProgress};
use api::{HostedSessions, Retros};
use core_model::{LandsIn, Whose};
use ipc::{
    ManifestId, PartialRetroItem, RetroStage, RetroState, SendSessionMessage, SessionId,
    SessionRow, SessionTurn, StartSession,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};
use tokio::time::Instant as Clock;

use super::session_host::{Shared, StandIn};
use super::session_retro::{rig, said, Hosted, TWO_ITEMS};
use crate::daemon::Fleet;
use crate::retro::live::{Live, Progress, GAP, RESTARTED};
use crate::retro::streamed::settled_items;
use crate::tests::tmp::TempDir;

/// Every `retro` row published, in order.
fn collecting(fleet: &Arc<Hosted>) -> Arc<Mutex<Vec<SessionRow>>> {
    let mut watching = fleet.events().subscribe();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let into = Arc::clone(&seen);
    tokio::spawn(async move {
        while let Some(next) = watching.next().await {
            if let api::Next::Send(delivered) = next {
                if let ipc::Event::SessionRow(changed) = delivered.event {
                    if matches!(changed.row, SessionRow::Retro { .. }) {
                        into.lock().unwrap().push(changed.row);
                    }
                }
            }
        }
    });
    seen
}

fn stages(seen: &Arc<Mutex<Vec<SessionRow>>>) -> Vec<RetroStage> {
    let mut out: Vec<RetroStage> = Vec::new();
    for row in seen.lock().unwrap().iter() {
        if let SessionRow::Retro { stage, .. } = row {
            if out.last() != Some(stage) {
                out.push(*stage);
            }
        }
    }
    out
}

async fn retro_rows(fleet: &Arc<Hosted>, id: &SessionId) -> Vec<SessionRow> {
    Arc::clone(fleet)
        .get_session(id.clone())
        .await
        .expect("read")
        .rows
        .into_iter()
        .filter(|row| matches!(row, SessionRow::Retro { .. }))
        .collect()
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(100)).await;
}

fn titled(items: &[PartialRetroItem]) -> Vec<Option<&str>> {
    items.iter().map(|item| item.title.as_deref()).collect()
}

#[test]
fn a_value_counts_only_once_it_has_ended() {
    let answer = r#"{"items":[{"who":"owner","lands_in":"kit","title":"Walks opened in the browser","what":"The agent ran open on a walk.","fix":"Tell it.","evidence":["a:1"]},{"who":"ag"#;
    let at = |end: &str| answer.find(end).map(|at| at + end.len()).expect("in the answer");
    assert!(settled_items("").is_empty());
    assert!(settled_items(r#"{"items":[{"who":"own"#).is_empty(), "a word still being written");
    let [one] = &settled_items(&answer[..at(r#""who":"owner""#)])[..] else { panic!("one item") };
    assert_eq!(one.who.map(|who| who.domain()), Some(Whose::Owner));
    assert_eq!((one.title.as_deref(), one.lands_in.is_some()), (None, false));
    let half = &answer[..at(r#""title":"Walks opened in the bro"#) - 1];
    assert_eq!(settled_items(half)[0].title, None, "{half}");
    let [one] = &settled_items(&answer[..at(r#""fix":"Tell it.""#)])[..] else { panic!("one item") };
    assert_eq!(one.title.as_deref(), Some("Walks opened in the browser"));
    assert_eq!(one.what.as_deref(), Some("The agent ran open on a walk."));
    assert_eq!(one.fix.as_deref(), Some("Tell it."));
    assert_eq!(one.lands_in.map(|lands| lands.domain()), Some(LandsIn::Kit));
    // The second item opens with its first complete word, and the first is untouched.
    let both = settled_items(answer);
    assert_eq!(both.len(), 1, "the second has no complete field yet");
}

#[test]
fn items_come_in_the_order_written_and_nothing_nested_is_read_as_one() {
    let answer = "Here is the retro:\n```json\n{\"items\":[\
        {\"who\":\"agent\",\"lands_in\":\"kit\",\"title\":\"A \\\"quoted\\\" word\",\"what\":\"Line\\none \\u00e9.\",\
         \"fix\":\"Allow it.\",\"evidence\":[\"refusal:1\"],\"change\":{\"kind\":\"allow_command\",\"title\":\"not an item\"}},\
        {\"title\":\"Second\",\"who\":\"nobody\"}],\"note\":\"after\"}\n```";
    let items = settled_items(answer);
    assert_eq!(titled(&items), [Some("A \"quoted\" word"), Some("Second")]);
    assert_eq!(items[0].what.as_deref(), Some("Line\none \u{e9}."));
    assert_eq!(items[1].who, None, "a spelling nothing holds is no word");
    assert_eq!(items[0].who.map(|who| who.domain()), Some(Whose::Agent));
}

#[test]
fn a_movement_waits_for_its_gap_and_a_stage_or_the_end_never_does() {
    let t0 = Clock::now();
    let at = ipc::Instant::carried("2026-10-12T10:00:00Z");
    let mut progress = Progress::begin("retro-1".into(), at.clone(), RetroStage::ReadingRecord);
    assert!(!progress.due(t0), "nothing has moved");
    progress.heard(CallProgress::Answering { characters: 5, text: "{\"ite".into() });
    assert!(progress.due(t0), "the first is not held");
    let _ = progress.sent(t0);
    progress.heard(CallProgress::Answering { characters: 3, text: "ms\"".into() });
    assert!(!progress.due(t0 + GAP / 2));
    assert_eq!(progress.next_due(), Some(t0 + GAP));
    assert!(progress.due(t0 + GAP));
    // A stage change goes while a movement is held, and carries the count so far.
    let row = progress.staged(RetroStage::Writing, at.clone(), t0 + GAP / 2).expect("a new stage");
    assert!(matches!(row, SessionRow::Retro { stage: RetroStage::Writing, characters: 8, .. }));
    assert!(progress.staged(RetroStage::Writing, at.clone(), t0).is_none(), "the same stage is no news");
    // The last word goes whether or not a movement is due.
    progress.heard(CallProgress::Answering { characters: 2, text: "[]".into() });
    assert!(!progress.due(t0 + GAP / 2 + GAP / 4));
    let SessionRow::Retro { stage, retro, items, characters, why, .. } =
        progress.written(7, at, t0 + GAP / 2 + GAP / 4)
    else {
        panic!("a retro row")
    };
    assert_eq!((stage, retro, characters, why), (RetroStage::Written, Some(7), 10, None));
    assert!(items.is_empty(), "the kept retro holds the items");
    assert_eq!(progress.next_due(), None, "nothing is owed after the last word");
}

#[tokio::test]
async fn a_press_draws_its_stages_and_the_items_as_they_settle() {
    let (fleet, id, _home) = rig(FakeJudge::saying(TWO_ITEMS).streamed_in(24)).await;
    said(&fleet, &id, "no, show walks with show_window").await;
    let seen = collecting(&fleet);

    let started = std::time::Instant::now();
    let retro = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect("written");
    let took = started.elapsed();
    settle().await;

    // The agent was mid-turn, so it was not asked.
    assert_eq!(
        stages(&seen),
        [RetroStage::ReadingRecord, RetroStage::Writing, RetroStage::Written]
    );
    let rows = seen.lock().unwrap().clone();
    let ids: std::collections::BTreeSet<&str> = rows.iter().map(SessionRow::id).collect();
    assert_eq!(ids.len(), 1, "one row, replaced by id");

    let mut longest: Vec<PartialRetroItem> = Vec::new();
    let mut counts = Vec::new();
    for row in &rows {
        let SessionRow::Retro { stage: RetroStage::Writing, items, characters, .. } = row else {
            continue;
        };
        counts.push(*characters);
        for item in items {
            for title in item.title.iter() {
                assert!(
                    TWO_ITEMS.contains(&format!("\"title\":\"{title}\"")),
                    "never half a title: {title}"
                );
            }
        }
        assert!(items.len() >= longest.len(), "an item does not leave");
        longest = items.clone();
    }
    assert!(counts.windows(2).all(|pair| pair[0] <= pair[1]), "{counts:?}");
    assert!(counts.len() >= 2, "the count moved: {counts:?}");
    // Throttled against the clock the answer actually took: a loaded machine stretches the
    // streamed answer, and more rows for a longer answer is the throttle working. `GAP` itself is
    // pinned on a fixed clock above.
    let allowed = (took.as_millis() / GAP.as_millis()) as usize + 2;
    assert!(counts.len() <= allowed, "throttled: {} rows in {took:?}, at most {allowed}", counts.len());
    // The last frames may have come inside the gap before the row that replaced it, so what was
    // drawn is the front of the model's items, in order.
    let all = [
        Some("Walks opened in the browser"),
        Some("A reset waited a long time"),
        Some("Cites nothing real"),
    ];
    assert!(longest.len() >= 2, "{longest:?}");
    assert_eq!(titled(&longest), all[..longest.len()], "the model's items, before Fleet holds them to the record");
    assert_eq!(longest[0].who.map(|who| who.domain()), Some(Whose::Owner));
    assert_eq!(longest[0].lands_in.map(|lands| lands.domain()), Some(LandsIn::Kit));

    let SessionRow::Retro { stage, retro: number, items, characters, why, .. } =
        rows.last().cloned().expect("a last row")
    else {
        panic!("a retro row")
    };
    assert_eq!(stage, RetroStage::Written);
    assert_eq!(number, Some(1));
    assert_eq!(characters, TWO_ITEMS.chars().count() as u64);
    assert!(items.is_empty() && why.is_none());
    assert_eq!(retro.state, RetroState::Written);
    assert_eq!(retro.items.len(), 2, "the kept retro is what it was without a row");

    // Reopened, the thread holds the finished row.
    let [SessionRow::Retro { stage: RetroStage::Written, retro: Some(1), .. }] =
        &retro_rows(&fleet, &id).await[..]
    else {
        panic!("one finished row in the thread");
    };
}

#[tokio::test]
async fn a_call_that_streams_nothing_keeps_the_same_retro() {
    let (fleet, id, _home) = rig(FakeJudge::saying(TWO_ITEMS)).await;
    said(&fleet, &id, "no, show walks with show_window").await;
    let retro = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect("written");
    assert_eq!(retro.items.len(), 2);
    assert_eq!(retro.items[0].title.as_deref(), Some("Walks opened in the browser"));
    let [SessionRow::Retro { stage: RetroStage::Written, characters: 0, .. }] =
        &retro_rows(&fleet, &id).await[..]
    else {
        panic!("a finished row that never saw a frame");
    };
}

#[tokio::test]
async fn watched_and_unwatched_calls_say_the_same() {
    // The fake prints a frame per `sleep 0.05`, and a `sleep` costs about twice that on a loaded
    // Mac: 61 frames of 9 characters ran past the call's 5 second budget, and `TimedOut` was the
    // call doing what it is for. Fewer, larger frames still make the watched path select over
    // frames.
    let (fleet, id, _home) = rig(FakeJudge::saying(TWO_ITEMS).streamed_in(60)).await;
    let explaining = fleet.writing_retros().expect("a call can be made");
    let ask = Ask::put(explaining.model.clone(), "question", explaining.environment.clone())
        .expect("an ask");
    let plain = fleet.retro_said(&mut Live::none(), &explaining, &ask).await.expect("said");
    let progress = Progress::begin("retro-x".into(), fleet.instant(), RetroStage::Writing);
    let mut live = Live::on(id.as_str(), progress);
    let watched = fleet.retro_said(&mut live, &explaining, &ask).await.expect("said");
    assert_eq!(plain, watched);
}

#[tokio::test]
async fn a_refused_press_adds_no_row() {
    let (fleet, id, _home) =
        rig(FakeJudge::saying(TWO_ITEMS).taking(Duration::from_millis(400))).await;
    let nothing = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect_err("nothing to write about");
    assert_eq!(nothing.error().code, "fleet.session_retro_nothing_new");
    assert!(retro_rows(&fleet, &id).await.is_empty());

    said(&fleet, &id, "a first word").await;
    let first = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        let id = id.as_str().to_string();
        async move { fleet.write_session_retro(id).await }
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    let busy = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect_err("being written");
    assert_eq!(busy.error().code, "fleet.session_retro_being_written");
    // While the first is out, its row is live and the second added none.
    let [SessionRow::Retro { stage, .. }] = &retro_rows(&fleet, &id).await[..] else {
        panic!("one row for one press");
    };
    assert!(!stage.over() && *stage != RetroStage::Failed, "{stage:?}");
    first.await.unwrap().expect("the first is written");

    // Nothing new since: the last retro answers and no row is added.
    Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect("answered");
    assert_eq!(retro_rows(&fleet, &id).await.len(), 1);

    // Something new is a second press and a second row.
    said(&fleet, &id, "and another").await;
    Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect("written");
    let rows = retro_rows(&fleet, &id).await;
    assert!(matches!(
        &rows[..],
        [
            SessionRow::Retro { retro: Some(1), .. },
            SessionRow::Retro { retro: Some(2), .. }
        ]
    ));
}

#[tokio::test]
async fn a_failed_call_is_a_failed_row_with_fleets_words_and_keeps_nothing() {
    let (fleet, id, _home) = rig(FakeJudge::that_fails("the network")).await;
    said(&fleet, &id, "a word").await;
    let seen = collecting(&fleet);
    let refused = Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect_err("the call failed");
    assert_eq!(refused.error().code, "fleet.session_retro_failed");
    settle().await;
    assert_eq!(
        stages(&seen),
        [RetroStage::ReadingRecord, RetroStage::Writing, RetroStage::Failed]
    );
    let [SessionRow::Retro { stage: RetroStage::Failed, why: Some(why), retro: None, items, .. }] =
        &retro_rows(&fleet, &id).await[..]
    else {
        panic!("one failed row");
    };
    assert_eq!(why, &refused.error().message);
    assert!(why.contains("the call failed"), "{why}");
    assert!(items.is_empty());
    let pending = fleet.get_session_retro(id.as_str().to_string(), None).await.unwrap();
    assert_eq!(pending.state, RetroState::Pending);
}

#[tokio::test]
async fn a_row_a_restart_left_going_is_read_as_failed_and_stays_so() {
    let (fleet, id, _home) = rig(FakeJudge::saying(TWO_ITEMS)).await;
    let left = |stage| SessionRow::Retro {
        id: "retro-left".into(),
        at: fleet.instant(),
        stage,
        stage_since: fleet.instant(),
        characters: 40,
        items: vec![PartialRetroItem { title: Some("Half".into()), ..Default::default() }],
        retro: None,
        why: None,
    };
    fleet.row_put(id.as_str(), left(RetroStage::Writing)).await;
    let [SessionRow::Retro { stage, why, items, characters, .. }] =
        &retro_rows(&fleet, &id).await[..]
    else {
        panic!("one row");
    };
    assert_eq!(*stage, RetroStage::Failed);
    assert_eq!(why.as_deref(), Some(RESTARTED));
    assert!(items.is_empty());
    assert_eq!(*characters, 40);
    // Kept as failed, not only shown so.
    let stored = fleet.rows_of(id.as_str()).await.unwrap();
    assert!(matches!(&stored[..], [SessionRow::Retro { stage: RetroStage::Failed, .. }]));

    // A row that finished is left as it was.
    fleet.row_put(id.as_str(), left(RetroStage::Written)).await;
    assert!(matches!(
        &retro_rows(&fleet, &id).await[..],
        [SessionRow::Retro { stage: RetroStage::Written, why: None, .. }]
    ));
}

#[tokio::test]
async fn a_live_asking_of_the_agent_is_the_first_stage() {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.judge = Arc::new(FakeJudge::saying(TWO_ITEMS));
    let fleet: Hosted = Fleet::assembled(fittings)
        .hosting_sessions_on(Arc::new(Shared(Arc::clone(&stand_in))), Duration::from_secs(600));
    let manifest = ManifestId::carried(
        fleet.repositories().first().expect("served").manifest().id().as_str(),
    );
    let fleet = Arc::new(fleet);
    let id = fleet
        .start_session(StartSession {
            manifest_id: manifest,
            title: Some("Fix the flaky test".into()),
            model: None,
            effort: None,
            mode: None,
            root: None,
            pilot: None,
            fork: None,
        })
        .await
        .expect("started")
        .id;
    Arc::clone(&fleet)
        .send_session_message(SendSessionMessage {
            session_id: id.clone(),
            text: "no, use show_window".into(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        })
        .await
        .expect("taken");
    stand_in.init(0);
    stand_in.finishes(0, "done");
    let mut idle = false;
    for _ in 0..40 {
        let record = Arc::clone(&fleet).get_session(id.clone()).await.unwrap().session;
        idle = matches!(record.hosted.expect("hosted").turn, SessionTurn::Idle);
        if idle {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(idle, "the agent has finished its turn");
    let seen = collecting(&fleet);

    let pressed = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        let id = id.as_str().to_string();
        async move { fleet.write_session_retro(id).await }
    });
    for _ in 0..100 {
        let asked = stand_in
            .process(0)
            .sent
            .lock()
            .unwrap()
            .iter()
            .any(|line| line.contains("What got in your way"));
        if asked {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    settle().await;
    assert_eq!(stages(&seen), [RetroStage::AskingAgent], "waiting on the agent");
    tokio::time::sleep(Duration::from_millis(400)).await;
    // A turn starts with the agent's `init` line, which is what takes the message off the queue.
    stand_in.init(0);
    stand_in.finishes(0, "The reset took a long time.");
    pressed.await.unwrap().expect("written");
    settle().await;
    assert_eq!(
        stages(&seen),
        [
            RetroStage::AskingAgent,
            RetroStage::ReadingRecord,
            RetroStage::Writing,
            RetroStage::Written
        ]
    );
}

// Keep the unused-import check honest for the shared fakes.
#[allow(dead_code)]
type Fakes = (FakeHarness, FakeVcs);
