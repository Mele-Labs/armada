//! What the timeline enters of a Session's and a Drone's acts: the reads, edits and commands that come
//! one per tool call and are entered one per burst, a Session's wait and its end, its subagents and
//! the Sessions it starts.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::Sessions;
use ipc::{SessionFact, SessionId, SessionOrigin, SessionReport, TimelineSubject, TimelineTone};

use super::timeline::{kept, kinds, publish, read};
use crate::tests::tmp::TempDir;
use crate::timeline::{Doer, JobToolsSeen};
use crate::transcript::Tap;

fn row(id: &str, state: &str, waiting: &str, attachments: &str) -> String {
    format!(
        r#"{{"kind":"session.changed","id":"{id}","harness":"a-harness","origin":"bridge","manifest_id":"armada",
        "cwd":"/work","title":"Flaky store test","state":"{state}","started_at":"2026-10-12T09:00:00.000Z",
        "last_seen_at":"2026-10-12T09:40:00.000Z","usage":{{}},"attachments":[{attachments}],
        "waiting_for":[{waiting}]}}"#
    )
}

const LISTED: &str = r#"{"id":"w1","text":"Review the plan","since":"2026-10-12T09:20:00.000Z","source":"agent"}"#;

fn subagent(state: &str, changed: &str) -> String {
    format!(
        r#"{{"kind":"subagent","target":"a1","state":"{state}","detail":{{"type":"Explore","description":"Map file claims"}},
        "since":"2026-10-12T09:21:00.000Z","changed_at":"{changed}"}}"#
    )
}

/// What the agent listed is a wait and its leaving the list is a resolution, each once; a subagent is
/// entered starting and finishing; and neither is read into a Session Fleet found already holding it
/// or one whose list went with its end.
#[tokio::test]
async fn a_wait_a_resolution_and_a_subagent_are_entered_by_what_the_row_changed_to() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    publish(&fleet, &row("s1", "live", "", ""));
    publish(&fleet, &row("s1", "live", LISTED, &subagent("standing", "2026-10-12T09:21:00.000Z")));
    publish(&fleet, &row("s1", "live", LISTED, &subagent("standing", "2026-10-12T09:21:00.000Z")));
    publish(&fleet, &row("s1", "live", "", &subagent("spent", "2026-10-12T09:25:00.000Z")));
    // Held before Fleet looked, and ended with a list.
    publish(&fleet, &row("s2", "live", LISTED, &subagent("spent", "2026-10-12T09:25:00.000Z")));
    publish(&fleet, &row("s2", "ended", "", &subagent("spent", "2026-10-12T09:25:00.000Z")));

    let events = read(&fleet).await;
    let of = |id: &str| -> Vec<_> {
        events
            .iter()
            .filter(|one| one.subject == TimelineSubject::Session { id: SessionId::carried(id) })
            .collect()
    };
    // Oldest first by the instant each states, so the resolution, dated by the row that dropped it,
    // follows the finish the subagent states for itself.
    let first = of("s1");
    assert_eq!(
        first.iter().map(|one| one.kind.as_wire()).collect::<Vec<_>>(),
        ["session.started", "session.waiting", "session.subagent", "session.subagent", "session.resolved"]
    );
    assert_eq!((first[1].says.as_str(), first[1].tone), ("Waiting: Review the plan", TimelineTone::Ask));
    assert_eq!(first[1].at.as_str(), "2026-10-12T09:20:00.000Z", "when the item began to wait");
    assert_eq!((first[2].says.as_str(), first[2].detail.as_deref()), ("Subagent started", Some("Explore: Map file claims")));
    assert_eq!((first[3].says.as_str(), first[3].tone), ("Subagent finished", TimelineTone::Ok));
    assert_eq!(first[3].at.as_str(), "2026-10-12T09:25:00.000Z");
    assert_eq!((first[4].says.as_str(), first[4].tone), ("Resolved: Review the plan", TimelineTone::Ok));
    let second: Vec<_> = of("s2").iter().map(|one| one.kind.as_wire()).collect();
    assert_eq!(second, ["session.started", "session.waiting", "session.ended"], "no resolution, no subagent");
}

/// A subagent its parent sent more work is entered again, from when it started on it, and finishes again.
#[tokio::test]
async fn a_resumed_subagent_is_entered_resuming_and_finishing_again() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    let resumed = subagent("standing", "2026-10-12T09:30:00.000Z").replace("09:21:00", "09:30:00");
    publish(&fleet, &row("s1", "live", "", ""));
    publish(&fleet, &row("s1", "live", "", &subagent("standing", "2026-10-12T09:21:00.000Z")));
    publish(&fleet, &row("s1", "live", "", &subagent("spent", "2026-10-12T09:25:00.000Z")));
    publish(&fleet, &row("s1", "live", "", &resumed));
    publish(&fleet, &row("s1", "live", "", &resumed));
    publish(&fleet, &row("s1", "live", "", &subagent("spent", "2026-10-12T09:35:00.000Z")));

    let said: Vec<_> = read(&fleet)
        .await
        .into_iter()
        .filter(|one| one.kind.as_wire() == "session.subagent")
        .map(|one| (one.says, one.tone, one.at.as_str().to_string()))
        .collect();
    let at = |time: &str| format!("2026-10-12T{time}.000Z");
    assert_eq!(
        said,
        [
            ("Subagent started".to_string(), TimelineTone::Run, at("09:21:00")),
            ("Subagent finished".to_string(), TimelineTone::Ok, at("09:25:00")),
            ("Subagent resumed".to_string(), TimelineTone::Run, at("09:30:00")),
            ("Subagent finished".to_string(), TimelineTone::Ok, at("09:35:00")),
        ]
    );
}

async fn report(fleet: &super::timeline::Fixture, id: &str, fact: SessionFact) {
    fleet
        .report_session(SessionReport {
            harness: "a-harness".to_string(),
            session_id: SessionId::carried(id),
            fact,
        })
        .await
        .expect("a report is kept");
}

/// A terminal Session's mod tells each file it read and wrote and each command it ran; the timeline
/// holds them until the turn ends and enters one entry for each act, with the count and the latest.
#[tokio::test]
async fn a_terminal_sessions_acts_are_entered_once_where_its_turn_ends() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    let started = SessionFact::Started {
        cwd: "/work".to_string(),
        title: Some("Flaky store test".to_string()),
        origin: SessionOrigin::Terminal,
        mod_version: None,
    };
    report(&fleet, "t1", started).await;
    for n in 0..3 {
        report(&fleet, "t1", SessionFact::Read { path: format!("src/{n}.rs") }).await;
    }
    report(&fleet, "t1", SessionFact::Ran { command: "API_TOKEN=abc123 cargo test".to_string() }).await;
    report(&fleet, "t1", SessionFact::Touched { paths: vec!["src/a.rs".to_string(), "src/b.rs".to_string()], by: None, act: None }).await;
    assert_eq!(kinds(&read(&fleet).await), ["session.started"], "nothing is entered while the burst is open");

    report(&fleet, "t1", SessionFact::TurnCompleted { said: None }).await;
    let events = read(&fleet).await;
    assert_eq!(events[0].kind.as_wire(), "session.started");
    // Dated by each act's first call, so the order between them is the clock's.
    let mut acts: Vec<_> = events[1..].iter().map(|one| (one.kind.as_wire(), one.says.as_str(), one.detail.as_deref())).collect();
    acts.sort();
    assert_eq!(
        acts,
        [
            ("session.edited".to_string(), "Edited 2 files", Some("src/b.rs")),
            ("session.ran".to_string(), "Ran 1 command", Some("API_TOKEN=*** cargo test")),
            ("session.read".to_string(), "Read 3 files", Some("src/2.rs")),
        ]
    );
    assert!(events[1..].iter().all(|one| one.tone == TimelineTone::Run && one.live));
    assert!(events[1..].iter().all(|one| one.title.as_deref() == Some("Flaky store test")));
}

/// A Job's Drone reads and runs are counted off its stream; its edits are `job.files`, so they are not.
#[tokio::test]
async fn a_jobs_reads_and_commands_are_entered_and_its_edits_are_left_to_its_files() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    let tap = JobToolsSeen {
        harness: Arc::clone(fleet.harness()),
        timeline: fleet.timeline().clone(),
        doer: Doer::job("01JOB", Some("job-7".to_string()), Some("armada".to_string())),
    };
    let called = |tool: &str, detail: &str| DroneEvent::Called {
        tool: tool.to_string(),
        call: "c".to_string(),
        detail: CallDetail::of(detail),
    };
    tap.saw(&[
        called(testkit::READS, "src/a.rs"),
        called(testkit::READS, "src/b.rs"),
        called(testkit::RUNS, "cargo build"),
        called(testkit::RUNS, "API_TOKEN=abc123 deploy now"),
        called(testkit::EDITS, "src/a.rs +3 -1"),
        called("another_tool", "anything"),
    ]);
    fleet.timeline().close_bursts_of(&Doer::job("01JOB", None, None));

    let events = read(&fleet).await;
    assert_eq!(kinds(&events), ["job.read", "job.ran"]);
    assert_eq!((events[0].says.as_str(), events[0].detail.as_deref()), ("Read 2 files", Some("src/b.rs")));
    assert_eq!(events[0].title.as_deref(), Some("job-7"));
    assert_eq!(events[1].says, "Ran 2 commands");
    assert_eq!(events[0].items, ["src/b.rs", "src/a.rs"], "newest first, kept through the store");
    assert_eq!(events[1].items, ["API_TOKEN=*** deploy now", "cargo build"], "a command is hidden as its detail is");
    assert_eq!(events[1].detail.as_deref(), events[1].items.first().map(String::as_str));
}

/// A Session whose agent starts another, or forks its own, is the subject of the entry.
#[tokio::test]
async fn a_session_that_starts_another_is_entered() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    fleet.session_spawned("s1", (Some("Parent".to_string()), Some("armada".to_string())), "Child", false);
    fleet.session_spawned("s1", (None, None), "Fork", true);

    let events = read(&fleet).await;
    assert_eq!(kinds(&events), ["session.spawned", "session.spawned"]);
    assert_eq!((events[0].says.as_str(), events[0].detail.as_deref()), ("Started a session", Some("Child")));
    assert_eq!(events[0].title.as_deref(), Some("Parent"));
    assert_eq!(events[1].says, "Forked a session");
}

/// A Session Bridge hosts is heard from its own process, so its calls are counted off that stream
/// and entered where its turn ends, ahead of the turn's own length.
#[tokio::test]
async fn a_hosted_sessions_calls_are_entered_where_its_turn_ends() {
    use crate::tests::session_host::{eventually, Shared, StandIn};
    use api::HostedSessions;

    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = Arc::new(
        crate::tests::daemon::a_fleet(&home, testkit::FakeWorkProduct::changed(&[]))
            .hosting_sessions_on(Arc::new(Shared(Arc::clone(&stand_in))), std::time::Duration::from_secs(600)),
    );
    crate::timeline::keep_timeline(Arc::clone(&fleet));
    let manifest = fleet.repositories().first().expect("a repository").manifest().id().as_str().to_string();
    let started = Arc::clone(&fleet)
        .start_session(ipc::StartSession {
            manifest_id: ipc::ManifestId::carried(&manifest),
            root: None,
            title: None,
            model: None,
            effort: None,
            mode: None,
            pilot: None,
            fork: None,
        })
        .await
        .expect("started");
    let message = ipc::SendSessionMessage {
        session_id: started.id.clone(),
        text: "fix the login".into(),
        attachments: Vec::new(),
        mentions: Vec::new(),
    };
    Arc::clone(&fleet).send_session_message(message).await.expect("taken");
    stand_in.init(0);
    eventually(|| async { fleet.hosts().of(started.id.as_str()).state().turn_began.is_some() }).await;
    let called = |tool: &str, detail: &str| DroneEvent::Called {
        tool: tool.to_string(),
        call: "c".to_string(),
        detail: CallDetail::of(detail),
    };
    stand_in.says(
        0,
        vec![
            called(testkit::READS, "src/a.rs"),
            called(testkit::READS, "src/b.rs"),
            called(testkit::RUNS, "cargo test"),
            called(testkit::EDITS, "src/b.rs"),
            called("an_armada_tool", "anything"),
        ],
    );
    stand_in.finishes(0, "done");
    eventually(|| async {
        let thread = Arc::clone(&fleet).get_session(started.id.clone()).await;
        thread.is_ok_and(|one| one.session.hosted.is_some_and(|hosted| hosted.turn == ipc::SessionTurn::Idle))
    })
    .await;

    let events = read(&fleet).await;
    let mut acts: Vec<_> = events
        .iter()
        .filter(|one| matches!(one.kind.as_wire().as_str(), "session.read" | "session.edited" | "session.ran"))
        .map(|one| (one.kind.as_wire(), one.says.clone(), one.detail.clone()))
        .collect();
    acts.sort();
    let want = |kind: &str, says: &str, detail: &str| (kind.to_string(), says.to_string(), Some(detail.to_string()));
    assert_eq!(
        acts,
        [
            want("session.edited", "Edited 1 file", "src/b.rs"),
            want("session.ran", "Ran 1 command", "cargo test"),
            want("session.read", "Read 2 files", "src/b.rs"),
        ]
    );
}
