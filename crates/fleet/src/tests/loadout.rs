//! A session's Loadout through a real Fleet: the latest list and the latest turn ride every record,
//! a repeat publishes nothing, and `get_session_loadout` answers the whole of it, turn by turn.
//! `docs/practices/protocol.md`, *A Session's Loadout*.

use std::time::Duration;

use api::{Next, Sessions, Subscription};
use ipc::{
    Event, LoadoutItem, LoadoutItems, LoadoutKind, LoadoutSource, LoadoutUse, LoadoutUseKind, SessionFact, SessionId,
    SessionRecord, SessionReport,
};
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

fn report(fact: SessionFact) -> SessionReport {
    SessionReport { harness: "a_harness".into(), session_id: SessionId::carried("s1"), fact }
}

fn item(kind: LoadoutKind, name: &str) -> LoadoutItem {
    LoadoutItem { kind, name: name.into(), source: LoadoutSource::Repository, from: None, state: None, tokens: None }
}

fn equipped(items: Vec<LoadoutItem>) -> SessionFact {
    SessionFact::Equipped { loadout: LoadoutItems { items } }
}

fn used(turn: u32, name: &str) -> SessionFact {
    SessionFact::Used {
        used: LoadoutUse {
            count: None,
            turn_started_at: ipc::Instant::carried(format!("2026-10-17T09:{turn:02}:00.000Z")),
            kind: LoadoutUseKind::Skill,
            name: name.into(),
            source: LoadoutSource::Repository,
            from: None,
            model: None,
            by: None,
        },
    }
}

/// The next `session.changed`, or `None` where none comes within a moment.
async fn next_session(watching: &mut Subscription, wait: Duration) -> Option<SessionRecord> {
    tokio::time::timeout(wait, async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::SessionChanged(record) = delivered.event {
                        return record;
                    }
                }
                Some(_) => continue,
                None => panic!("the stream ended"),
            }
        }
    })
    .await
    .ok()
}

#[tokio::test]
async fn the_latest_list_and_turn_ride_the_record_and_a_repeat_publishes_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let started = SessionFact::Started { cwd: "/nowhere".into(), title: None, origin: ipc::SessionOrigin::Terminal, mod_version: None };
    let first = fleet.report_session(report(started)).await.unwrap();
    assert_eq!(first.loadout_summary, None, "nothing equipped is no summary");
    let mut watching = fleet.events().subscribe();

    let list = vec![item(LoadoutKind::Skill, "land"), item(LoadoutKind::Tool, "Read"), item(LoadoutKind::Rules, "AGENTS.md")];
    let record = fleet.report_session(report(equipped(list.clone()))).await.unwrap();
    let summary = record.loadout_summary.expect("a summary");
    assert_eq!((summary.knows, summary.told, summary.latest_turn), (2, 1, None));
    assert!(next_session(&mut watching, Duration::from_secs(5)).await.is_some());

    fleet.report_session(report(equipped(list))).await.unwrap();
    assert_eq!(next_session(&mut watching, Duration::from_millis(200)).await, None, "the same list again");

    fleet.report_session(report(used(1, "land"))).await.unwrap();
    fleet.report_session(report(used(2, "land"))).await.unwrap();
    let record = fleet.report_session(report(used(2, "walk"))).await.unwrap();
    let latest = record.loadout_summary.expect("a summary").latest_turn.expect("a turn");
    assert_eq!(latest.started_at.as_str(), "2026-10-17T09:02:00.000Z");
    assert_eq!(latest.used.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["land", "walk"]);
    while next_session(&mut watching, Duration::from_millis(200)).await.is_some() {}
    fleet.report_session(report(used(2, "walk"))).await.unwrap();
    assert_eq!(next_session(&mut watching, Duration::from_millis(200)).await, None, "the same use in its turn");

    let listed = fleet.list_sessions(None, None, None).await.unwrap();
    assert_eq!(listed.sessions[0].loadout_summary.as_ref().map(|one| one.knows), Some(2));
}

#[tokio::test]
async fn a_repeated_use_is_counted_and_the_count_publishes_with_the_turn_end() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let started = SessionFact::Started { cwd: "/nowhere".into(), title: None, origin: ipc::SessionOrigin::Terminal, mod_version: None };
    fleet.report_session(report(started)).await.unwrap();
    fleet.report_session(report(equipped(vec![item(LoadoutKind::Skill, "land")]))).await.unwrap();
    let mut watching = fleet.events().subscribe();

    // The mod's own count is Fleet's to set, so it is ignored.
    let SessionFact::Used { used: mut first } = used(1, "land") else { unreachable!() };
    first.count = Some(9);
    fleet.report_session(report(SessionFact::Used { used: first })).await.unwrap();
    assert!(next_session(&mut watching, Duration::from_secs(5)).await.is_some(), "the first is new");
    for _ in 0..39 {
        fleet.report_session(report(used(1, "land"))).await.unwrap();
    }
    assert_eq!(next_session(&mut watching, Duration::from_millis(200)).await, None, "a repeat publishes nothing");

    fleet.report_session(report(SessionFact::TurnCompleted { said: None })).await.unwrap();
    let record = next_session(&mut watching, Duration::from_secs(5)).await.expect("the turn end publishes");
    let latest = record.loadout_summary.expect("a summary").latest_turn.expect("a turn");
    assert_eq!(latest.used.len(), 1);
    assert_eq!(latest.used[0].count, Some(40));

    let whole = fleet.get_session_loadout(SessionId::carried("s1")).await.unwrap();
    assert_eq!(whole.turns[0].used[0].count, Some(40));
    fleet.report_session(report(used(2, "land"))).await.unwrap();
    let whole = fleet.get_session_loadout(SessionId::carried("s1")).await.unwrap();
    assert_eq!(whole.turns[1].used[0].count, None, "heard once is left out");
}

#[tokio::test]
async fn the_whole_loadout_is_answered_turn_by_turn_and_an_unknown_session_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    assert!(fleet.get_session_loadout(SessionId::carried("s1")).await.is_err(), "no such session");

    fleet.report_session(report(used(1, "land"))).await.unwrap();
    let before = fleet.get_session_loadout(SessionId::carried("s1")).await.unwrap();
    assert!(before.items.is_empty(), "uses before any list are kept, with no list");
    fleet.report_session(report(equipped(vec![item(LoadoutKind::Skill, "land")]))).await.unwrap();
    fleet.report_session(report(used(2, "land"))).await.unwrap();
    fleet.report_session(report(used(2, "walk"))).await.unwrap();

    let whole = fleet.get_session_loadout(SessionId::carried("s1")).await.unwrap();
    assert_eq!(whole.items.len(), 1);
    let turns: Vec<(&str, Vec<&str>)> = whole
        .turns
        .iter()
        .map(|turn| (turn.started_at.as_str(), turn.used.iter().map(|one| one.name.as_str()).collect()))
        .collect();
    assert_eq!(turns, [("2026-10-17T09:01:00.000Z", vec!["land"]), ("2026-10-17T09:02:00.000Z", vec!["land", "walk"])]);
}

#[tokio::test]
async fn a_jobs_attachment_history_is_every_version_oldest_first_and_an_unknown_job_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    assert!(fleet.get_attachment_history(SessionId::carried("s1")).await.is_err(), "no such session");

    let artifact = |detail: &[(&str, &str)]| SessionFact::Attached {
        attachment: ipc::AttachmentReport {
            kind: "artifact".into(),
            target: "notes.md".into(),
            detail: detail.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        },
    };
    fleet.report_session(report(artifact(&[]))).await.unwrap();
    fleet.report_session(report(artifact(&[("lines", "40")]))).await.unwrap();
    fleet.report_session(report(artifact(&[("lines", "40")]))).await.unwrap();
    fleet
        .report_session(report(SessionFact::Settled {
            attachment: ipc::AttachmentNamed { kind: "artifact".into(), target: "notes.md".into() },
            state: ipc::AttachmentState::Spent,
        }))
        .await
        .unwrap();

    let history = fleet.get_attachment_history(SessionId::carried("s1")).await.unwrap();
    let read: Vec<_> = history.versions.iter().map(|v| (v.kind.as_str(), v.target.as_str(), v.state)).collect();
    assert_eq!(
        read,
        [
            ("artifact", "notes.md", ipc::AttachmentState::Standing),
            ("artifact", "notes.md", ipc::AttachmentState::Standing),
            ("artifact", "notes.md", ipc::AttachmentState::Spent),
        ],
        "a repeat adds none"
    );
    assert!(history.versions[0].detail.is_empty());
    assert_eq!(history.versions[1].detail.get("lines").map(String::as_str), Some("40"));
    assert!(history.versions.windows(2).all(|pair| pair[0].at.as_str() <= pair[1].at.as_str()));
}
