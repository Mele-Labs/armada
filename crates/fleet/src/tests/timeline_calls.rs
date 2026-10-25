//! Calls a Session makes outside the repository, entered on the timeline one for each call: from a
//! terminal Session's mod and from a Session Bridge hosts, a subagent's counted as the Session's.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Sessions};
use ipc::{SessionFact, SessionId, SessionOrigin, SessionReport, TimelineKind};
use testkit::{FakeWorkProduct, REACHES};

use super::timeline::{kept, kinds, read};
use crate::tests::tmp::TempDir;
use crate::timeline::Doer;

async fn report(fleet: &super::timeline::Fixture, fact: SessionFact) {
    fleet
        .report_session(SessionReport { harness: "a-harness".to_string(), session_id: SessionId::carried("t1"), fact })
        .await
        .expect("a report is kept");
}

/// A terminal Session's `gh` command is a call to the forge, entered at once; any other command is still
/// counted in the burst of commands, and the `gh` one is in neither's count.
#[tokio::test]
async fn a_terminal_sessions_gh_command_is_a_call_and_not_a_command_run() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    report(
        &fleet,
        SessionFact::Started {
            cwd: "/work".to_string(),
            title: Some("Land it".to_string()),
            origin: SessionOrigin::Terminal,
            mod_version: None,
        },
    )
    .await;
    let reached = adapters::command_reach("gh pr view").expect("a call");
    report(&fleet, SessionFact::Ran { command: "gh pr view 2540 --json body --jq .body".to_string() }).await;
    report(&fleet, SessionFact::Ran { command: "gh issue create --title t --body 'a password in it'".to_string() }).await;
    report(&fleet, SessionFact::Ran { command: "cargo test --workspace".to_string() }).await;
    fleet.timeline().close_bursts_of(&Doer::session("t1", None, None));

    let events = read(&fleet).await;
    let calls: Vec<_> = events.iter().filter(|one| one.kind == TimelineKind::SessionCalled).collect();
    assert_eq!(calls.len(), 2, "{:?}", kinds(&events));
    assert_eq!(calls[0].says, format!("Called {}", reached.name));
    assert_eq!(calls[0].detail.as_deref(), Some("gh pr view"));
    let call = calls[0].call.as_ref().expect("a call");
    assert_eq!((call.service.as_str(), call.line.as_str()), (reached.service.as_str(), "gh pr view"));
    assert_eq!(call.name, reached.name);
    assert_eq!(calls[0].title.as_deref(), Some("Land it"));
    assert!(calls[0].live, "the Session is working");
    assert_eq!(calls[1].says, "Called the issue tracker");
    assert_eq!(calls[1].call.as_ref().map(|one| one.line.as_str()), Some("gh issue create"));
    assert!(!ipc::encode(&events).expect("plain data").contains("password"), "no argument is kept");

    let ran: Vec<_> = events.iter().filter(|one| one.kind == TimelineKind::SessionRan).collect();
    assert_eq!(ran.len(), 1, "{:?}", kinds(&events));
    assert_eq!(ran[0].says, "Ran 1 command");
    assert_eq!(ran[0].call, None, "a command run names no call");
}

/// The entry is the wire's: `call` rides beside `detail`, survives the store, and is left out of every
/// other entry rather than sent as null.
#[tokio::test]
async fn a_call_is_served_with_its_service_and_line_after_it_was_kept() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    report(
        &fleet,
        SessionFact::Started { cwd: "/work".to_string(), title: None, origin: SessionOrigin::Terminal, mod_version: None },
    )
    .await;
    report(&fleet, SessionFact::Ran { command: "gh run list --limit 3".to_string() }).await;
    fleet.timeline().settled().await;
    let window = fleet.fleet_timeline(ipc::TimelineAsk::default()).await.expect("a timeline");
    let served = ipc::encode(&window).expect("plain data");
    assert!(served.contains(r#""kind":"session.called""#), "{served}");
    assert!(served.contains(r#""detail":"gh run list""#), "{served}");
    let reached = adapters::command_reach("gh run list").expect("a call");
    let call = format!(r#""call":{{"service":"{}","name":"{}","line":"gh run list"}}"#, reached.service, reached.name);
    assert!(served.contains(&call), "{served}");
    assert_eq!(served.matches(r#""call""#).count(), 1, "only the call carries one: {served}");
    assert!(!served.contains("null"), "{served}");
}

/// A hosted Session's call, and a subagent's in it, are entered under the Session through the harness, and
/// the call counts as the Session working.
#[tokio::test]
async fn a_hosted_sessions_call_is_entered_through_the_harness() {
    use crate::tests::planted::Held;
    use crate::tests::session_host::{eventually, Shared, StandIn};

    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.clock = Arc::new(Held::started());
    let fleet = Arc::new(
        crate::Fleet::assembled(fittings)
            .hosting_sessions_on(Arc::new(Shared(Arc::clone(&stand_in))), std::time::Duration::from_secs(600)),
    );
    crate::timeline::keep_timeline(Arc::clone(&fleet));
    let manifest = fleet.repositories().first().expect("a repository").manifest().id().as_str().to_string();
    let started = Arc::clone(&fleet)
        .start_session(ipc::StartSession {
            manifest_id: ipc::ManifestId::carried(&manifest),
            title: None,
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
        })
        .await
        .expect("started");
    Arc::clone(&fleet)
        .send_session_message(ipc::SendSessionMessage {
            session_id: started.id.clone(),
            text: "look it up".into(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        })
        .await
        .expect("taken");
    eventually(|| async { !fleet.hosts().of(started.id.as_str()).state().proposal.holds_back() }).await;
    stand_in.init(0);
    eventually(|| async { fleet.hosts().of(started.id.as_str()).state().turn_began.is_some() }).await;
    stand_in.says(
        0,
        vec![
            DroneEvent::Called { tool: REACHES.into(), call: "c1".into(), detail: CallDetail::of("web docs.rs") },
            DroneEvent::Called { tool: REACHES.into(), call: "c2".into(), detail: CallDetail::of("figma get_design_context") },
        ],
    );
    stand_in.finishes(0, "done");
    eventually(|| async {
        read(&fleet).await.iter().filter(|one| one.kind == TimelineKind::SessionCalled).count() == 2
    })
    .await;

    let events = read(&fleet).await;
    let calls: Vec<_> = events.iter().filter(|one| one.kind == TimelineKind::SessionCalled).collect();
    let seen: Vec<_> = calls
        .iter()
        .map(|one| one.call.as_ref().map(|call| (call.service.as_str(), call.line.as_str())))
        .collect();
    assert_eq!(seen, [Some(("web", "docs.rs")), Some(("figma", "get_design_context"))]);
    assert!(calls.iter().all(|one| one.title.as_deref() == Some("look it up")), "named as the Session was then");
    assert!(calls.iter().all(|one| one.manifest_id.as_ref().map(|id| id.as_str()) == Some(manifest.as_str())));
}
