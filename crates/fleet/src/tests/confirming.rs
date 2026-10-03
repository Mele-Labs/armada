//! A gate's red confirmed before it is ruled against the step: each failing
//! test alone, then the whole Check alone. `crate::confirming`.
//!
//! Job 3 on 3 Oct 2026 is the case: `desktop_test` timed out beside the step's
//! other Checks on all three attempts, passed alone, and the Job stopped out of
//! retries over work that was fine. The scripts here print nextest's own
//! summary shape, which is what `checks_runner::failing_tests` reads, and are
//! files in the worktree for `crate::tests::repeated_failures`' reason.

use std::sync::Arc;

use config::ResolvedWorkflow;
use core_model::{Job, ResolvedCheck};
use ipc::CheckRunBy;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, OneTest, Sketch};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, diff_evidence, fitted_over, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// Red the first time the whole suite runs, green every time after, and each
/// whole run counted in `runs`.
fn red_once_naming(tests: &[&str]) -> String {
    let mut script = String::from(
        "echo whole >> runs\nif [ -f .ran ]; then exit 0; fi\ntouch .ran\n\
         echo 'Summary [   0.010s] 9 tests run: 0 passed, 9 failed, 0 skipped'\n",
    );
    for (n, test) in tests.iter().enumerate() {
        script.push_str(&format!(
            "echo 'FAIL [   0.010s] ({}/9) nt {test}'\n",
            n + 1
        ));
    }
    script.push_str("exit 1\n");
    script
}

/// One test by name: written down in `alone`, then the exit it is given.
fn one_test_exiting(code: u8) -> String {
    format!("echo \"$1\" >> alone\nexit {code}\n")
}

fn step() -> Sketch<'static> {
    Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "suite",
            run: "sh check.sh",
            expect_exit_code: 0,
            when: &[],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }
}

/// Retried twice, so a red with budget left is handed back rather than stopped.
fn gated(one_test: bool) -> ResolvedWorkflow {
    match one_test {
        true => testkit::retried_and_testing_one(
            &[step()],
            2,
            &[OneTest {
                check: "suite",
                run: "sh one.sh {}",
            }],
        ),
        false => testkit::retried(&[step()], 2),
    }
}

fn a_fleet(home: &TempDir, workflow: ResolvedWorkflow) -> Arc<Fixture> {
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        FakeHarness::running("/bin/sh", &["-c", "sleep 30"]),
        FakeVcs::new(),
    );
    fittings.starting().workflows = one(workflow);
    Arc::new(Fleet::assembled(fittings))
}

fn worktree_of(home: &TempDir, job: &Job) -> std::path::PathBuf {
    crate::tests::daemon::spec_held(home, job)
        .expect("a legal spec")
        .worktree_path()
        .into()
}

/// Start the Job with both scripts in its worktree, submit once, and rule.
async fn ruled_once(
    home: &TempDir,
    workflow: ResolvedWorkflow,
    check: &str,
    one: &str,
) -> (Arc<Fixture>, Job, crate::turning::Turned) {
    let (fleet, job, turned, _) = heard_ruling_once(home, workflow, check, one).await;
    (fleet, job, turned)
}

/// [`ruled_once`], and every event Fleet published from its start to the
/// ruling's end.
async fn heard_ruling_once(
    home: &TempDir,
    workflow: ResolvedWorkflow,
    check: &str,
    one: &str,
) -> (Arc<Fixture>, Job, crate::turning::Turned, Vec<ipc::Event>) {
    let fleet = a_fleet(home, workflow);
    let mut watching = fleet.events().subscribe();
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .expect("a proposed Job");
    worktree_directory(home, &job);
    let at = worktree_of(home, &job);
    std::fs::write(at.join("check.sh"), check).expect("the check written");
    std::fs::write(at.join("one.sh"), one).expect("the one test written");
    dispatched(&fleet, job.id()).await.expect("an approved Job");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("taken");
    let turned = fleet.turn().await.expect("a turn");
    let mut seen = Vec::new();
    while let Ok(Some(api::Next::Send(delivered))) =
        tokio::time::timeout(std::time::Duration::from_millis(200), watching.next()).await
    {
        seen.push(delivered.event);
    }
    (fleet, job, turned, seen)
}

/// What the step's live row for `suite` read in each `job.checking` that
/// carried one, in order, with repeats run together.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Read {
    Waiting,
    /// Started and not finished, and whether it named a live log.
    Running {
        log: bool,
    },
    Ended(core_model::CheckOutcome),
}

fn suite_reads(seen: &[ipc::Event]) -> Vec<Read> {
    let mut reads: Vec<Read> = seen
        .iter()
        .filter_map(|event| match event {
            ipc::Event::JobChecking(one) => one.checking.as_ref(),
            _ => None,
        })
        .filter_map(|checking| checking.checks.iter().find(|row| row.name == "suite"))
        .map(|row| match (&row.ran, &row.started_at) {
            (Some(ran), _) => Read::Ended(ran.outcome.domain()),
            (None, Some(_)) => Read::Running {
                log: row.output_path.is_some(),
            },
            (None, None) => Read::Waiting,
        })
        .collect();
    reads.dedup();
    reads
}

/// Everything the row read after the gate's first run came back red.
fn after_the_red(seen: &[ipc::Event]) -> Vec<Read> {
    let reads = suite_reads(seen);
    let red = reads
        .iter()
        .position(|read| *read == Read::Ended(core_model::CheckOutcome::Failed))
        .unwrap_or_else(|| panic!("the first run read red: {reads:?}"));
    reads[red + 1..].to_vec()
}

/// Each line one of the worktree's tally files holds; none where it is absent.
fn lines_in(home: &TempDir, job: &Job, file: &str) -> Vec<String> {
    std::fs::read_to_string(worktree_of(home, job).join(file))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn log_of(home: &TempDir, job: &Job) -> Vec<String> {
    let root = home.path().to_string_lossy().into_owned();
    crate::journal::read_from(&root, &job.handle(), 0)
        .notes
        .into_iter()
        .map(|note| note.msg)
        .collect()
}

fn handed_back(turned: &crate::turning::Turned) -> bool {
    matches!(turned.ruled(), Some(Ruling::HandedBack { .. }))
}

/// **The claim.** Every failing test passes alone, so the red was the
/// machine's: the whole Check runs again alone, the step is ruled on that run
/// and passes, nothing goes back to the Drone, and the Job's log and its retro
/// record both say it happened.
#[tokio::test]
async fn a_red_whose_tests_pass_alone_is_run_again_alone_and_advances() {
    let home = TempDir::new();
    let (fleet, job, turned) = ruled_once(
        &home,
        gated(true),
        &red_once_naming(&["tests::slow_one", "tests::slow_two"]),
        &one_test_exiting(0),
    )
    .await;

    assert!(
        matches!(
            turned.ruled(),
            Some(Ruling::Finished { .. } | Ruling::Advanced { .. })
        ),
        "the step is ruled on the run alone"
    );
    assert_eq!(
        lines_in(&home, &job, "alone"),
        vec!["tests::slow_one", "tests::slow_two"],
        "each failing test ran alone, one at a time"
    );
    assert_eq!(
        lines_in(&home, &job, "runs").len(),
        2,
        "and the whole Check again"
    );
    let ruling = turned.ruled().expect("a ruling");
    let alone = ruling.output()[0]
        .alone
        .as_ref()
        .expect("said on the output");
    assert!(alone.one_by_one);
    assert_eq!(alone.failing, vec!["tests::slow_one", "tests::slow_two"]);

    assert!(log_of(&home, &job)
        .iter()
        .any(|msg| msg == crate::retro::lines::A_RED_RUN_ALONE));
    let record = fleet
        .retro_record(job.id())
        .await
        .expect("the record")
        .record;
    let row = record
        .failed_checks
        .iter()
        .find(|row| row.name == "suite" && row.run == CheckRunBy::Gate)
        .expect("the retro reads it as the gate's");
    let produced = row.produced.as_deref().unwrap_or_default();
    assert!(produced.contains("each passed run alone"), "{produced}");
    assert!(produced.contains("alone passed"), "{produced}");
}

/// A test that fails alone too is the work's: handed back as before, with no
/// second whole run and nothing in the log.
#[tokio::test]
async fn a_red_whose_test_fails_alone_is_handed_back_as_before() {
    let home = TempDir::new();
    let (_fleet, job, turned) = ruled_once(
        &home,
        gated(true),
        &red_once_naming(&["tests::broken"]),
        &one_test_exiting(1),
    )
    .await;

    assert!(
        handed_back(&turned),
        "{:?}",
        turned.ruled().map(Ruling::checks)
    );
    assert_eq!(lines_in(&home, &job, "alone"), vec!["tests::broken"]);
    assert_eq!(
        lines_in(&home, &job, "runs").len(),
        1,
        "never run whole again"
    );
    assert!(turned.ruled().expect("a ruling").output()[0]
        .alone
        .is_none());
    assert!(!log_of(&home, &job)
        .iter()
        .any(|msg| msg == crate::retro::lines::A_RED_RUN_ALONE));
}

/// **The owner's call on 3 Oct 2026**: while a red is run again alone, its
/// row reads running with its live log, never red, and ends at what the run
/// alone came to.
#[tokio::test]
async fn a_red_run_again_alone_reads_running_until_the_run_alone_ends() {
    let home = TempDir::new();
    let (_fleet, _job, _turned, seen) = heard_ruling_once(
        &home,
        gated(true),
        &red_once_naming(&["tests::slow_one", "tests::slow_two"]),
        &one_test_exiting(0),
    )
    .await;

    let after = after_the_red(&seen);
    assert!(
        after.contains(&Read::Running { log: true }),
        "running again, with its live log: {after:?}"
    );
    assert_eq!(
        after.last(),
        Some(&Read::Ended(core_model::CheckOutcome::Passed)),
        "{after:?}"
    );
    assert!(
        after[..after.len() - 1]
            .iter()
            .all(|read| !matches!(read, Read::Ended(_))),
        "nothing ended before the run alone did: {after:?}"
    );
}

/// A test that fails alone stands the red: the row reads running while it
/// runs, then red again, and nothing between.
#[tokio::test]
async fn a_red_whose_test_fails_alone_reads_running_then_red() {
    let home = TempDir::new();
    let (_fleet, _job, _turned, seen) = heard_ruling_once(
        &home,
        gated(true),
        &red_once_naming(&["tests::broken"]),
        &one_test_exiting(1),
    )
    .await;

    let after = after_the_red(&seen);
    assert!(after.contains(&Read::Running { log: true }), "{after:?}");
    assert_eq!(
        after.last(),
        Some(&Read::Ended(core_model::CheckOutcome::Failed)),
        "{after:?}"
    );
    assert!(
        after[..after.len() - 1]
            .iter()
            .all(|read| !matches!(read, Read::Ended(_))),
        "{after:?}"
    );
}

/// A Check with no `one_test` cannot be confirmed, so nothing changes.
#[tokio::test]
async fn a_red_on_a_check_with_no_one_test_is_handed_back_as_before() {
    let home = TempDir::new();
    let (_fleet, job, turned) = ruled_once(
        &home,
        gated(false),
        &red_once_naming(&["tests::slow_one"]),
        &one_test_exiting(0),
    )
    .await;

    assert!(handed_back(&turned));
    assert!(lines_in(&home, &job, "alone").is_empty());
    assert_eq!(lines_in(&home, &job, "runs").len(), 1);
}

/// A red whose output names no test cannot be confirmed either.
#[tokio::test]
async fn a_red_naming_no_test_is_handed_back_as_before() {
    let home = TempDir::new();
    let (_fleet, job, turned) = ruled_once(
        &home,
        gated(true),
        "echo whole >> runs\nif [ -f .ran ]; then exit 0; fi\ntouch .ran\nexit 1\n",
        &one_test_exiting(0),
    )
    .await;

    assert!(handed_back(&turned));
    assert_eq!(lines_in(&home, &job, "runs").len(), 1);
}

/// Past [`crate::confirming::ONE_BY_ONE`] failing tests, none runs one by one:
/// the whole Check alone is the only run, and it decides.
#[tokio::test]
async fn past_the_cap_only_the_whole_check_runs_again_alone() {
    let home = TempDir::new();
    let many: Vec<String> = (0..=crate::confirming::ONE_BY_ONE)
        .map(|n| format!("tests::slow_{n}"))
        .collect();
    let named: Vec<&str> = many.iter().map(String::as_str).collect();
    let (_fleet, job, turned) = ruled_once(
        &home,
        gated(true),
        &red_once_naming(&named),
        &one_test_exiting(1),
    )
    .await;

    assert!(!handed_back(&turned));
    assert!(
        lines_in(&home, &job, "alone").is_empty(),
        "no test ran by name"
    );
    assert_eq!(lines_in(&home, &job, "runs").len(), 2);
    let ruling = turned.ruled().expect("a ruling");
    assert!(!ruling.output()[0].alone.as_ref().expect("said").one_by_one);
}

/// Alone is every place: asked for whole, which `crate::places` clamps to the
/// limit in force, and without the prerequisites the gate already met.
#[test]
fn a_check_run_alone_asks_for_every_place() {
    let workflow = gated(true);
    let check = workflow.steps()[0].checks()[0].clone();
    let Some(ResolvedCheck::ManifestCheck {
        places, requires, ..
    }) = crate::confirming::holding_every_place(&check, None)
    else {
        panic!("a Manifest Check");
    };
    assert_eq!(places, std::num::NonZeroU32::MAX);
    assert!(requires.is_empty());
}
