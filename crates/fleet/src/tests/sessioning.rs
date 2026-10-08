//! The session ledger through a real Fleet: a harness reports facts, Fleet
//! resolves the directory to a repository and a pool slot, keeps what the
//! session holds, answers the two queries and publishes a row when it changed.

use std::time::Duration;

use api::{Next, Sessions, Subscription};
use ipc::{
    AttachmentNamed, AttachmentReport, AttachmentState, Event, HolderKind, SessionFact, SessionId,
    SessionRecord, SessionReport, SessionState, SessionUsage,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::sessioning::{placed, Placed};
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

fn roots() -> Vec<(String, String)> {
    vec![
        ("armada".into(), "/repos/armada".into()),
        ("inner".into(), "/repos/armada/vendor/inner".into()),
    ]
}

#[test]
fn a_directory_is_placed_in_the_deepest_repository_that_holds_it() {
    let at = |cwd: &str| placed(cwd, &roots());
    assert_eq!(
        at("/repos/armada/crates/ipc"),
        Placed {
            manifest: Some("armada".into()),
            slot: None
        }
    );
    assert_eq!(
        at("/repos/armada/vendor/inner/src").manifest.as_deref(),
        Some("inner")
    );
    assert_eq!(at("/repos/armada-two"), Placed::default(), "a longer name");
    assert_eq!(at("/elsewhere"), Placed::default());
}

#[test]
fn a_slot_is_read_off_the_path_whole_and_only_where_it_is_a_slot() {
    let slot = |cwd: &str| placed(cwd, &roots()).slot;
    assert_eq!(slot("/repos/armada/.armada/slots/slot-3"), Some(3));
    assert_eq!(
        slot("/repos/armada/.armada/slots/slot-12/crates/ipc"),
        Some(12)
    );
    assert_eq!(slot("/repos/armada/.armada/slots/slot-3x"), None);
    assert_eq!(slot("/repos/armada/.armada/slots/slot-"), None);
    assert_eq!(slot("/repos/armada/.armada/worktrees/01JOB"), None);
}

fn report(session: &str, fact: SessionFact) -> SessionReport {
    SessionReport {
        harness: "a_harness".into(),
        session_id: SessionId::carried(session),
        fact,
    }
}

fn started(cwd: &str) -> SessionFact {
    SessionFact::Started {
        cwd: cwd.into(),
        title: Some("fix the ledger".into()),
        origin: ipc::SessionOrigin::Terminal,
        mod_version: None,
    }
}

fn attached(kind: &str, target: &str) -> SessionFact {
    SessionFact::Attached {
        attachment: AttachmentReport {
            kind: kind.into(),
            target: target.into(),
            detail: Default::default(),
        },
    }
}

fn held(record: &SessionRecord, kind: &str) -> Vec<(String, AttachmentState)> {
    record
        .attachments
        .iter()
        .filter(|one| one.kind == kind)
        .map(|one| (one.target.clone(), one.state))
        .collect()
}

async fn next_session(watching: &mut Subscription) -> SessionRecord {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::SessionChanged(record) = delivered.event {
                        return record;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("a session.changed arrives")
}

/// The one repository the test Fleet serves, and the path of its slot 2.
fn served(fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>) -> (String, String) {
    let one = fleet.repositories().first().expect("a served repository");
    (
        one.manifest().id().as_str().to_string(),
        one.root().to_string(),
    )
}

#[tokio::test]
async fn a_session_that_starts_in_a_slot_holds_it_and_giving_it_up_is_recorded() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (manifest, root) = served(&fleet);
    let slot = adapter_traits::slot_path(&root, 2);

    let record = fleet
        .report_session(report("s1", started(&slot)))
        .await
        .expect("kept");
    assert_eq!(record.state, SessionState::Live);
    assert_eq!(
        record.manifest_id.as_ref().map(|id| id.as_str()),
        Some(manifest.as_str())
    );
    assert_eq!(
        held(&record, "slot"),
        [("2".into(), AttachmentState::Standing)]
    );

    let moved = fleet
        .report_session(report("s1", SessionFact::Moved { cwd: root.clone() }))
        .await
        .expect("moved");
    assert_eq!(
        held(&moved, "slot"),
        [("2".into(), AttachmentState::GivenBack)]
    );
}

#[tokio::test]
async fn a_branch_is_one_at_a_time_and_a_pull_request_is_not() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (_, root) = served(&fleet);
    fleet
        .report_session(report("s1", started(&root)))
        .await
        .unwrap();
    for fact in [
        attached("branch", "fleet/a"),
        attached("branch", "fleet/b"),
        attached("pr", "12"),
        attached("pr", "13"),
    ] {
        fleet.report_session(report("s1", fact)).await.unwrap();
    }
    let record = fleet
        .report_session(report("s1", SessionFact::TurnCompleted))
        .await
        .unwrap();

    assert_eq!(
        held(&record, "branch"),
        [
            ("fleet/b".into(), AttachmentState::Standing),
            ("fleet/a".into(), AttachmentState::GivenBack)
        ]
    );
    assert!(held(&record, "pr")
        .iter()
        .all(|(_, state)| *state == AttachmentState::Standing));
    assert!(record.last_turn_at.is_some());
}

#[tokio::test]
async fn ending_a_session_gives_back_what_it_held_and_a_spent_attachment_stays_spent() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (_, root) = served(&fleet);
    let slot = adapter_traits::slot_path(&root, 1);
    fleet
        .report_session(report("s1", started(&slot)))
        .await
        .unwrap();
    fleet
        .report_session(report("s1", attached("pr", "12")))
        .await
        .unwrap();
    fleet
        .report_session(report(
            "s1",
            SessionFact::Settled {
                attachment: AttachmentNamed {
                    kind: "pr".into(),
                    target: "12".into(),
                },
                state: AttachmentState::Spent,
            },
        ))
        .await
        .unwrap();
    let ended = fleet
        .report_session(report(
            "s1",
            SessionFact::Ended {
                reason: "prompt_input_exit".into(),
            },
        ))
        .await
        .unwrap();

    assert_eq!(ended.state, SessionState::Ended);
    assert_eq!(
        held(&ended, "slot"),
        [("1".into(), AttachmentState::GivenBack)]
    );
    assert_eq!(held(&ended, "pr"), [("12".into(), AttachmentState::Spent)]);
}

#[tokio::test]
async fn a_fact_that_repeats_the_ledger_publishes_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (_, root) = served(&fleet);
    let mut watching = fleet.events().subscribe();

    fleet
        .report_session(report("s1", started(&root)))
        .await
        .unwrap();
    assert_eq!(next_session(&mut watching).await.id.as_str(), "s1");
    fleet
        .report_session(report("s1", attached("branch", "fleet/a")))
        .await
        .unwrap();
    assert_eq!(held(&next_session(&mut watching).await, "branch").len(), 1);

    // The same branch again, then a turn: only the turn is news.
    fleet
        .report_session(report("s1", attached("branch", "fleet/a")))
        .await
        .unwrap();
    fleet
        .report_session(report("s1", SessionFact::TurnCompleted))
        .await
        .unwrap();
    let next = next_session(&mut watching).await;
    assert!(
        next.last_turn_at.is_some(),
        "the repeated branch published nothing"
    );
}

#[tokio::test]
async fn usage_keeps_what_was_reported_and_a_later_report_adds_to_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    fleet
        .report_session(report("s1", started("/nowhere")))
        .await
        .unwrap();
    fleet
        .report_session(report(
            "s1",
            SessionFact::Measured {
                usage: SessionUsage {
                    context_tokens: Some(40_000),
                    context_window: Some(200_000),
                    cost_micros: None,
                },
            },
        ))
        .await
        .unwrap();
    let record = fleet
        .report_session(report(
            "s1",
            SessionFact::Measured {
                usage: SessionUsage {
                    context_tokens: None,
                    context_window: None,
                    cost_micros: Some(2_000_000),
                },
            },
        ))
        .await
        .unwrap();
    assert_eq!(record.usage.context_tokens, Some(40_000));
    assert_eq!(record.usage.cost_micros, Some(2_000_000));
    assert_eq!(
        record.manifest_id, None,
        "a directory in no served repository"
    );
}

#[tokio::test]
async fn the_queries_find_a_session_by_what_it_holds_and_say_who_holds_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (manifest, root) = served(&fleet);
    for (session, branch, pr) in [("old", "fleet/a", "12"), ("new", "fleet/a", "13")] {
        fleet
            .report_session(report(session, started(&root)))
            .await
            .unwrap();
        fleet
            .report_session(report(session, attached("branch", branch)))
            .await
            .unwrap();
        fleet
            .report_session(report(session, attached("pr", pr)))
            .await
            .unwrap();
    }
    fleet
        .report_session(report(
            "old",
            SessionFact::Ended {
                reason: "other".into(),
            },
        ))
        .await
        .unwrap();

    let listed = fleet
        .list_sessions(None, Some("#13".into()), None)
        .await
        .unwrap();
    assert_eq!(listed.sessions.len(), 1);
    assert_eq!(listed.sessions[0].id.as_str(), "new");

    let live = fleet
        .list_sessions(None, None, Some(SessionState::Live))
        .await
        .unwrap();
    assert_eq!(live.sessions.len(), 1);

    let owners = fleet
        .who_owns(
            "branch".into(),
            "fleet/a".into(),
            Some(ipc::ManifestId::carried(&manifest)),
        )
        .await
        .unwrap();
    let who: Vec<(&str, AttachmentState)> = owners
        .holders
        .iter()
        .map(|one| (one.holder.id.as_str(), one.attachment.state))
        .collect();
    assert_eq!(
        who,
        [
            ("new", AttachmentState::Standing),
            ("old", AttachmentState::GivenBack)
        ]
    );
    assert!(owners
        .holders
        .iter()
        .all(|one| one.holder.kind == HolderKind::Session));
    assert_eq!(owners.holders[0].title.as_deref(), Some("fix the ledger"));

    let none = fleet
        .who_owns("pr".into(), "99".into(), None)
        .await
        .unwrap();
    assert!(none.holders.is_empty(), "nothing holds it is an answer");
}

#[tokio::test]
async fn a_report_naming_nothing_is_refused_and_keeps_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    for bad in [
        report("", started("/x")),
        report("s1", attached(" ", "12")),
        report("s1", attached("pr", "")),
    ] {
        assert!(matches!(
            fleet.report_session(bad).await,
            Err(api::Refusal::Unacceptable(_))
        ));
    }
    assert!(fleet
        .list_sessions(None, None, None)
        .await
        .unwrap()
        .sessions
        .is_empty());
    assert!(matches!(
        fleet.who_owns("".into(), "12".into(), None).await,
        Err(api::Refusal::Unacceptable(_))
    ));
}

fn titled(title: &str, named: bool) -> SessionFact {
    SessionFact::Titled {
        title: title.into(),
        named,
    }
}

#[tokio::test]
async fn a_name_a_person_gave_beats_the_first_prompt_and_the_next_one_beats_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (_, root) = served(&fleet);
    fleet
        .report_session(report("s1", started(&root)))
        .await
        .expect("kept");

    let by_hand = fleet
        .report_session(report("s1", titled("ledger work", true)))
        .await
        .expect("renamed in the terminal");
    assert_eq!(by_hand.title.as_deref(), Some("ledger work"));

    let prompt = fleet
        .report_session(report("s1", titled("a later first line", false)))
        .await
        .expect("kept");
    assert_eq!(
        prompt.title.as_deref(),
        Some("ledger work"),
        "a prompt fills a gap only"
    );

    let mut watching = fleet.events().subscribe();
    let renamed = fleet
        .rename_session(ipc::RenameSession {
            session_id: SessionId::carried("s1"),
            title: "  the   Bridge name ".into(),
        })
        .await
        .expect("renamed in Bridge");
    assert_eq!(renamed.title.as_deref(), Some("the Bridge name"));
    assert_eq!(
        next_session(&mut watching).await.title.as_deref(),
        Some("the Bridge name")
    );

    let again = fleet
        .report_session(report("s1", titled("typed in the terminal", true)))
        .await
        .expect("kept");
    assert_eq!(again.title.as_deref(), Some("typed in the terminal"));
}

#[tokio::test]
async fn renaming_to_nothing_or_a_stranger_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (_, root) = served(&fleet);
    fleet
        .report_session(report("s1", started(&root)))
        .await
        .expect("kept");
    for (id, title) in [("s1", "   "), ("nobody", "a name")] {
        let rename = ipc::RenameSession {
            session_id: SessionId::carried(id),
            title: title.into(),
        };
        assert!(
            fleet.rename_session(rename).await.is_err(),
            "{id} {title:?}"
        );
    }
    let kept = fleet
        .report_session(report("s1", SessionFact::TurnCompleted))
        .await
        .expect("kept");
    assert_eq!(kept.title.as_deref(), Some("fix the ledger"));
}

fn artifact(target: &str, form: &str, title: &str) -> SessionFact {
    SessionFact::Attached {
        attachment: AttachmentReport {
            kind: "artifact".into(),
            target: target.into(),
            detail: [("form".into(), form.into()), ("title".into(), title.into())].into(),
        },
    }
}

#[tokio::test]
async fn the_artifacts_a_session_made_are_kept_with_their_form_and_one_of_each_address() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (_, root) = served(&fleet);
    fleet
        .report_session(report("s1", started(&root)))
        .await
        .unwrap();
    for fact in [
        artifact("https://example.com/artifact/p1", "page", "Spike"),
        artifact("/repo/docs/spike.md", "file", "spike.md"),
        artifact("https://example.com/artifact/d1", "doc", "Write-up"),
        artifact("https://example.com/artifact/d1", "doc", "Write-up"),
    ] {
        fleet.report_session(report("s1", fact)).await.unwrap();
    }
    let record = fleet
        .report_session(report("s1", SessionFact::TurnCompleted))
        .await
        .unwrap();

    let mut kept: Vec<_> = record
        .attachments
        .iter()
        .filter(|one| one.kind == "artifact")
        .map(|one| (one.target.as_str(), one.detail.get("form").map(String::as_str)))
        .collect();
    kept.sort();
    assert_eq!(
        kept,
        [
            ("/repo/docs/spike.md", Some("file")),
            ("https://example.com/artifact/d1", Some("doc")),
            ("https://example.com/artifact/p1", Some("page")),
        ]
    );
}

#[tokio::test]
async fn a_page_a_session_shows_is_one_window_attachment_and_a_row_each_time() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (_, root) = served(&fleet);
    fleet
        .report_session(report("s1", started(&root)))
        .await
        .unwrap();
    let show = |url: &str| ipc::ShowWindow {
        url: url.into(),
        title: Some("Findings".into()),
        session_id: Some(SessionId::carried("s1")),
    };
    let mut watching = fleet.events().subscribe();
    fleet
        .show_window(None, show("http://localhost:5173/"))
        .await
        .expect("shown");
    let record = fleet
        .show_window(None, show("http://localhost:5173/"))
        .await
        .expect("shown again");
    let windows: Vec<_> = record
        .attachments
        .iter()
        .filter(|one| one.detail.get("form").map(String::as_str) == Some("window"))
        .collect();
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].target, "http://localhost:5173/");
    assert_eq!(windows[0].detail.get("title").map(String::as_str), Some("Findings"));
    let mut rows = 0;
    tokio::time::timeout(Duration::from_secs(5), async {
        while rows < 2 {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if matches!(
                        delivered.event,
                        Event::SessionRow(ipc::SessionRowChanged {
                            row: ipc::SessionRow::Window { .. },
                            ..
                        })
                    ) {
                        rows += 1;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("two window rows arrive");
    assert_eq!(rows, 2, "a row each time, so a window opens each time");
}

#[tokio::test]
async fn an_address_that_is_not_http_or_a_call_with_no_session_shows_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (_, root) = served(&fleet);
    fleet
        .report_session(report("s1", started(&root)))
        .await
        .unwrap();
    for (url, session) in [
        ("file:///etc/passwd", Some("s1")),
        ("javascript:alert(1)", Some("s1")),
        ("http://", Some("s1")),
        ("https://a b", Some("s1")),
        ("ftp://example.com/", Some("s1")),
        ("http://:8080/", Some("s1")),
        ("https://example.com", None),
        ("https://example.com", Some("nobody")),
    ] {
        let show = ipc::ShowWindow {
            url: url.into(),
            title: None,
            session_id: session.map(SessionId::carried),
        };
        assert!(fleet.show_window(None, show).await.is_err(), "{url} {session:?}");
    }
}
