//! A Job's burn over time, served: a pause carries Fleet's words for what it waited on beside its id, the figures
//! are the store's, and a Job Fleet does not know is refused. `docs/practices/protocol.md`, *A Job's burn over time*.

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
            model: "haiku-4".into(),
            input_tokens: 1_000_000,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            output_tokens: 0,
            tools: tools.iter().map(|one| one.to_string()).collect(),
            subagent: false,
            bash_use: None,
        },
    }
}

#[tokio::test]
async fn a_pause_is_served_with_the_words_of_what_it_waited_on_and_an_unknown_job_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    assert!(fleet.get_job_timeline(SessionId::carried("s1")).await.is_err(), "no such Job");

    let started = SessionFact::Started { cwd: "/nowhere".into(), title: None, origin: ipc::SessionOrigin::Terminal, mod_version: None };
    fleet.report_session(report(started)).await.unwrap();
    let calls: [(&str, u32, &[&str]); 5] = [("m1", 0, &["Bash"]), ("m2", 10, &["Task"]), ("m3", 20, &[]), ("m4", 30, &["Read"]), ("m5", 40, &[])];
    for (id, minute, tools) in calls {
        fleet.report_session(report(turn(id, minute, tools))).await.unwrap();
    }

    let read = fleet.get_job_timeline(SessionId::carried(" s1 ")).await.unwrap();
    assert_eq!(read.session_id, "s1");
    let words: Vec<(&str, &str)> = read.pauses.iter().map(|one| (one.waited_on.as_str(), one.label.as_str())).collect();
    assert_eq!(words, [("command", "Command ran"), ("subagent", "Subagent ran"), ("you", "Waited on you"), ("other", "Paused")]);
    assert!(read.pauses.iter().all(|one| !one.cold), "a turn heard with no shares is not cold");
    assert_eq!(read.ended, None, "a Job that runs has no end");
    assert!(read.started.as_str() <= "2026-10-17T09:00:00.000Z", "the span begins no later than the first request: {}", read.started);
    assert!((read.total_units - 5.0).abs() < 1e-9, "{read:?}");
    assert_eq!((read.peak_context, read.cold_wakes, read.context.len()), (1_000_000, 0, 5));
    assert!(read.week_units >= 0.0 && read.subagent_share == 0.0 && !read.buckets.is_empty());
}
