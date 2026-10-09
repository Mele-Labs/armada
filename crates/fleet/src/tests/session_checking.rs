//! A Session's `armada check` through a real Fleet: placed by the slot the directory is in, a
//! thread row and a Checks page row that move as the run ends, and its log read by run id.

use api::Sessions;
use ipc::{
    SessionCheckAct, SessionCheckAnswer, SessionCheckCall, SessionCheckState, SessionFact,
    SessionId, SessionReport,
};
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

fn call(act: SessionCheckAct, cwd: &str) -> SessionCheckCall {
    SessionCheckCall {
        act,
        cwd: cwd.into(),
        name: "components_test".into(),
        run: None,
        outcome: None,
        took_ms: None,
        log: None,
    }
}

#[tokio::test]
async fn a_session_check_in_a_held_slot_is_a_run_that_ends_with_its_log() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    let slot = adapter_traits::slot_path(&root, 2);
    fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: SessionId::carried("s1"),
            fact: SessionFact::Started {
                cwd: slot.clone(),
                title: None,
                origin: ipc::SessionOrigin::Bridge,
                mod_version: None,
            },
        })
        .await
        .expect("kept");

    let SessionCheckAnswer { run: Some(run) } = fleet
        .session_check_reported(call(SessionCheckAct::Start, &format!("{slot}/crates")))
        .await
        .expect("begins")
    else {
        panic!("the Session holds the slot");
    };
    let out = fleet.manifest_checks(None).await.expect("lists");
    let [row] = &out.rows[..] else { panic!("one run") };
    assert_eq!(row.state, "running");
    assert_eq!(row.session_id.as_deref(), Some("s1"));
    assert_eq!(row.job_id, None);
    assert_eq!(row.requester, ipc::Requester::session("s1", 2));
    assert_eq!(row.asked_run_id, Some(run as i64));

    let mut end = call(SessionCheckAct::End, &slot);
    end.run = Some(run);
    end.outcome = Some(SessionCheckState::Failed);
    end.took_ms = Some(21_000);
    end.log = Some("$ vitest\nFAIL\n".into());
    fleet.session_check_reported(end).await.expect("ends");

    let out = fleet.manifest_checks(None).await.expect("lists");
    assert_eq!((out.rows[0].state.as_str(), out.rows[0].took_ms), ("failed", Some(21_000)));
    let rows = fleet.rows_of("s1").await.expect("the thread");
    let [ipc::SessionRow::Check { name, state, run: row_run, .. }] = &rows[..] else {
        panic!("one check row, replaced as it ended");
    };
    assert_eq!((name.as_str(), *state, *row_run), ("components_test", SessionCheckState::Failed, run));
    let log = fleet.session_check_output(run).await.expect("reads");
    assert_eq!(log.lines, ["$ vitest", "FAIL"]);
    assert!(log.whole);
}

#[tokio::test]
async fn a_session_check_where_no_session_holds_the_slot_is_nobodys_run() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    for cwd in [adapter_traits::slot_path(&root, 4), root, "/elsewhere".to_string()] {
        let answered = fleet
            .session_check_reported(call(SessionCheckAct::Start, &cwd))
            .await
            .expect("answers");
        assert_eq!(answered, SessionCheckAnswer { run: None });
    }
    assert!(fleet.manifest_checks(None).await.expect("lists").rows.is_empty());
    assert!(fleet.session_check_output(99).await.is_err());
}
