//! One Check's log on a merge line, read off a real `armada-land/` directory
//! through `observe_land_check`.
//!
//! **What `api` cannot prove from its fake**: that the three names reach the
//! file the runner writes and nothing else, and that the reader follows that
//! file as it grows and stops when the outcome says the Check has ended.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use adapters::land_state::dir::StateDir;
use adapters::land_state::outcome::{
    merge_outcome, CheckRun, CheckState, OutcomePatch, OutcomeState,
};
use api::{Queries, Refusal};
use testkit::{FakeHarness, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fitted_with;
use crate::tests::tmp::TempDir;

const BRANCH: &str = "bridge/check-log-sheet";

fn git(root: &Path, args: &[&str]) {
    let ran = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(ran.status.success(), "git {args:?} failed");
}

/// A clone with a line, and one turn's directory under its `logs/`, as
/// `armada land` makes them.
fn a_turn(root: &Path) -> (StateDir, PathBuf) {
    git(root, &["init", "--quiet"]);
    let state = StateDir::resolve(root).expect("a state directory");
    let turn = state
        .path()
        .join("logs")
        .join("0123456789abcdef")
        .join("2026-10-02T10-00-00.000Z");
    std::fs::create_dir_all(&turn).expect("a turn directory");
    (state, turn)
}

/// The branch's outcome: its state, its turn's logs and each Check's.
fn say(state: &StateDir, said: OutcomeState, logs_in: &Path, checks: &[(&str, CheckState)]) {
    let foundations = logs_in
        .join("foundations.log")
        .to_string_lossy()
        .to_string();
    let checks = checks
        .iter()
        .map(|(name, state)| CheckRun {
            name: (*name).to_string(),
            state: *state,
        })
        .collect();
    merge_outcome(
        state,
        BRANCH,
        said,
        "running screens_test",
        "2026-10-02T10:00:00Z",
        OutcomePatch {
            logs: Some(vec![foundations]),
            checks: Some(checks),
            ..OutcomePatch::default()
        },
    )
    .expect("an outcome");
}

fn append(file: &Path, text: &str) {
    std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file)
        .and_then(|mut opened| opened.write_all(text.as_bytes()))
        .expect("appended");
}

/// **The route's claim.** A Check the runner is writing is read as it grows,
/// a line at a time, and the reader is told it has ended once the outcome
/// says so.
#[tokio::test]
async fn a_merge_line_checks_log_is_read_as_it_grows_and_ends_with_its_check() {
    let home = TempDir::new();
    let (state, turn) = a_turn(home.path());
    let log = turn.join("screens_test.log");
    append(&log, "RUN  v3.2.4\n");
    let running = [
        ("build", CheckState::Passed),
        ("screens_test", CheckState::Running),
    ];
    say(&state, OutcomeState::Gating, &turn, &running);
    let fleet = Arc::new(Fleet::assembled(fitted_with(
        &home,
        FakeWorkProduct::untouched(),
        FakeHarness::that_listens(),
    )));
    let root = fleet
        .list_repositories()
        .await
        .expect("a list")
        .repositories[0]
        .root
        .clone();

    let land = fleet
        .observe_land_check(root.clone(), BRANCH.into(), "screens_test".into())
        .await
        .unwrap_or_else(|_| panic!("a running Check's log opens"));
    assert_eq!(
        (land.root.as_str(), land.name.as_str()),
        (root.as_str(), "screens_test")
    );
    assert!(land.follow.writing(), "the runner is writing it");

    let first = land.follow.read(0, false);
    assert_eq!(first.lines, vec!["RUN  v3.2.4".to_string()]);

    append(&log, " ok src/merge-line.test.ts\n Test Files  21 pa");
    let second = land.follow.read(first.from, false);
    assert_eq!(
        second.lines,
        vec![" ok src/merge-line.test.ts".to_string()],
        "what was appended since, and not the half line still being written"
    );

    say(
        &state,
        OutcomeState::Red,
        &turn,
        &[
            ("build", CheckState::Passed),
            ("screens_test", CheckState::Failed),
        ],
    );
    assert!(
        !land.follow.writing(),
        "the outcome says the Check has ended"
    );
    let last = land.follow.read(second.from, true);
    assert_eq!(last.lines, vec![" Test Files  21 pa".to_string()]);
}

/// **Path confinement.** Only the three names are taken, and they reach only a
/// Check the branch's own turn has started, in that turn's directory under this
/// line's `logs/`. Every other ask is the one refusal, and no file is opened.
#[tokio::test]
async fn only_a_started_check_in_the_outcomes_own_turn_is_opened() {
    let home = TempDir::new();
    let (state, turn) = a_turn(home.path());
    let outside = home.path().join("outside.log");
    std::fs::write(&outside, "a file no Check wrote\n").expect("a file outside");
    std::fs::write(turn.join("screens_test.log"), "RUN\n").expect("a log");
    // A link where a Check's log goes, pointing out of the turn.
    std::os::unix::fs::symlink(&outside, turn.join("build.log")).expect("a planted link");
    say(
        &state,
        OutcomeState::Gating,
        &turn,
        &[
            ("build", CheckState::Passed),
            ("screens_test", CheckState::Running),
            ("desktop_test", CheckState::Waiting),
            ("..", CheckState::Running),
        ],
    );
    let fleet = Arc::new(Fleet::assembled(fitted_with(
        &home,
        FakeWorkProduct::untouched(),
        FakeHarness::that_listens(),
    )));
    let root = fleet
        .list_repositories()
        .await
        .expect("a list")
        .repositories[0]
        .root
        .clone();
    let refused =
        |result: Result<api::LandOutput, Refusal>| matches!(result, Err(Refusal::Unacceptable(_)));

    for (root, branch, check, why) in [
        (
            root.as_str(),
            BRANCH,
            "../../outcomes/x",
            "a path, not a name",
        ),
        (root.as_str(), BRANCH, "..", "a name that leaves the turn"),
        (
            root.as_str(),
            BRANCH,
            "screens_test/../build",
            "two components",
        ),
        (
            root.as_str(),
            BRANCH,
            "rust_test",
            "a Check the turn does not run",
        ),
        (
            root.as_str(),
            BRANCH,
            "desktop_test",
            "a Check still waiting",
        ),
        (root.as_str(), BRANCH, "build", "a link out of the turn"),
        (
            root.as_str(),
            "fleet/another",
            "screens_test",
            "a branch with no outcome",
        ),
        (
            "/etc",
            BRANCH,
            "screens_test",
            "a root this Fleet does not serve",
        ),
    ] {
        let asked = fleet
            .observe_land_check(root.into(), branch.into(), check.into())
            .await;
        assert!(refused(asked), "{why} is refused");
    }

    // A turn the outcome names outside this line's `logs/` is not one.
    let elsewhere = home.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("a directory");
    say(
        &state,
        OutcomeState::Gating,
        &elsewhere,
        &[("screens_test", CheckState::Running)],
    );
    let asked = fleet
        .observe_land_check(root.clone(), BRANCH.into(), "screens_test".into())
        .await;
    assert!(refused(asked), "a turn outside `logs/` is refused");

    // A link put where the log goes after it was found is unreadable, not followed.
    say(
        &state,
        OutcomeState::Gating,
        &turn,
        &[("screens_test", CheckState::Running)],
    );
    let land = fleet
        .observe_land_check(root, BRANCH.into(), "screens_test".into())
        .await
        .unwrap_or_else(|_| panic!("the log opens"));
    std::fs::remove_file(turn.join("screens_test.log")).expect("removed");
    std::os::unix::fs::symlink(&outside, turn.join("screens_test.log")).expect("a link");
    let read = land.follow.read(0, false);
    assert!(read.unreadable && read.lines.is_empty());
}
