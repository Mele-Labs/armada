//! The build Fleet runs on, read off the files `scripts/restart` leaves, and
//! the restart started detached. The wrapper is a stand-in script that writes
//! down what it was run with; the real one is `scripts/test_restart_build.py`'s.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

use ipc::{BuildPosition, BuildSource, ChangeFleetBuild};

use crate::building::{checkout_of, Building, NotChanged, NotOffered};
use crate::tests::tmp::TempDir;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// A checkout whose wrapper records its arguments in `ran`, and a support folder.
fn rig() -> (TempDir, TempDir) {
    let checkout = TempDir::new();
    let scripts = checkout.path().join("scripts");
    fs::create_dir_all(&scripts).expect("scripts");
    let wrapper = scripts.join("restart-build");
    fs::write(&wrapper, "#!/bin/sh\necho \"$@\" > ran\n").expect("a wrapper");
    Command::new("chmod").arg("+x").arg(&wrapper).status().expect("chmod");
    (checkout, TempDir::new())
}

fn git(root: &Path, args: &[&str]) -> String {
    let done = Command::new("git")
        .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "{args:?}: {}", String::from_utf8_lossy(&done.stderr));
    String::from_utf8_lossy(&done.stdout).trim().to_string()
}

#[tokio::test]
async fn a_fleet_nobody_restarted_runs_main_with_no_commit() {
    let (checkout, support) = rig();
    let report = Building::default()
        .report(Some(checkout.path()), support.path(), SystemTime::now())
        .await
        .expect("a build");
    assert_eq!(report.on, BuildSource::Main);
    assert_eq!(report.commit, None);
    assert_eq!(report.position, None);
    assert_eq!(report.restarting, None);
}

#[tokio::test]
async fn the_source_file_says_preview_for_the_preview_tree_and_nothing_else() {
    let (checkout, support) = rig();
    let building = Building::default();
    fs::write(support.path().join("restart-source"), "/Users/user/armada/.armada/preview\n").unwrap();
    let report = building
        .report(Some(checkout.path()), support.path(), SystemTime::now())
        .await
        .expect("the preview");
    assert_eq!(report.on, BuildSource::Preview);

    fs::write(support.path().join("restart-source"), "/Users/user/elsewhere\n").unwrap();
    let refused = building
        .report(Some(checkout.path()), support.path(), SystemTime::now())
        .await;
    assert_eq!(refused, Err(NotOffered::OtherTree(String::from("/Users/user/elsewhere"))));
}

#[tokio::test]
async fn no_checkout_with_the_wrapper_is_no_build() {
    let support = TempDir::new();
    let bare = TempDir::new();
    assert_eq!(checkout_of([bare.path().to_str().unwrap()]), None);
    let refused = Building::default().report(None, support.path(), SystemTime::now()).await;
    assert_eq!(refused, Err(NotOffered::NoWrapper));
}

#[tokio::test]
async fn the_position_is_counted_against_origin_main_for_the_recorded_commit() {
    let (checkout, support) = rig();
    let root = checkout.path();
    git(root, &["init", "--quiet", "--initial-branch=main"]);
    git(root, &["commit", "--quiet", "--allow-empty", "-m", "first"]);
    git(root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(root, &["commit", "--quiet", "--allow-empty", "-m", "ahead of main"]);
    let built = git(root, &["rev-parse", "HEAD"]);
    fs::write(support.path().join("restart-commit"), format!("{built}\n")).unwrap();

    let report = Building::default()
        .report(Some(root), support.path(), SystemTime::now())
        .await
        .expect("a build");
    assert_eq!(report.commit.as_deref(), Some(built.as_str()));
    assert_eq!(report.position, Some(BuildPosition { ahead: 1, behind: 0 }));
}

#[tokio::test]
async fn a_recorded_commit_that_is_not_an_object_name_is_ignored() {
    let (checkout, support) = rig();
    fs::write(support.path().join("restart-commit"), "not a commit\n").unwrap();
    let report = Building::default()
        .report(Some(checkout.path()), support.path(), SystemTime::now())
        .await
        .expect("a build");
    assert_eq!(report.commit, None);
}

#[tokio::test]
async fn a_restart_under_way_is_reported_until_it_is_old_enough_to_be_a_dead_wrapper() {
    let (checkout, support) = rig();
    let status = support.path().join("restart-build.status");
    fs::write(&status, "running\npreview\n").unwrap();
    let building = Building::default();
    let now = SystemTime::now();
    let report = building.report(Some(checkout.path()), support.path(), now).await.unwrap();
    assert_eq!(report.restarting, Some(BuildSource::Preview));

    let later = now + Duration::from_secs(21 * 60);
    let report = building.report(Some(checkout.path()), support.path(), later).await.unwrap();
    assert_eq!(report.restarting, None, "twenty minutes on, nothing is working");
}

#[tokio::test]
async fn a_failure_is_reported_only_for_the_build_it_failed_on() {
    let (checkout, support) = rig();
    let status = support.path().join("restart-build.status");
    let building = Building::default();
    fs::write(&status, format!("failed\nmain\n{SHA}\nrefusing, drone-3's Drone is working\n")).unwrap();

    fs::write(support.path().join("restart-commit"), format!("{SHA}\n")).unwrap();
    let report = building.report(Some(checkout.path()), support.path(), SystemTime::now()).await.unwrap();
    assert_eq!(report.failed.as_deref(), Some("refusing, drone-3's Drone is working"));

    fs::write(support.path().join("restart-commit"), "f".repeat(40)).unwrap();
    let report = building.report(Some(checkout.path()), support.path(), SystemTime::now()).await.unwrap();
    assert_eq!(report.failed, None, "the build moved on, so the failure is history");
}

async fn wait_for(path: &Path) -> String {
    for _ in 0..100 {
        if let Ok(text) = fs::read_to_string(path) {
            return text;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{} was never written", path.display());
}

#[tokio::test]
async fn a_change_starts_the_wrapper_with_the_build_and_adopt_and_marks_it_running() {
    let (checkout, support) = rig();
    let building = Building::default();
    let asked = ChangeFleetBuild { build: BuildSource::Preview, adopt: true };
    let changing = building
        .change(Some(checkout.path()), support.path(), &asked, SystemTime::now())
        .expect("started");
    assert_eq!(changing.build, BuildSource::Preview);
    assert_eq!(
        fs::read_to_string(support.path().join("restart-build.status")).unwrap(),
        "running\npreview\n"
    );
    assert_eq!(wait_for(&checkout.path().join("ran")).await.trim(), "preview --adopt");
}

#[tokio::test]
async fn a_change_without_adopt_passes_none() {
    let (checkout, support) = rig();
    let asked = ChangeFleetBuild { build: BuildSource::Main, adopt: false };
    Building::default()
        .change(Some(checkout.path()), support.path(), &asked, SystemTime::now())
        .expect("started");
    assert_eq!(wait_for(&checkout.path().join("ran")).await.trim(), "main");
}

#[tokio::test]
async fn a_second_change_while_one_is_under_way_is_refused() {
    let (checkout, support) = rig();
    let building = Building::default();
    fs::write(support.path().join("restart-build.status"), "running\nmain\n").unwrap();
    let asked = ChangeFleetBuild { build: BuildSource::Preview, adopt: false };
    let refused = building.change(Some(checkout.path()), support.path(), &asked, SystemTime::now());
    assert_eq!(refused, Err(NotChanged::Restarting(BuildSource::Main)));
    assert!(!checkout.path().join("ran").exists(), "nothing was started");
}

#[tokio::test]
async fn a_change_with_no_wrapper_to_run_is_refused() {
    let support = TempDir::new();
    let asked = ChangeFleetBuild { build: BuildSource::Main, adopt: false };
    let refused = Building::default().change(None, support.path(), &asked, SystemTime::now());
    assert_eq!(refused, Err(NotChanged::NotOffered(NotOffered::NoWrapper)));
}

#[tokio::test]
async fn a_wrapper_that_will_not_start_leaves_no_status_behind() {
    let (checkout, support) = rig();
    let wrapper = checkout.path().join("scripts/restart-build");
    Command::new("chmod").arg("-x").arg(&wrapper).status().expect("chmod");
    let asked = ChangeFleetBuild { build: BuildSource::Main, adopt: false };
    let refused = Building::default().change(Some(checkout.path()), support.path(), &asked, SystemTime::now());
    assert!(matches!(refused, Err(NotChanged::NotStarted(_))), "{refused:?}");
    assert!(!support.path().join("restart-build.status").exists());
}
