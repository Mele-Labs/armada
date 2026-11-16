//! A checkout whose `main` cannot fast-forward because it holds an untracked file `origin/main`
//! tracks: the report names it, and a change asked to clear it stashes or deletes it, then
//! fast-forwards `main`, before the wrapper starts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use ipc::{BuildSource, ChangeFleetBuild, CheckoutBlocked, ClearBlocked};

use crate::building::{Building, NotChanged};
use crate::tests::tmp::TempDir;

const IN_THE_WAY: &str = ".armada/workflows/bug.toml";

fn git(root: &Path, args: &[&str]) -> String {
    let done = Command::new("git")
        .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "{args:?}: {}", String::from_utf8_lossy(&done.stderr));
    String::from_utf8_lossy(&done.stdout).trim().to_string()
}

/// An origin whose main is one commit past the checkout's, that commit tracking `IN_THE_WAY`, which
/// the checkout holds untracked with other words in it. The checkout's wrapper records its arguments.
fn blocked_rig() -> (TempDir, PathBuf, PathBuf) {
    let dir = TempDir::new();
    let origin = dir.path().join("origin");
    fs::create_dir_all(origin.join("scripts")).unwrap();
    git(&origin, &["init", "--quiet", "--initial-branch=main"]);
    let wrapper = origin.join("scripts/restart-build");
    fs::write(&wrapper, "#!/bin/sh\necho \"$@\" > ran\n").unwrap();
    Command::new("chmod").arg("+x").arg(&wrapper).status().expect("chmod");
    git(&origin, &["add", "."]);
    git(&origin, &["commit", "--quiet", "-m", "first"]);
    let checkout = dir.path().join("checkout");
    git(dir.path(), &["clone", "--quiet", origin.to_str().unwrap(), "checkout"]);
    for (key, value) in [("user.name", "armada"), ("user.email", "armada@example.invalid"), ("commit.gpgsign", "false")] {
        git(&checkout, &["config", key, value]);
    }
    fs::create_dir_all(origin.join(".armada/workflows")).unwrap();
    fs::write(origin.join(IN_THE_WAY), "origin's\n").unwrap();
    git(&origin, &["add", "."]);
    git(&origin, &["commit", "--quiet", "-m", "track the workflow"]);
    git(&checkout, &["fetch", "--quiet", "origin"]);
    fs::create_dir_all(checkout.join(".armada/workflows")).unwrap();
    fs::write(checkout.join(IN_THE_WAY), "the checkout's\n").unwrap();
    (dir, origin, checkout)
}

async fn ran(checkout: &Path) -> String {
    for _ in 0..100 {
        if let Ok(text) = fs::read_to_string(checkout.join("ran")) {
            if text.ends_with('\n') {
                return text;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the wrapper never ran");
}

#[tokio::test]
async fn the_report_names_the_untracked_files_origin_main_tracks_and_how_far_behind_main_is() {
    let (_dir, _origin, checkout) = blocked_rig();
    let support = TempDir::new();
    let report = Building::default().report(Some(&checkout), support.path(), SystemTime::now()).await.expect("a build");
    assert_eq!(report.blocked, Some(CheckoutBlocked { behind: 1, files: vec![IN_THE_WAY.to_string()] }));
}

#[tokio::test]
async fn a_checkout_level_with_origin_main_is_not_blocked() {
    let (_dir, _origin, checkout) = blocked_rig();
    fs::remove_file(checkout.join(IN_THE_WAY)).unwrap();
    git(&checkout, &["pull", "--quiet", "--ff-only", "origin", "main"]);
    fs::write(checkout.join("notes.txt"), "mine\n").unwrap();
    let support = TempDir::new();
    let report = Building::default().report(Some(&checkout), support.path(), SystemTime::now()).await.expect("a build");
    assert_eq!(report.blocked, None);
}

#[tokio::test]
async fn a_change_that_stashes_keeps_the_file_in_the_stash_and_fast_forwards_main() {
    let (_dir, _origin, checkout) = blocked_rig();
    let support = TempDir::new();
    let asked = ChangeFleetBuild { build: BuildSource::Preview, adopt: false, clear: Some(ClearBlocked::Stash) };
    Building::default().change(Some(&checkout), support.path(), &asked, &[], SystemTime::now()).await.expect("started");
    let stashed = git(&checkout, &["stash", "list"]);
    assert!(stashed.contains("armada: before updating main"), "{stashed}");
    assert_eq!(git(&checkout, &["show", "--format=", "--name-only", "stash@{0}^3"]), IN_THE_WAY);
    assert_eq!(git(&checkout, &["rev-parse", "main"]), git(&checkout, &["rev-parse", "origin/main"]));
    assert_eq!(fs::read_to_string(checkout.join(IN_THE_WAY)).unwrap(), "origin's\n");
    assert_eq!(ran(&checkout).await.trim(), "preview");
}

#[tokio::test]
async fn a_change_that_deletes_removes_the_file_and_fast_forwards_main() {
    let (_dir, _origin, checkout) = blocked_rig();
    let support = TempDir::new();
    let asked = ChangeFleetBuild { build: BuildSource::Main, adopt: false, clear: Some(ClearBlocked::Delete) };
    Building::default().change(Some(&checkout), support.path(), &asked, &[], SystemTime::now()).await.expect("started");
    assert_eq!(git(&checkout, &["stash", "list"]), "");
    assert_eq!(git(&checkout, &["rev-parse", "main"]), git(&checkout, &["rev-parse", "origin/main"]));
    assert_eq!(fs::read_to_string(checkout.join(IN_THE_WAY)).unwrap(), "origin's\n");
    assert_eq!(ran(&checkout).await.trim(), "main");
}

/// A pull that fails starts nothing, and says git's line.
#[tokio::test]
async fn a_change_whose_pull_fails_starts_nothing() {
    let (dir, origin, checkout) = blocked_rig();
    fs::rename(&origin, dir.path().join("gone")).unwrap();
    let support = TempDir::new();
    let asked = ChangeFleetBuild { build: BuildSource::Main, adopt: false, clear: Some(ClearBlocked::Delete) };
    let refused = Building::default().change(Some(&checkout), support.path(), &asked, &[], SystemTime::now()).await;
    assert!(matches!(&refused, Err(NotChanged::NotStarted(said)) if !said.is_empty()), "{refused:?}");
    assert!(!support.path().join("restart-build.status").exists());
    assert!(!checkout.join("ran").exists(), "nothing was started");
}
