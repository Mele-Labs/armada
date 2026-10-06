//! The merge line served off a real `armada-land/` directory: every state a
//! branch can be in, written by the same functions `armada land` writes with,
//! read back through `GET /merge_lines` — and published when it moves.
//!
//! **The answer is compared with `merge-lines.served.json`**, which Bridge's own
//! test folds into the panel's rows (`packages/screens/src/merge-line.test.ts`),
//! so the two halves are held to one reading of the wire.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use adapters::land_state::dir::StateDir;
use adapters::land_state::outcome::{
    merge_outcome, CheckRun, CheckState, OutcomePatch, OutcomeState, Place, PullRequestSettled,
};
use adapters::land_state::queue::{write_queue_entry, QueueEntry};
use axum::http::StatusCode;
use testkit::{FakeHarness, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fitted_with;
use crate::tests::http::call;
use crate::tests::repositories::served;
use crate::tests::tmp::TempDir;

const LANDED: &str = "29064cc27aaf70cbbb924eee1d2212693540c9ad";
const SHARED: &str = "../../packages/screens/src/fixtures/build/merge-lines.served.json";

fn git(root: &Path, args: &[&str]) {
    let ran = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(ran.status.success(), "git {args:?} failed");
}

fn queue(state: &StateDir, branch: &str, place: Place, pr: Option<u64>) {
    let entry = QueueEntry {
        branch: branch.to_string(),
        pr,
        head: "h".into(),
        tree: "t".into(),
        place,
        worktree: "w".into(),
        nonce: "n".into(),
    };
    write_queue_entry(state, &entry).expect("a queue entry");
}

fn say(
    state: &StateDir,
    branch: &str,
    said: OutcomeState,
    detail: &str,
    patch: OutcomePatch,
) -> PathBuf {
    merge_outcome(state, branch, said, detail, "2026-10-02T10:00:00Z", patch).expect("an outcome");
    state.outcome_path(branch)
}

fn runs(states: &[(&str, CheckState)]) -> Option<Vec<CheckRun>> {
    Some(
        states
            .iter()
            .map(|(name, state)| CheckRun {
                name: (*name).to_string(),
                state: *state,
                started_at: None,
            })
            .collect(),
    )
}

/// Written `age` seconds ago, so which outcomes are newest is the test's to say.
fn aged(path: &Path, age: u64) {
    let file = File::options().write(true).open(path).expect("the outcome");
    file.set_modified(SystemTime::now() - Duration::from_secs(age))
        .expect("an mtime");
}

/// The line `armada land --status` printed on 2 Oct 2026, near enough, and
/// what left it: one of every state.
fn a_line(root: &Path) -> StateDir {
    git(root, &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    let origin = format!("https://{}armada-dev/armada.git", adapters::FORGE_HOST);
    git(root, &["remote", "add", "origin", &origin]);
    let state = StateDir::resolve(root).expect("a state directory");

    queue(&state, "fleet/helm-kills-processes", 100, None);
    let together = |doing: &str, others: &str| format!("{doing} — together with {others}");
    for (branch, place, doing, others) in [
        (
            "docs/wire-lock-signed",
            200,
            "reading verify-foundations against main",
            "worktree-agent-a, fleet/gate-policy-every-run",
        ),
        (
            "worktree-agent-a",
            300,
            "running screens_test (build, screens_test, desktop_test)",
            "docs/wire-lock-signed, fleet/gate-policy-every-run",
        ),
        (
            "fleet/gate-policy-every-run",
            400,
            "merging main (c527f60e09) into fleet/gate-policy-every-run",
            "docs/wire-lock-signed, worktree-agent-a",
        ),
    ] {
        queue(&state, branch, place, None);
        // One member is in its Checks: they cross, and the runner's words for them do not.
        let checks = (branch == "worktree-agent-a")
            .then(|| {
                runs(&[
                    ("build", CheckState::Passed),
                    ("screens_test", CheckState::Running),
                    ("desktop_test", CheckState::Waiting),
                ])
            })
            .flatten();
        say(
            &state,
            branch,
            OutcomeState::Gating,
            &together(doing, others),
            OutcomePatch {
                checks,
                ..OutcomePatch::default()
            },
        );
    }
    queue(&state, "fleet/push-the-base", 500, None);
    say(
        &state,
        "fleet/push-the-base",
        OutcomeState::Merging,
        "pushing the merge onto main",
        // Kept on disk from the gate, and not served past it.
        OutcomePatch {
            checks: runs(&[("build", CheckState::Passed)]),
            ..OutcomePatch::default()
        },
    );
    queue(&state, "fleet/read-in-cluster-membership", 600, Some(1770));
    say(
        &state,
        "fleet/read-in-cluster-membership",
        OutcomeState::Waiting,
        "in line",
        OutcomePatch::default(),
    );

    let landed = say(
        &state,
        "bridge/land-board-reads-plainly",
        OutcomeState::Landed,
        "landed",
        OutcomePatch {
            merge_commit: Some(LANDED.into()),
            pr: Some(1769),
            pr_settled: Some(PullRequestSettled::Merged),
            ..OutcomePatch::default()
        },
    );
    aged(&landed, 30);
    // Landed once, then red: the old merge commit is still in the file.
    say(
        &state,
        "fleet/pulse-log-rows",
        OutcomeState::Landed,
        "landed",
        OutcomePatch {
            merge_commit: Some("0000000000000000000000000000000000000000".into()),
            ..OutcomePatch::default()
        },
    );
    let red = say(
        &state,
        "fleet/pulse-log-rows",
        OutcomeState::Red,
        "red with main merged in",
        OutcomePatch {
            pr: Some(1768),
            failed: Some(vec!["desktop_test".into(), "screens_test".into()]),
            checks: runs(&[
                ("build", CheckState::Passed),
                ("desktop_test", CheckState::Failed),
                ("screens_test", CheckState::TimedOut),
            ]),
            ..OutcomePatch::default()
        },
    );
    aged(&red, 20);
    let conflict = say(
        &state,
        "bridge/overview-strip-width",
        OutcomeState::Conflict,
        "conflict",
        OutcomePatch {
            conflicts: Some(vec!["apps/desktop/src/renderer/src/App.tsx".into()]),
            ..OutcomePatch::default()
        },
    );
    aged(&conflict, 10);
    // Older than the three, so past `off` but still landed; and a turn a killed runner left, never drawn.
    let older = say(
        &state,
        "fleet/an-older-landing",
        OutcomeState::Landed,
        "landed",
        OutcomePatch::default(),
    );
    aged(&older, 3_600);
    // Sent back four days ago, past `SENT_BACK_FOR`: not drawn.
    let stale = say(
        &state,
        "fleet/a-red-days-ago",
        OutcomeState::Red,
        "red",
        OutcomePatch {
            failed: Some(vec!["rust_test".into()]),
            ..OutcomePatch::default()
        },
    );
    aged(&stale, 4 * 24 * 3_600);
    say(
        &state,
        "fleet/a-runner-killed-mid-turn",
        OutcomeState::Gating,
        "reading verify-foundations",
        OutcomePatch::default(),
    );
    state
}

/// The served answer with this machine's root and forge spelled as the shared file spells them.
fn as_shared(body: &[u8], root: &Path) -> ipc::MergeLines {
    let text = String::from_utf8_lossy(body)
        .replace(&root.to_string_lossy().to_string(), "/repo")
        .replace(
            &format!("https://{}", adapters::FORGE_HOST),
            "https://git.example/",
        );
    ipc::decode("merge lines", text.as_bytes()).expect("merge lines")
}

fn shared() -> ipc::MergeLines {
    let at = Path::new(env!("CARGO_MANIFEST_DIR")).join(SHARED);
    ipc::decode("merge lines", &std::fs::read(at).expect("the shared file")).expect("merge lines")
}

#[tokio::test]
async fn fleet_serves_the_line_on_disk_in_place_order_with_what_landed_and_what_was_sent_back() {
    let home = TempDir::new();
    let state = a_line(home.path());
    let fleet = Arc::new(Fleet::assembled(fitted_with(
        &home,
        FakeWorkProduct::untouched(),
        FakeHarness::that_listens(),
    )));

    let (status, body) = call(&served(&fleet), "GET", "/merge_lines", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(
        as_shared(&body, home.path()),
        shared(),
        "{}",
        String::from_utf8_lossy(&body)
    );

    // Read, and nothing else: no runner took the turn, and no lock was made.
    assert!(!state.path().join("runner.lock").exists());
}

#[tokio::test]
async fn a_repository_nobody_landed_in_has_no_line_and_gains_no_directory() {
    let home = TempDir::new();
    git(home.path(), &["init", "--quiet"]);
    let fleet = Arc::new(Fleet::assembled(fitted_with(
        &home,
        FakeWorkProduct::untouched(),
        FakeHarness::that_listens(),
    )));

    let (status, body) = call(&served(&fleet), "GET", "/merge_lines", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        ipc::decode::<ipc::MergeLines>("merge lines", &body)
            .expect("lines")
            .lines,
        []
    );
    assert!(!home.path().join(".git").join("armada-land").exists());
}

#[tokio::test]
async fn a_line_that_moves_on_disk_is_published_whole() {
    let home = TempDir::new();
    let state = a_line(home.path());
    let fleet = Arc::new(Fleet::assembled(fitted_with(
        &home,
        FakeWorkProduct::untouched(),
        FakeHarness::that_listens(),
    )));
    let mut events = fleet.events().subscribe();
    let reading = crate::merge_lines::keep_reading(
        Arc::clone(&fleet),
        fleet.events(),
        Duration::from_millis(50),
        |_| {},
    );

    // Past the first read, which is the baseline and publishes nothing.
    tokio::time::sleep(Duration::from_millis(200)).await;
    say(
        &state,
        "fleet/a-stop",
        OutcomeState::Stopped,
        "withdrawn",
        OutcomePatch::default(),
    );

    let moved = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(api::Next::Send(delivered)) = events.next().await {
                if let ipc::Event::MergeLinesChanged(lines) = delivered.event {
                    return lines;
                }
            }
        }
    })
    .await
    .expect("published once the file moved");
    reading.abort();

    let off: Vec<(String, ipc::LandState)> = moved.lines[0]
        .off
        .iter()
        .map(|one| (one.branch.clone(), one.state))
        .collect();
    assert_eq!(
        off,
        [
            ("fleet/a-stop".to_string(), ipc::LandState::Stopped),
            (
                "bridge/overview-strip-width".to_string(),
                ipc::LandState::Conflict
            ),
            ("fleet/pulse-log-rows".to_string(), ipc::LandState::Red),
        ],
        "the newest first, and the landing pushed past the three"
    );
    let sent_back: Vec<&str> = moved.lines[0]
        .sent_back
        .iter()
        .map(|one| one.branch.as_str())
        .collect();
    assert_eq!(
        sent_back,
        [
            "fleet/a-stop",
            "bridge/overview-strip-width",
            "fleet/pulse-log-rows"
        ],
        "a stop is sent back, newest first, and nothing landed is"
    );
    let landed: Vec<&str> = moved.lines[0]
        .landed
        .iter()
        .map(|one| one.branch.as_str())
        .collect();
    assert_eq!(
        landed,
        ["bridge/land-board-reads-plainly", "fleet/an-older-landing"],
        "the stop pushes no landing out"
    );
}

/// **When a Check began is the line's own record**, so a Check that has started
/// says when and one still waiting says nothing.
#[tokio::test]
async fn a_check_the_line_started_says_when_and_one_waiting_does_not() {
    let home = TempDir::new();
    git(home.path(), &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    let origin = format!("https://{}armada-dev/armada.git", adapters::FORGE_HOST);
    git(home.path(), &["remote", "add", "origin", &origin]);
    let state = StateDir::resolve(home.path()).expect("a state directory");
    queue(&state, "fleet/a-branch", 100, None);
    let started = |name: &str, state, at: Option<&str>| CheckRun {
        name: name.to_string(),
        state,
        started_at: at.map(str::to_string),
    };
    say(
        &state,
        "fleet/a-branch",
        OutcomeState::Gating,
        "running screens_test",
        OutcomePatch {
            checks: Some(vec![
                started("build", CheckState::Passed, Some("2026-10-06T10:00:00Z")),
                started("screens_test", CheckState::Running, Some("2026-10-06T10:01:00Z")),
                started("desktop_test", CheckState::Waiting, None),
            ]),
            ..OutcomePatch::default()
        },
    );
    let fleet = Arc::new(Fleet::assembled(fitted_with(
        &home,
        FakeWorkProduct::untouched(),
        FakeHarness::that_listens(),
    )));

    let (_, body) = call(&served(&fleet), "GET", "/merge_lines", "").await;
    let lines = as_shared(&body, home.path());
    let checks = &lines.lines[0].line[0].checks;
    let at: Vec<Option<&str>> = checks
        .iter()
        .map(|check| check.started_at.as_ref().map(|at| at.as_str()))
        .collect();
    assert_eq!(
        at,
        [Some("2026-10-06T10:00:00Z"), Some("2026-10-06T10:01:00Z"), None]
    );
}
