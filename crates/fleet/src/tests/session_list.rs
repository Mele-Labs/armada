//! The list of every Job, drawn off reads taken under one hold of the store lock.

use ipc::{BuildStopId, SessionFact, SessionId, SessionReport};
use store::{AttachmentState, Holder, KeptAttachment, KeptBuild, TimelineEntry};

use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;
use api::Sessions;
use testkit::FakeWorkProduct;

fn report(session: &str, fact: SessionFact) -> SessionReport {
    SessionReport { harness: "a_harness".into(), session_id: SessionId::carried(session), fact }
}

fn started(cwd: &str) -> SessionFact {
    SessionFact::Started {
        cwd: cwd.into(),
        title: Some("a job".into()),
        origin: ipc::SessionOrigin::Terminal,
        mod_version: None,
    }
}

fn at(minute: u32) -> String {
    format!("2026-10-20T10:{minute:02}:00.000Z")
}

fn entry(minute: u32, kind: &str, subject_kind: &str, id: &str, repo: &str) -> TimelineEntry {
    TimelineEntry {
        at: at(minute),
        kind: kind.to_string(),
        subject_kind: subject_kind.to_string(),
        subject_id: id.to_string(),
        subject_url: None,
        title: None,
        repo: Some(repo.to_string()),
        tone: "idle".to_string(),
        says: "x".to_string(),
        detail: None,
        items: Vec::new(),
        service: None,
        service_name: None,
        to_id: None,
        live: false,
        over: false,
    }
}

/// **A list draws each row exactly as the row alone is drawn**, including a repository's forecast,
/// which a list reads once and shares between its rows.
#[tokio::test]
async fn a_list_draws_every_row_as_that_row_alone_is_drawn() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let one = fleet.repositories().first().expect("a served repository").clone();
    let (manifest, root) = (one.manifest().id().as_str().to_string(), one.root().to_string());
    for session in ["live", "over", "landed"] {
        fleet.report_session(report(session, started(&root))).await.unwrap();
    }
    fleet.report_session(report("over", SessionFact::Ended { reason: "other".into() })).await.unwrap();
    {
        let mut store = fleet.store().lock().await;
        // Two landed pull requests that both walked leave a forecast, and "landed" is one of their Sessions.
        for (holder, number) in [("landed", "12"), ("history", "13")] {
            store
                .attach(
                    &KeptAttachment {
                        holder: Holder::session(holder),
                        kind: "pr".to_string(),
                        manifest_id: manifest.clone(),
                        target: number.to_string(),
                        state: AttachmentState::Spent,
                        detail: Default::default(),
                        since: at(0),
                        changed_at: at(0),
                    },
                    false,
                )
                .expect("attached");
        }
        for e in [
            entry(1, "session.walked", "session", "landed", &manifest),
            entry(2, "pr.opened", "pull_request", "12", &manifest),
            entry(10, "pr.merged", "pull_request", "12", &manifest),
            entry(11, "session.walked", "session", "history", &manifest),
            entry(12, "pr.opened", "pull_request", "13", &manifest),
            entry(20, "pr.merged", "pull_request", "13", &manifest),
        ] {
            store.append_timeline(&e).expect("appended");
        }
        for session in ["live", "over", "landed"] {
            let build = KeptBuild {
                begun_at: at(0),
                first_edit_at: Some(at(1)),
                last_edit_at: Some(at(2)),
                last_tool_at: Some(at(2)),
                plan_left_at: None,
                tasks_at: None,
            };
            store.keep_session_build(session, &build).expect("kept");
        }
    }

    let listed = fleet.list_sessions(None, None, None).await.unwrap().sessions;
    assert_eq!(listed.len(), 3);
    let store = fleet.store().lock().await;
    for record in &listed {
        let kept = store.session(record.id.as_str()).unwrap().expect("kept");
        assert_eq!(record, &fleet.ledger_row(&store, &kept).unwrap(), "{} is drawn as it is alone", record.id.as_str());
    }
    let build = |id: &str| listed.iter().find(|one| one.id.as_str() == id).and_then(|one| one.build.clone()).expect("a build");
    assert_eq!(build("live").forecast, [BuildStopId::Walk, BuildStopId::Pr], "a Session not landed is shown the usual road");
    assert_eq!(build("over").forecast, [BuildStopId::Walk, BuildStopId::Pr], "and so is an ended one");
}

/// An ended Job's row is not kept apart from its Session: it reads what the Session holds now.
#[tokio::test]
async fn an_ended_row_is_current_after_its_session_changes() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("a served repository").root().to_string();
    fleet.report_session(report("over", started(&root))).await.unwrap();
    fleet.report_session(report("over", SessionFact::Ended { reason: "other".into() })).await.unwrap();

    let before = fleet.list_sessions(None, None, None).await.unwrap().sessions;
    assert_eq!(before[0].title.as_deref(), Some("a job"));
    fleet
        .rename_session(ipc::RenameSession { session_id: SessionId::carried("over"), title: "renamed".into() })
        .await
        .expect("renamed");
    let after = fleet.list_sessions(None, None, None).await.unwrap().sessions;
    assert_eq!(after[0].title.as_deref(), Some("renamed"));
}
