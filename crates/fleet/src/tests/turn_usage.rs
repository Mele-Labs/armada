//! A session's turns through a real Fleet: each is kept under its message id, a repeat is one, and
//! `get_session_usage` answers a window with the model, the four token fields and the tools.
//! `docs/practices/protocol.md`, *Usage a turn at a time*.

use api::Sessions;
use ipc::{SessionFact, SessionId, SessionReport, TurnUsage};
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

fn report(fact: SessionFact) -> SessionReport {
    SessionReport { harness: "a_harness".into(), session_id: SessionId::carried("s1"), fact }
}

fn turn(message_id: &str, minute: u32, tools: &[&str]) -> SessionFact {
    SessionFact::TurnMeasured {
        turn: TurnUsage {
            message_id: message_id.into(),
            at: ipc::Instant::carried(format!("2026-10-17T09:{minute:02}:00.000Z")),
            model: "model-a".into(),
            input_tokens: 6,
            cache_creation_tokens: 1000,
            cache_read_tokens: 10_000,
            output_tokens: 90,
            tools: tools.iter().map(|one| one.to_string()).collect(),
            subagent: false,
            bash_use: None,
        },
    }
}

#[tokio::test]
async fn turns_are_kept_once_and_read_by_window_and_an_unknown_session_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    assert!(fleet.get_session_usage(SessionId::carried("s1"), None, None).await.is_err(), "no such session");

    let started = SessionFact::Started { cwd: "/nowhere".into(), title: None, origin: ipc::SessionOrigin::Terminal, mod_version: None };
    fleet.report_session(report(started)).await.unwrap();
    fleet.report_session(report(turn("m1", 1, &["Bash", "Read"]))).await.unwrap();
    fleet.report_session(report(turn("m1", 1, &["Bash", "Read"]))).await.unwrap();
    fleet.report_session(report(turn("m2", 2, &[]))).await.unwrap();
    fleet.report_session(report(turn("m3", 3, &["Edit"]))).await.unwrap();

    let all = fleet.get_session_usage(SessionId::carried("s1"), None, None).await.unwrap();
    let ids: Vec<&str> = all.turns.iter().map(|one| one.message_id.as_str()).collect();
    assert_eq!(ids, ["m1", "m2", "m3"]);
    assert_eq!(all.turns[0].tools, ["Bash", "Read"]);
    assert_eq!((all.turns[0].input_tokens, all.turns[0].cache_creation_tokens), (6, 1000));
    assert_eq!((all.turns[0].cache_read_tokens, all.turns[0].output_tokens), (10_000, 90));

    let window = fleet
        .get_session_usage(SessionId::carried("s1"), Some("2026-10-17T09:02:00.000Z".into()), Some("2026-10-17T09:03:00.000Z".into()))
        .await
        .unwrap();
    assert_eq!(window.turns.len(), 1);
    assert_eq!(window.turns[0].message_id, "m2");
}
