//! A Job that crosses a token rule is told in its thread, once, and the telling is kept as a row with
//! the figure and the burn of the hour before. The turns are put in the store by hand, as ingest
//! leaves them, and the Job is a hosted one.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{ManifestId, SessionId, SessionRow, StartSession};
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

#[tokio::test]
async fn a_job_over_a_limit_is_told_once_and_the_telling_is_kept() {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = Arc::new(
        rescuing_on(&home, fittings(&home, FakeWorkProduct::changed(&[])))
            .hosting_sessions_on(Arc::new(Shared(stand_in)), Duration::from_secs(600)),
    );
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let session = fleet
        .start_session(StartSession {
            manifest_id: ManifestId::carried(manifest),
            title: None,
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
        })
        .await
        .expect("started")
        .id
        .as_str()
        .to_string();

    heard(&fleet, &session, &[turn(0, 90_000), turn(1, 95_000)]).await;
    assert!(thread_of(&fleet, &session).await.is_empty(), "under every limit");

    heard(&fleet, &session, &[turn(2, 312_000)]).await;
    let said = thread_of(&fleet, &session).await;
    assert_eq!(said, ["Context is 312k tokens. To carry on in a new job, ask the owner with the options \"Carry on in a new Job\" and \"Stay here\", and write your brief (where the work is, what is open, what you hold) in the description of \"Carry on in a new Job\". Start nothing yourself."]);
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
    heard(&fleet, &session, &[turn(6, 340_000)]).await;
    assert_eq!(thread_of(&fleet, &session).await.len(), 1, "fell under, but the cooldown has not passed");

    heard(&fleet, &session, &[turn(7, 80_000)]).await;
    heard(&fleet, &session, &[turn(45, 340_000)]).await;
    assert_eq!(thread_of(&fleet, &session).await.len(), 2, "re-armed after falling under and the cooldown");
    assert_eq!(fleet.store().lock().await.usage_rule_trips(&session).unwrap().len(), 2);

    // The hour after the first trip (08:02) is covered by a turn at 09:02. Its turns are those after
    // 08:02 up to then: gaps of 1, 1, 1, 1, 38 and 17 minutes, the last two counted as 5, so 14
    // active minutes for 0.193 units. The second trip (08:45) is not covered yet.
    heard(&fleet, &session, &[turn(62, 90_000)]).await;
    let trips = fleet.store().lock().await.usage_rule_trips(&session).unwrap();
    assert_eq!(trips[0].after_turns, Some(7));
    let rate = trips[0].burn_per_hour_after.expect("a reading");
    assert!((rate - 0.193 / (14.0 / 60.0)).abs() < 1e-9, "{rate}");
    assert_eq!((trips[1].after_turns, trips[1].burn_per_hour_after), (None, None));
    assert_eq!(thread_of(&fleet, &session).await.len(), 2, "measuring tells nobody");

    let asked = Arc::clone(&fleet)
        .get_usage_rules(Some("2026-10-18T00:00:00Z".into()), Some("2026-10-19T00:00:00Z".into()))
        .await
        .unwrap();
    let names: Vec<&str> = asked.rules.iter().map(|one| one.rule.as_str()).collect();
    assert_eq!(names, ["context", "subagents", "cache_hit", "opus_reading"]);
    let context = &asked.rules[0];
    assert_eq!((context.on, context.limit.value, context.limit.unit.as_str()), (true, 300, "thousand tokens"));
    let ats: Vec<&str> = context.trips.iter().map(|one| one.at.as_str()).collect();
    assert_eq!(ats, ["2026-10-18T08:45:00.000Z", "2026-10-18T08:02:00.000Z"]);
    assert_eq!(context.trips[0].burn_per_hour_after, None);
    assert!(context.trips[1].burn_per_hour_after.is_some());
    assert!(asked.rules[1].trips.is_empty());
    assert!(Arc::clone(&fleet).get_usage_rules(Some("lately".into()), None).await.is_err());
}
