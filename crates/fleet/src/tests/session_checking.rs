//! A Session's `armada check` through a real Fleet: placed by the slot the directory is in and the
//! lease there, a thread row and a Checks page row that move as the run ends, and its log read by
//! run id.

use api::Sessions;
use ipc::{
    SessionCheckAct, SessionCheckAnswer, SessionCheckCall, SessionCheckState, SessionCheckStop, SessionFact,
    SessionId, SessionReport,
};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;

use crate::tests::daemon::a_fleet;
use crate::tests::slot_leases::{leased, By};
use crate::tests::tmp::TempDir;

fn call(act: SessionCheckAct, cwd: &str) -> SessionCheckCall {
    SessionCheckCall {
        act,
        cwd: cwd.into(),
        name: "components_test".into(),
        target: None,
        run: None,
        outcome: None,
        took_ms: None,
        log: None,
    }
}

/// A run started with a target carries it on its row, through the end; a `--changed` run only
/// knows its target at the end, and a plain run has none.
#[tokio::test]
async fn a_session_check_row_carries_what_the_run_was_narrowed_to() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    let slot = leased(&root, 2, "fleet/topic", By::Job("s1"));
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
    let start = |target: Option<&str>| {
        let mut start = call(SessionCheckAct::Start, &slot);
        start.target = target.map(str::to_string);
        fleet.session_check_reported(start)
    };
    let run_of = |answer: SessionCheckAnswer| answer.run.expect("the Session holds the slot");
    let filtered = run_of(start(Some("store::reads_the_last_row")).await.expect("begins"));
    let changed = run_of(start(None).await.expect("begins"));
    let plain = run_of(start(None).await.expect("begins"));

    let mut end = call(SessionCheckAct::End, &slot);
    end.run = Some(changed);
    end.outcome = Some(SessionCheckState::Passed);
    end.target = Some("--changed fleet, ipc".into());
    fleet.session_check_reported(end).await.expect("ends");

    let rows = fleet.rows_of("s1").await.expect("the thread");
    let target_of = |run: u64| {
        rows.iter()
            .find_map(|row| match row {
                ipc::SessionRow::Check { run: at, target, .. } if *at == run => Some(target.clone()),
                _ => None,
            })
            .expect("a row for the run")
    };
    assert_eq!(target_of(filtered).as_deref(), Some("store::reads_the_last_row"));
    assert_eq!(target_of(changed).as_deref(), Some("--changed fleet, ipc"));
    assert_eq!(target_of(plain), None);
}

async fn a_running_check(fleet: &Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>, root: &str) -> (String, u64) {
    let slot = leased(root, 2, "fleet/topic", By::Job("s1"));
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
    let SessionCheckAnswer { run: Some(run), .. } = fleet
        .session_check_reported(call(SessionCheckAct::Start, &slot))
        .await
        .expect("begins")
    else {
        panic!("held");
    };
    (slot, run)
}

#[tokio::test]
async fn a_running_session_checks_follow_yields_lines_before_end() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    let (slot, run) = a_running_check(&fleet, &root).await;

    let mut chunk = call(SessionCheckAct::Chunk, &slot);
    chunk.run = Some(run);
    chunk.log = Some("compiling\nrunning 3 tests\npartial".into());
    fleet.session_check_reported(chunk).await.expect("heard");

    let live = fleet.session_check_observed(run).await.expect("observes");
    assert!(live.follow.writing(), "the run is still out");
    let read = live.follow.read(0, false);
    assert_eq!(read.lines, ["compiling", "running 3 tests"], "whole lines, before end");

    let mut more = call(SessionCheckAct::Chunk, &slot);
    more.run = Some(run);
    more.log = Some(" line\nlast\n".into());
    fleet.session_check_reported(more).await.expect("heard");
    let next = live.follow.read(read.from, false);
    assert_eq!(next.lines, ["partial line", "last"]);

    let mut end = call(SessionCheckAct::End, &slot);
    end.run = Some(run);
    end.outcome = Some(SessionCheckState::Passed);
    end.log = Some("$ t\nok\n".into());
    fleet.session_check_reported(end).await.expect("ends");
    assert!(!live.follow.writing(), "an ended run is final");
    let over = fleet.session_check_observed(run).await.expect("an ended run is served too");
    assert_eq!(over.follow.read(0, true).lines, ["$ t", "ok"]);
}

#[tokio::test]
async fn each_stop_route_is_answered_to_the_runs_next_chunk_and_force_outranks() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    let (slot, run) = a_running_check(&fleet, &root).await;
    let beat = || {
        let mut chunk = call(SessionCheckAct::Chunk, &slot);
        chunk.run = Some(run);
        chunk
    };
    assert_eq!(fleet.session_check_reported(beat()).await.expect("heard").stop, None);

    let said = fleet.session_check_stopped(run, false).await.expect("asks");
    assert_eq!(said.stop, Some(SessionCheckStop::Graceful));
    assert_eq!(
        fleet.session_check_reported(beat()).await.expect("heard").stop,
        Some(SessionCheckStop::Graceful)
    );

    let said = fleet.session_check_stopped(run, true).await.expect("asks");
    assert_eq!(said.stop, Some(SessionCheckStop::Force));
    // A graceful ask after a force one does not take it back.
    let said = fleet.session_check_stopped(run, false).await.expect("asks");
    assert_eq!(said.stop, Some(SessionCheckStop::Force));
    assert_eq!(
        fleet.session_check_reported(beat()).await.expect("heard").stop,
        Some(SessionCheckStop::Force)
    );

    let mut end = call(SessionCheckAct::End, &slot);
    end.run = Some(run);
    end.outcome = Some(SessionCheckState::Failed);
    fleet.session_check_reported(end).await.expect("ends");
    assert!(fleet.session_check_stopped(run, false).await.is_err(), "an ended run takes no stop");
    assert!(fleet.session_check_stopped(999, true).await.is_err());
}

/// A failed run's row carries where its log says it failed, read in the slot as it ends; a passed
/// run's carries none, though its log names the same file.
#[tokio::test]
async fn a_failed_runs_row_carries_its_blame_and_a_passed_runs_does_not() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    let (slot, failed) = a_running_check(&fleet, &root).await;
    let file = std::path::Path::new(&slot).join("crates/store/src/reads.rs");
    std::fs::create_dir_all(file.parent().expect("a folder")).expect("its folder");
    std::fs::write(&file, "").expect("the file");
    let log = format!("thread 'main' panicked at {slot}/crates/store/src/reads.rs:88:31:\nboom\n");
    let SessionCheckAnswer { run: Some(passed), .. } =
        fleet.session_check_reported(call(SessionCheckAct::Start, &slot)).await.expect("begins")
    else {
        panic!("held");
    };
    for (run, outcome) in [(failed, SessionCheckState::Failed), (passed, SessionCheckState::Passed)] {
        let mut end = call(SessionCheckAct::End, &slot);
        end.run = Some(run);
        end.outcome = Some(outcome);
        end.log = Some(log.clone());
        fleet.session_check_reported(end).await.expect("ends");
    }

    let rows = fleet.rows_of("s1").await.expect("the thread");
    let blame_of = |run: u64| {
        rows.iter()
            .find_map(|row| match row {
                ipc::SessionRow::Check { run: at, blame, .. } if *at == run => Some(blame.clone()),
                _ => None,
            })
            .expect("a row for the run")
    };
    assert_eq!(
        blame_of(failed),
        Some(ipc::CheckBlame { path: "crates/store/src/reads.rs".into(), line: Some(88) })
    );
    assert_eq!(blame_of(passed), None);
}
