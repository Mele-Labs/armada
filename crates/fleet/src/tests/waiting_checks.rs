//! A Job's Checks reported while they wait: a gate Check behind another thing
//! in its gate, one waiting for a machine slot, a Drone's run not yet started,
//! and a merge-line Check naming the Job whose branch it is. Protocol 23.52.
//!
//! Real commands and a real `flock`, for `tests::underway`'s reason. Every
//! fake wait here ends when its test does.

use std::sync::Arc;
use std::time::Duration;

use core_model::{Attempt, Prerequisite, ResolvedCheck, StepId};

use crate::checking::ran;
use crate::headroom::{Bytes, Headroom, Spare};
use crate::manifest_checks::{queued, waiting_rows};
use crate::places::{Asking, ChecksAtOnce, Places, Room};
use crate::tests::gate::Stopped;
use crate::tests::headroom::Plentiful;
use crate::tests::tmp::TempDir;
use crate::underway::{Announcing, Underway};

const JOB: &str = "01JOB";
const STEP: &str = "implement";

fn check(name: &str, run: &str, requires: Vec<Prerequisite>) -> ResolvedCheck {
    ResolvedCheck::ManifestCheck {
        manifest_dir: String::new(),
        name: name.to_string(),
        run: run.to_string(),
        expect_exit_code: 0,
        when: None,
        requires,
        narrow: None,
        one_test: None,
        runs_at: core_model::RunsAt::Everywhere,
        places: std::num::NonZeroU32::MIN,
        width: None,
        runner: None,
    }
}

fn announcing(underway: &Underway, repo: &TempDir) -> Announcing {
    Announcing::on(
        ipc::JobId::carried(JOB),
        StepId::new(STEP),
        Attempt::FIRST,
        underway.clone(),
        api::Broadcaster::new(),
        Arc::new(Stopped),
        &repo.path().display().to_string(),
        "7-the-job",
    )
}

/// What the Checks read says of this Job's gate right now.
fn states(underway: &Underway) -> Vec<(String, String)> {
    let live = underway.live(&ipc::JobId::carried(JOB));
    waiting_rows(&ipc::JobId::carried(JOB), "7-the-job", "The job", &live)
        .into_iter()
        .map(|row| (row.name, row.state))
        .collect()
}

/// Looked at until `want` holds, or the test fails with what it last saw.
async fn until(underway: &Underway, want: impl Fn(&[(String, String)]) -> bool) {
    let mut seen = Vec::new();
    for _ in 0..300 {
        seen = states(underway);
        if want(&seen) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("never read as wanted; last read {seen:?}");
}

fn at(rows: &[(String, String)], name: &str) -> Option<String> {
    rows.iter()
        .find(|(one, _)| one == name)
        .map(|(_, state)| state.clone())
}

/// **A gate Check blocked by what it `requires` lists as waiting**, and as
/// running once the prerequisite is met.
#[tokio::test]
async fn a_gate_check_behind_its_requires_lists_as_waiting_then_running() {
    let repo = TempDir::new();
    let go = repo.path().join("go");
    let script = repo.path().join("wait.sh");
    std::fs::write(
        &script,
        "until [ -e \"$1\" ]; do kill -0 $PPID 2>/dev/null || exit 0; sleep 0.1; done\n",
    )
    .expect("a script");
    let build = Prerequisite::resolved(
        "build".to_string(),
        format!("/bin/sh {} {}", script.display(), go.display()),
    );
    let checks = [check("test", "/bin/sleep 0.5", vec![build])];
    let underway = Underway::default();
    let writer = announcing(&underway, &repo);
    let room = Room::ignoring_the_machine(ChecksAtOnce::of(4));
    let (ports, stop) = (
        std::collections::BTreeMap::new(),
        crate::checking::Stop::never(),
    );
    let batch = ran(
        &checks,
        &[],
        false,
        crate::checking::Reading::Whole,
        repo.path(),
        Duration::from_secs(30),
        &room,
        &writer,
        &ports,
        &[],
        None,
        &stop,
        None,
        Attempt::FIRST,
        None,
        None,
    );
    let watching = async {
        until(&underway, |rows| {
            at(rows, "test").as_deref() == Some("waiting")
        })
        .await;
        std::fs::write(&go, "").expect("the prerequisite is met");
        until(&underway, |rows| {
            at(rows, "test").as_deref() == Some("running")
        })
        .await;
    };
    tokio::join!(batch, watching);
}

/// **A Check blocked on the machine's slots lists as waiting, then running.**
/// The one slot is held by a flock this test took in its own directory.
#[tokio::test]
async fn a_check_waiting_for_a_machine_slot_lists_as_waiting_then_running() {
    let repo = TempDir::new();
    let slots_dir = TempDir::new();
    let slots = checks_runner::CheckSlots::at(slots_dir.path(), 1);
    let elsewhere = slots.try_take(1).expect("writable").expect("free");
    let places = Places::on_the_machine(
        ChecksAtOnce::of(4),
        checks_runner::CheckWidth::read(1),
        Some(slots),
    );
    let room = Room::sharing(
        &places,
        Asking::Gate,
        Arc::new(Plentiful),
        Headroom::of(Spare::percent(15), Bytes::gibibytes(10)),
        checks_runner::CheckWidth::read(1),
    );
    let checks = [check("suite", "/bin/sleep 0.5", Vec::new())];
    let underway = Underway::default();
    let writer = announcing(&underway, &repo);
    let (ports, stop) = (
        std::collections::BTreeMap::new(),
        crate::checking::Stop::never(),
    );
    let batch = ran(
        &checks,
        &[],
        false,
        crate::checking::Reading::Whole,
        repo.path(),
        Duration::from_secs(30),
        &room,
        &writer,
        &ports,
        &[],
        None,
        &stop,
        None,
        Attempt::FIRST,
        None,
        None,
    );
    let watching = async {
        until(&underway, |rows| {
            at(rows, "suite").as_deref() == Some("waiting")
        })
        .await;
        // Held for longer than one look, so it is the slot and not the start.
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(at(&states(&underway), "suite").as_deref(), Some("waiting"));
        drop(elsewhere);
        until(&underway, |rows| {
            at(rows, "suite").as_deref() == Some("running")
        })
        .await;
    };
    tokio::join!(batch, watching);
}

/// A Drone's run that has started none of its Checks is queued, and reads so.
#[test]
fn a_drones_run_that_has_started_nothing_is_queued() {
    let unstarted = |started: bool| ipc::CheckUnderway {
        name: "suite".to_string(),
        started_at: started.then(|| ipc::Instant::carried("2026-10-07T10:00:00Z")),
        took_ms: None,
        ran: None,
        output_path: None,
        stopped_by: None,
        waiting_behind: None,
        places: None,
    };
    let run = |started: bool| {
        (
            ipc::StepId::carried(STEP),
            ipc::ChecksUnderway {
                attempt: 1,
                requester: ipc::Requester::outside(),
                checks: vec![unstarted(false), unstarted(started)],
            },
        )
    };
    assert!(queued(Some(&run(false)), STEP));
    assert!(!queued(Some(&run(true)), STEP), "one has started");
    assert!(!queued(Some(&run(false)), "review"), "another step's run");
    assert!(!queued(None, STEP));
}

/// **A merge-line Check's requester names the Job whose branch it is**, so a Job's
/// Checks tab can narrow the line's Checks to it. A branch no Job owns is left alone.
#[test]
fn a_merge_line_checks_requester_names_the_job_that_owns_the_branch() {
    let entry = |branch: &str| ipc::MergeLineEntry {
        branch: branch.to_string(),
        place: Some(1),
        pull_request: None,
        state: ipc::LandState::Gating,
        doing: None,
        batch: None,
        merge_commit: None,
        failed: Vec::new(),
        conflicts: Vec::new(),
        checks: vec![ipc::MergeLineCheck {
            name: "ipc_test".to_string(),
            requester: ipc::Requester::merge_line(branch),
            started_at: None,
            state: ipc::LandCheckState::Waiting,
        }],
    };
    let lines = ipc::MergeLines {
        lines: vec![ipc::MergeLine {
            root: "/repo".to_string(),
            line: vec![entry("fleet/mine"), entry("fleet/theirs")],
            off: Vec::new(),
            landed: Vec::new(),
            sent_back: Vec::new(),
            hub: None,
        }],
    };
    let jobs = [(
        "fleet/mine".to_string(),
        ipc::JobId::carried("01JOB"),
        "7-mine".to_string(),
    )];
    let named = crate::merge_lines::naming_jobs(lines, &jobs, &[]);
    let [mine, theirs] = &named.lines[0].line[..] else {
        panic!("two branches");
    };
    assert_eq!(
        mine.checks[0].requester,
        ipc::Requester {
            job_id: Some(ipc::JobId::carried("01JOB")),
            ..ipc::Requester::merge_line("fleet/mine").with_handle("7-mine")
        }
    );
    assert_eq!(
        theirs.checks[0].requester,
        ipc::Requester::merge_line("fleet/theirs")
    );
}

/// **A branch queued behind another lists the Manifest's Checks as waiting, naming
/// its Job**, until the line gates it and writes its own rows.
#[test]
fn a_branch_queued_behind_another_lists_the_declared_checks_as_waiting_with_its_job() {
    let entry = |branch: &str, state: ipc::LandState| ipc::MergeLineEntry {
        branch: branch.to_string(),
        place: Some(1),
        pull_request: None,
        state,
        doing: None,
        batch: None,
        merge_commit: None,
        failed: Vec::new(),
        conflicts: Vec::new(),
        checks: Vec::new(),
    };
    let lines = ipc::MergeLines {
        lines: vec![ipc::MergeLine {
            root: "/repo".to_string(),
            line: vec![
                entry("fleet/ahead", ipc::LandState::Gating),
                entry("fleet/mine", ipc::LandState::Waiting),
            ],
            off: Vec::new(),
            landed: Vec::new(),
            sent_back: Vec::new(),
            hub: None,
        }],
    };
    let jobs = [
        (
            "fleet/ahead".to_string(),
            ipc::JobId::carried("01AHEAD"),
            "6-ahead".to_string(),
        ),
        (
            "fleet/mine".to_string(),
            ipc::JobId::carried("01JOB"),
            "7-mine".to_string(),
        ),
    ];
    let declared = [(
        "/repo".to_string(),
        vec!["ipc_test".to_string(), "fleet_test".to_string()],
    )];
    let named = crate::merge_lines::naming_jobs(lines, &jobs, &declared);
    let [ahead, mine] = &named.lines[0].line[..] else {
        panic!("two branches");
    };
    assert!(
        ahead.checks.is_empty(),
        "the line writes a gating branch's own"
    );
    let listed: Vec<(&str, ipc::LandCheckState)> = mine
        .checks
        .iter()
        .map(|check| (check.name.as_str(), check.state))
        .collect();
    assert_eq!(
        listed,
        [
            ("ipc_test", ipc::LandCheckState::Waiting),
            ("fleet_test", ipc::LandCheckState::Waiting)
        ]
    );
    assert_eq!(mine.checks[0].requester.kind, "merge_line");
    assert_eq!(
        mine.checks[0].requester.job_id,
        Some(ipc::JobId::carried("01JOB"))
    );
    assert_eq!(mine.checks[0].requester.handle.as_deref(), Some("7-mine"));
}
