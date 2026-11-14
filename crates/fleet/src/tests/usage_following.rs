//! A Job that crosses a token rule is told in its thread, once, and the telling is kept as a row with
//! the figure and the burn of the hour before. The turns are put in the store by hand, as ingest
//! leaves them, and the Job is a hosted one.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{ManifestId, SessionId, SessionRow, SetWaitingFor, StartSession, WaitingInput};
use store::KeptTurnUsage;
use testkit::FakeWorkProduct;

use super::rescuing::{rescuing_on, Rescued};
use super::session_host::{Shared, StandIn};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

async fn thread_of(fleet: &Arc<Rescued>, id: &str) -> Vec<String> {
    Arc::clone(fleet)
        .get_session(SessionId::carried(id))
        .await
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

fn turn(minute: u32, context: u64) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: format!("m{minute}"),
        at: format!("2026-10-18T{:02}:{:02}:00.000Z", 8 + minute / 60, minute % 60),
        model: "model-a".into(),
        input_tokens: 0,
        cache_creation_tokens: 0,
        cache_creation_1h_tokens: 0,
        cache_read_tokens: context,
        output_tokens: 1_000,
        tools: vec!["Edit".into()],
        subagent: false,
        bash_use: None,
    }
}

async fn heard(fleet: &Arc<Rescued>, session: &str, turns: &[KeptTurnUsage]) {
    let mut store = fleet.store().lock().await;
    for one in turns {
        store.keep_turn_usage(session, one).unwrap();
    }
    drop(store);
    fleet.usage_rules_read(&[session.to_string()]).await;
}

async fn started() -> (TempDir, Arc<Rescued>, String) {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = Arc::new(
        rescuing_on(&home, fittings(&home, FakeWorkProduct::changed(&[])))
            .hosting_sessions_on(Arc::new(Shared(stand_in)), Duration::from_secs(600)),
    );
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let session = fleet
        .start_session(StartSession {
            project: None,
            manifest_id: ManifestId::carried(manifest),
            title: None,
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
        })
        .await
        .expect("started")
        .id
        .as_str()
        .to_string();
    (home, fleet, session)
}

#[tokio::test]
async fn a_job_over_a_limit_is_told_once_and_the_telling_is_kept() {
    let (_home, fleet, session) = started().await;

    heard(&fleet, &session, &[turn(0, 90_000), turn(1, 95_000)]).await;
    assert!(thread_of(&fleet, &session).await.is_empty(), "under every limit");

    heard(&fleet, &session, &[turn(2, 312_000)]).await;
    let said = thread_of(&fleet, &session).await;
    assert_eq!(said, ["Context is 312k tokens, past the 300k limit. Write only a brief of where the work is, what is open and what you hold, and send it with handoff_brief. The owner answers on a card. Ask him nothing and start nothing yourself. Once it is sent, end your turn."]);
    let tell = fleet.store().lock().await.tell_of(&session).unwrap().expect("the tell is kept for the refusal reader");
    assert_eq!(tell.kind, "context");
    let trips = fleet.store().lock().await.usage_rule_trips(&session).unwrap();
    assert_eq!(trips.len(), 1);
    assert_eq!((trips[0].rule.as_str(), trips[0].at.as_str()), ("context", "2026-10-18T08:02:00.000Z"));
    assert!((trips[0].figure - 312_000.0).abs() < 1e-6);
    // Three turns a minute apart, each 1,000 output tokens and the context read from the cache: 0.0647
    // units over two active minutes.
    let expected = (90_000.0 * 0.1 + 95_000.0 * 0.1 + 312_000.0 * 0.1 + 3.0 * 1_000.0 * 5.0) / 1_000_000.0 / (2.0 / 60.0);
    assert!((trips[0].burn_per_hour_before - expected).abs() < 1e-9, "{}", trips[0].burn_per_hour_before);

    heard(&fleet, &session, &[turn(3, 320_000), turn(4, 330_000)]).await;
    assert_eq!(thread_of(&fleet, &session).await.len(), 1, "staying over tells it nothing more");

    heard(&fleet, &session, &[turn(5, 80_000)]).await;
    heard(&fleet, &session, &[turn(45, 340_000)]).await;
    assert_eq!(thread_of(&fleet, &session).await.len(), 1, "told once for the Job: falling under and over again says nothing");
    assert_eq!(fleet.store().lock().await.usage_rule_trips(&session).unwrap().len(), 1);

    // The hour after the first trip (08:02) is covered by a turn at 09:02. Its turns are those after
    // 08:02 up to then: gaps of 1, 1, 40 and 17 minutes, the last two counted as 5, so 12
    // active minutes for 0.141 units.
    heard(&fleet, &session, &[turn(62, 90_000)]).await;
    let trips = fleet.store().lock().await.usage_rule_trips(&session).unwrap();
    assert_eq!(trips[0].after_turns, Some(5));
    let rate = trips[0].burn_per_hour_after.expect("a reading");
    assert!((rate - 0.141 / (12.0 / 60.0)).abs() < 1e-9, "{rate}");
    assert_eq!(thread_of(&fleet, &session).await.len(), 1, "measuring tells nobody");

    let asked = Arc::clone(&fleet)
        .get_usage_rules(Some("2026-10-18T00:00:00Z".into()), Some("2026-10-19T00:00:00Z".into()))
        .await
        .unwrap();
    let names: Vec<&str> = asked.rules.iter().map(|one| one.rule.as_str()).collect();
    assert_eq!(names, ["context"]);
    let context = &asked.rules[0];
    assert_eq!((context.on, context.limit.value, context.limit.unit.as_str()), (true, 300, "thousand tokens"));
    let ats: Vec<&str> = context.trips.iter().map(|one| one.at.as_str()).collect();
    assert_eq!(ats, ["2026-10-18T08:02:00.000Z"]);
    assert!(context.trips[0].burn_per_hour_after.is_some());
    assert!(Arc::clone(&fleet).get_usage_rules(Some("lately".into()), None).await.is_err());
}

#[tokio::test]
async fn the_size_nudge_waits_for_a_quiet_moment_and_is_sent_once() {
    let (_home, fleet, session) = started().await;
    let id = SessionId::carried(&session);
    let wait = |items: Vec<WaitingInput>| SetWaitingFor { session_id: Some(id.clone()), items };
    fleet
        .waiting_for(None, wait(vec![WaitingInput { approves: false, id: "pick".into(), text: "Pick the name".into(), act: None, options: Vec::new(), steps: Vec::new() }]))
        .await
        .expect("kept");

    heard(&fleet, &session, &[turn(0, 90_000), turn(1, 312_000)]).await;
    assert!(thread_of(&fleet, &session).await.is_empty(), "something waits on him: held");

    // Nothing new is heard from the Job, and the held telling goes the moment he has nothing to decide.
    fleet.waiting_for(None, wait(Vec::new())).await.expect("kept");
    fleet.usage_rules_read(&[]).await;
    assert_eq!(thread_of(&fleet, &session).await.len(), 1, "sent at the quiet moment");

    fleet.usage_rules_read(&[]).await;
    heard(&fleet, &session, &[turn(2, 330_000)]).await;
    assert_eq!(thread_of(&fleet, &session).await.len(), 1, "once for the Job");
}
