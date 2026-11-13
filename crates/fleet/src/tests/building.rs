//! The build Fleet runs on, read off the files `scripts/restart` leaves, and
//! the restart started detached. The wrapper is a stand-in script that writes
//! down what it was run with; the real one is `scripts/test_restart_build.py`'s.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

use ipc::{BuildPosition, BuildSource, BuildStage, ChangeFleetBuild, SnapshotState};

use crate::building::{
    checkout_of, left_out_in, Building, LeftOut, NotChanged, NotOffered, NotReverted, NotSnapshotted,
    NotSwapped,
};
use crate::kept_builds::{fleet_label, BUILDS, CURRENT, KEPT_TRIED, PICKED, PREVIOUS, SNAPSHOT, STATUS, SWAP_STATUS};
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

/// What the report says of a restart, for a status file holding `text`.
async fn restarting_on(text: &str) -> (Option<BuildSource>, Option<BuildStage>) {
    let (checkout, support) = rig();
    fs::write(support.path().join("restart-build.status"), text).unwrap();
    let report = Building::default()
        .report(Some(checkout.path()), support.path(), SystemTime::now())
        .await
        .expect("a build");
    (report.restarting, report.stage)
}

#[tokio::test]
async fn a_status_with_a_third_line_reports_that_stage() {
    for (word, stage) in [
        ("merging", BuildStage::Merging),
        ("fetching_main", BuildStage::FetchingMain),
        ("building_fleet", BuildStage::BuildingFleet),
        ("building_bridge", BuildStage::BuildingBridge),
        ("restarting_fleet", BuildStage::RestartingFleet),
        ("reopening_bridge", BuildStage::ReopeningBridge),
        ("retrying", BuildStage::Retrying),
    ] {
        let (restarting, said) = restarting_on(&format!("running\npreview\n{word}\n")).await;
        assert_eq!(restarting, Some(BuildSource::Preview));
        assert_eq!(said, Some(stage), "{word}");
    }
}

#[tokio::test]
async fn an_old_two_line_status_is_a_restart_with_no_stage() {
    assert_eq!(restarting_on("running\nmain\n").await, (Some(BuildSource::Main), None));
    assert_eq!(restarting_on("running\nmain").await, (Some(BuildSource::Main), None));
}

#[tokio::test]
async fn a_stage_word_this_fleet_does_not_know_is_no_stage_and_still_a_restart() {
    assert_eq!(restarting_on("running\nmain\ncompiling\n").await, (Some(BuildSource::Main), None));
    assert_eq!(restarting_on("running\nmain\n\n").await, (Some(BuildSource::Main), None));
}

#[tokio::test]
async fn a_failed_status_and_a_dead_wrapper_report_no_stage() {
    let (checkout, support) = rig();
    let status = support.path().join("restart-build.status");
    let building = Building::default();
    fs::write(&status, format!("failed\nmain\n{SHA}\nwhy\n")).unwrap();
    let report = building.report(Some(checkout.path()), support.path(), SystemTime::now()).await.unwrap();
    assert_eq!(report.stage, None);
    fs::write(&status, "running\nmain\nbuilding_fleet\n").unwrap();
    let later = SystemTime::now() + Duration::from_secs(21 * 60);
    let report = building.report(Some(checkout.path()), support.path(), later).await.unwrap();
    assert_eq!((report.restarting, report.stage), (None, None));
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
    // The wrapper's `>` creates the file before `echo` writes its line, so a read can land
    // between the two on a busy machine: wait for the whole line.
    for _ in 0..100 {
        if let Ok(text) = fs::read_to_string(path) {
            if text.ends_with('\n') {
                return text;
            }
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

#[test]
fn the_branches_left_out_are_read_for_the_running_build_only() {
    let support = TempDir::new();
    assert!(left_out_in(support.path(), SHA).is_empty(), "no file");

    let other = "f".repeat(40);
    let (tip_a, tip_b) = ("a".repeat(40), "b".repeat(40));
    fs::write(
        support.path().join("preview-left-out"),
        format!("{SHA}\narmada/a\t{tip_a}\tconflict: apps/a.ts, apps/b.ts\nno tab here\narmada/b\t{tip_b}\tdoes not build: x.rs\n"),
    )
    .unwrap();
    assert!(left_out_in(support.path(), &other).is_empty(), "written for another build");
    let read = left_out_in(support.path(), SHA);
    assert_eq!(read.len(), 2, "{read:?}");
    assert_eq!(read["armada/a"], LeftOut { tip: "a".repeat(40), why: "conflict: apps/a.ts, apps/b.ts".into() });
    assert_eq!(read["armada/b"], LeftOut { tip: "b".repeat(40), why: "does not build: x.rs".into() });
}

#[tokio::test]
async fn the_report_carries_the_branches_the_preview_left_out_sorted_with_why() {
    let (checkout, support) = rig();
    a_preview_that_left_branches_out(support.path());
    let report = Building::default()
        .report(Some(checkout.path()), support.path(), SystemTime::now())
        .await
        .expect("the preview");
    let left: Vec<(&str, &str)> = report.left_out.iter().map(|l| (l.branch.as_str(), l.why.as_str())).collect();
    assert_eq!(
        left,
        [
            ("armada/a", "conflict: apps/a.ts"),
            ("armada/m", "breaking migration projects.span_repositories is not on main"),
        ]
    );
}

#[tokio::test]
async fn the_report_names_nothing_left_out_on_main_or_without_a_file_or_for_another_build() {
    let (checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();

    a_preview_that_left_branches_out(support.path());
    fs::write(support.path().join("restart-commit"), format!("{}\n", "f".repeat(40))).unwrap();
    let other = building.report(Some(checkout.path()), support.path(), now).await.unwrap();
    assert!(other.left_out.is_empty(), "written for another build");

    a_preview_that_left_branches_out(support.path());
    fs::write(support.path().join("restart-source"), "").unwrap();
    let main = building.report(Some(checkout.path()), support.path(), now).await.unwrap();
    assert_eq!(main.on, BuildSource::Main);
    assert!(main.left_out.is_empty(), "main left nothing out, whatever the file says");

    fs::remove_file(support.path().join("preview-left-out")).unwrap();
    fs::write(support.path().join("restart-source"), "/Users/user/armada/.armada/preview\n").unwrap();
    let none = building.report(Some(checkout.path()), support.path(), now).await.unwrap();
    assert!(none.left_out.is_empty(), "no file");
}

#[tokio::test]
async fn the_report_marks_a_left_out_branch_whose_local_tip_moved() {
    let (checkout, support) = rig();
    let root = checkout.path();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", "base"]);
    git(root, &["branch", "armada/a"]);
    let tip = git(root, &["rev-parse", "armada/a"]);
    fs::write(support.path().join("restart-source"), "/Users/user/armada/.armada/preview\n").unwrap();
    fs::write(support.path().join("restart-commit"), format!("{SHA}\n")).unwrap();
    fs::write(
        support.path().join("preview-left-out"),
        format!("{SHA}\narmada/a\t{tip}\tconflict: apps/a.ts\narmada/gone\t{tip}\tconflict: apps/b.ts\n"),
    )
    .unwrap();
    let building = Building::default();
    let moved = |report: ipc::FleetBuildReport| -> Vec<(String, bool)> {
        report.left_out.into_iter().map(|one| (one.branch, one.moved)).collect()
    };

    let still = building.report(Some(root), support.path(), SystemTime::now()).await.unwrap();
    assert_eq!(moved(still), [("armada/a".to_string(), false), ("armada/gone".to_string(), false)]);

    git(root, &["checkout", "-q", "armada/a"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", "moved on"]);
    let after = building.report(Some(root), support.path(), SystemTime::now()).await.unwrap();
    assert_eq!(
        moved(after),
        [("armada/a".to_string(), true), ("armada/gone".to_string(), false)],
        "a branch git cannot find has not moved"
    );
}

/// The preview at `SHA`, which left out `armada/a` at the tip `"a" * 40` and `armada/m` for a
/// migration that is not on main.
fn a_preview_that_left_branches_out(support: &Path) {
    fs::write(support.join("restart-source"), "/Users/user/armada/.armada/preview\n").unwrap();
    fs::write(support.join("restart-commit"), format!("{SHA}\n")).unwrap();
    fs::write(
        support.join("preview-left-out"),
        format!(
            "{SHA}\narmada/a\t{}\tconflict: apps/a.ts\narmada/m\t{}\tbreaking migration projects.span_repositories is not on main\n",
            "a".repeat(40),
            "c".repeat(40)
        ),
    )
    .unwrap();
}

/// Two kept builds in `support`, Fleet running from `builds/current` on `new`, with `old` before it. The
/// build `old` holds a stand-in `armada` that writes down what it was run with, in its working directory.
fn kept_in(support: &Path, label: &str, new_jobs: &[&str], old_jobs: &[&str]) {
    let builds = support.join(BUILDS);
    for (name, bridge, jobs) in [("new", "b2", new_jobs), ("old", "b1", old_jobs)] {
        fs::create_dir_all(builds.join(name)).unwrap();
        let mut text = format!("built_at = \"2026-11-05T1{}:20:00Z\"\ncommit = \"{name}{SHA}\"\nsource = \"preview\"\nbridge = \"{bridge}\"\n", if name == "new" { 7 } else { 6 });
        for branch in jobs {
            text.push_str(&format!("\n[[job]]\nbranch = \"{branch}\"\ntip = \"0123\"\n"));
        }
        fs::write(builds.join(name).join("build.toml"), text).unwrap();
    }
    let undo = builds.join("old").join("armada");
    fs::write(&undo, "#!/bin/sh\necho \"$@\" > ran\n").unwrap();
    Command::new("chmod").arg("+x").arg(&undo).status().expect("chmod");
    std::os::unix::fs::symlink("new", builds.join(CURRENT)).unwrap();
    std::os::unix::fs::symlink("old", builds.join(PREVIOUS)).unwrap();
    let runs = builds.join(CURRENT).join("armada");
    fs::write(support.join(format!("{label}.plist")), format!("<string>{}</string>", runs.display())).unwrap();
}

#[tokio::test]
async fn the_report_carries_the_running_build_and_the_one_before_with_what_a_revert_changes() {
    let (checkout, support) = rig();
    kept_in(support.path(), &fleet_label(), &["armada/a", "armada/both"], &["armada/both", "armada/c"]);

    let report = Building::default()
        .report(Some(checkout.path()), support.path(), SystemTime::now())
        .await
        .expect("a build");

    let running = report.running.expect("the running build");
    assert_eq!((running.built_at.as_str(), running.commit), ("2026-11-05T17:20:00Z", format!("new{SHA}")));
    let previous = report.previous.expect("a previous build");
    assert_eq!((previous.built_at.as_str(), previous.commit.clone()), ("2026-11-05T16:20:00Z", format!("old{SHA}")));
    assert!(previous.bridge_reopens);
    assert_eq!(previous.takes_out, ["armada/a"]);
    assert_eq!(previous.brings_back, ["armada/c"]);
    assert!(!report.reverting);
}

#[tokio::test]
async fn no_previous_build_is_offered_where_none_is_kept_or_launchd_runs_another_fleet() {
    let (checkout, support) = rig();
    let report = Building::default().report(Some(checkout.path()), support.path(), SystemTime::now()).await.unwrap();
    assert_eq!((report.running, report.previous), (None, None), "nothing kept");

    kept_in(support.path(), &fleet_label(), &[], &[]);
    fs::write(support.path().join(format!("{}.plist", fleet_label())), "<string>/Users/user/.cargo/bin/armada</string>").unwrap();
    let report = Building::default().report(Some(checkout.path()), support.path(), SystemTime::now()).await.unwrap();
    assert!(report.running.is_some(), "the build current names is still the one running");
    assert_eq!(report.previous, None, "a revert would change nothing Fleet runs");

    fs::remove_file(support.path().join(BUILDS).join(PREVIOUS)).unwrap();
    fs::write(support.path().join(format!("{}.plist", fleet_label())), "").unwrap();
    let report = Building::default().report(Some(checkout.path()), support.path(), SystemTime::now()).await.unwrap();
    assert_eq!(report.previous, None, "no previous build");
}

#[tokio::test]
async fn a_revert_under_way_is_reported_until_it_is_two_minutes_old_and_a_failed_one_gives_its_line() {
    let (checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();
    fs::write(support.path().join(STATUS), "running\n").unwrap();
    assert!(building.report(Some(checkout.path()), support.path(), now).await.unwrap().reverting);
    let later = now + Duration::from_secs(3 * 60);
    assert!(!building.report(Some(checkout.path()), support.path(), later).await.unwrap().reverting);

    kept_in(support.path(), &fleet_label(), &[], &[]);
    fs::write(support.path().join(STATUS), "failed\nnew\nlaunchd would not load /x/y.plist\n").unwrap();
    let report = building.report(Some(checkout.path()), support.path(), now).await.unwrap();
    assert_eq!((report.reverting, report.failed.as_deref()), (false, Some("launchd would not load /x/y.plist")));

    // A restart that failed on this build has its own line, which the panel already follows.
    fs::write(support.path().join("restart-commit"), format!("{SHA}\n")).unwrap();
    fs::write(support.path().join("restart-build.status"), format!("failed\nmain\n{SHA}\nmain cannot fast-forward\n")).unwrap();
    let report = building.report(Some(checkout.path()), support.path(), now).await.unwrap();
    assert_eq!(report.failed.as_deref(), Some("main cannot fast-forward"));
}

#[tokio::test]
async fn a_revert_marks_it_running_and_starts_the_previous_build_with_build_revert() {
    let (_checkout, support) = rig();
    kept_in(support.path(), "com.test.fleet", &[], &[]);

    let reverting = Building::default().revert(support.path(), "com.test.fleet", SystemTime::now()).expect("started");

    assert_eq!(reverting.commit, format!("old{SHA}"));
    assert_eq!(fs::read_to_string(support.path().join(STATUS)).unwrap(), "running\n");
    assert_eq!(wait_for(&support.path().join("ran")).await.trim(), "build revert");
}

#[tokio::test]
async fn a_revert_is_refused_where_none_is_kept_or_fleet_is_not_on_the_kept_builds() {
    let (_checkout, support) = rig();
    let building = Building::default();
    let refused = building.revert(support.path(), "com.test.fleet", SystemTime::now());
    assert!(matches!(refused, Err(NotReverted::NotRevertible(_))), "{refused:?}");

    kept_in(support.path(), "com.test.fleet", &[], &[]);
    fs::write(support.path().join("com.test.fleet.plist"), "<string>/elsewhere/armada</string>").unwrap();
    let refused = building.revert(support.path(), "com.test.fleet", SystemTime::now());
    assert!(matches!(refused, Err(NotReverted::NotRevertible(_))), "{refused:?}");
    assert!(!support.path().join(STATUS).exists());
    assert!(!support.path().join("ran").exists(), "nothing was started");
}

#[tokio::test]
async fn a_revert_is_refused_while_a_restart_or_a_revert_is_under_way() {
    let (_checkout, support) = rig();
    kept_in(support.path(), "com.test.fleet", &[], &[]);
    let building = Building::default();

    fs::write(support.path().join("restart-build.status"), "running\npreview\n").unwrap();
    assert_eq!(
        building.revert(support.path(), "com.test.fleet", SystemTime::now()),
        Err(NotReverted::Restarting(BuildSource::Preview))
    );
    fs::remove_file(support.path().join("restart-build.status")).unwrap();

    fs::write(support.path().join(STATUS), "running\n").unwrap();
    assert_eq!(building.revert(support.path(), "com.test.fleet", SystemTime::now()), Err(NotReverted::Reverting));
    assert!(!support.path().join("ran").exists(), "nothing was started");

    // One that died more than two minutes ago is not under way.
    let later = SystemTime::now() + Duration::from_secs(3 * 60);
    assert!(building.revert(support.path(), "com.test.fleet", later).is_ok());
}

#[tokio::test]
async fn an_undo_that_will_not_start_leaves_no_status_behind() {
    let (_checkout, support) = rig();
    kept_in(support.path(), "com.test.fleet", &[], &[]);
    Command::new("chmod").arg("-x").arg(support.path().join(BUILDS).join("old").join("armada")).status().expect("chmod");

    let refused = Building::default().revert(support.path(), "com.test.fleet", SystemTime::now());

    assert!(matches!(refused, Err(NotReverted::NotStarted(_))), "{refused:?}");
    assert!(!support.path().join(STATUS).exists());
}

/// A third kept build, `snap`, that `snapshot` names, with a stand-in `armada` in `current` too, which
/// writes down what it was run with. Call after [`kept_in`].
fn snapshot_in(support: &Path, jobs: &[&str]) {
    let snap = support.join(BUILDS).join("snap");
    fs::create_dir_all(&snap).unwrap();
    let mut text = format!("built_at = \"2026-11-05T18:20:00Z\"\ncommit = \"snap{SHA}\"\nsource = \"preview\"\nbridge = \"b3\"\n");
    for branch in jobs {
        text.push_str(&format!("\n[[job]]\nbranch = \"{branch}\"\ntip = \"0123\"\n"));
    }
    fs::write(snap.join("build.toml"), text).unwrap();
    fs::write(support.join(SNAPSHOT), "snap\n").unwrap();
    let swap = support.join(BUILDS).join("new").join("armada");
    fs::write(&swap, "#!/bin/sh\necho \"$@\" > ran\n").unwrap();
    Command::new("chmod").arg("+x").arg(&swap).status().expect("chmod");
}

#[tokio::test]
async fn the_report_reads_each_snapshot_state_from_the_files() {
    let (checkout, support) = rig();
    kept_in(support.path(), &fleet_label(), &["armada/a"], &[]);
    let building = Building::default();
    let now = SystemTime::now();
    let read = |at: SystemTime| {
        let (checkout, support) = (checkout.path().to_path_buf(), support.path().to_path_buf());
        let building = building.clone();
        async move { building.report(Some(&checkout), &support, at).await.unwrap() }
    };
    assert_eq!(read(now).await.snapshot, None, "nothing kept to swap onto");

    snapshot_in(support.path(), &["armada/a", "armada/s"]);
    let ready = read(now).await.snapshot.expect("a kept snapshot");
    assert_eq!(ready.state, SnapshotState::Ready);
    assert_eq!((ready.built_at.as_ref().map(|at| at.as_str()), ready.commit), (Some("2026-11-05T18:20:00Z"), Some(format!("snap{SHA}"))));
    assert_eq!(ready.jobs, ["armada/a", "armada/s"], "branches here; Fleet names them by Job");
    assert!(ready.bridge_reopens, "b3 against b2");
    assert_eq!((ready.stage, ready.failed), (None, None));

    fs::write(support.path().join("snapshot.status"), "running\nsnapshot\nbuilding_bridge\n").unwrap();
    let building_one = read(now).await.snapshot.expect("one being built");
    assert_eq!((building_one.state, building_one.stage), (SnapshotState::Building, Some(BuildStage::BuildingBridge)));
    assert_eq!((building_one.commit, building_one.jobs.len()), (None, 0), "the old snapshot is not offered while a new one builds");

    let later = now + Duration::from_secs(21 * 60);
    assert_eq!(read(later).await.snapshot.expect("a build that died is not building").state, SnapshotState::Ready);

    fs::write(support.path().join("snapshot.status"), "running\nsnapshot\n").unwrap();
    assert_eq!(read(now).await.snapshot.expect("building").stage, None, "a status with no third line has no stage");

    fs::write(support.path().join("snapshot.status"), format!("failed\nsnapshot\n{SHA}\narmada/s conflicts with main\n")).unwrap();
    let failed = read(now).await.snapshot.expect("a failed one");
    assert_eq!((failed.state, failed.failed.as_deref()), (SnapshotState::Failed, Some("armada/s conflicts with main")));
    assert_eq!((failed.commit, failed.built_at), (None, None));
}

#[tokio::test]
async fn a_swap_under_way_is_reported_and_a_failed_one_gives_its_line_where_nothing_else_failed() {
    let (checkout, support) = rig();
    kept_in(support.path(), &fleet_label(), &[], &[]);
    let building = Building::default();
    let now = SystemTime::now();
    fs::write(support.path().join(SWAP_STATUS), "running\n").unwrap();
    assert!(building.report(Some(checkout.path()), support.path(), now).await.unwrap().swapping);
    let later = now + Duration::from_secs(3 * 60);
    assert!(!building.report(Some(checkout.path()), support.path(), later).await.unwrap().swapping, "a swap that died is not under way");

    fs::write(support.path().join(SWAP_STATUS), "failed\nnew\nSwapped back, since the new Fleet did not answer\n").unwrap();
    let report = building.report(Some(checkout.path()), support.path(), now).await.unwrap();
    assert_eq!((report.swapping, report.failed.as_deref()), (false, Some("Swapped back, since the new Fleet did not answer")));
}

#[tokio::test]
async fn picked_defaults_to_the_running_builds_jobs_until_the_person_picks() {
    let (checkout, support) = rig();
    let building = Building::default();
    let read = || building.report(Some(checkout.path()), support.path(), SystemTime::now());
    assert!(read().await.unwrap().picked.is_empty(), "no build and no picks");

    kept_in(support.path(), &fleet_label(), &["armada/a", "armada/b"], &[]);
    let report = read().await.unwrap();
    let branches: Vec<_> = report.picked.iter().map(|one| one.branch.as_str()).collect();
    assert_eq!(branches, ["armada/a", "armada/b"]);
    assert!(report.picked.iter().all(|one| one.title.is_none()), "Fleet::fleet_build names them");

    fs::write(support.path().join(PICKED), "armada/c\n").unwrap();
    let branches: Vec<_> = read().await.unwrap().picked.into_iter().map(|one| one.branch).collect();
    assert_eq!(branches, ["armada/c"]);
}

#[tokio::test]
async fn trying_is_the_running_builds_jobs_until_keep_empties_it() {
    let (checkout, support) = rig();
    kept_in(support.path(), &fleet_label(), &["armada/a", "armada/b"], &[]);
    let building = Building::default();
    let report = building.report(Some(checkout.path()), support.path(), SystemTime::now()).await.unwrap();
    assert_eq!(report.trying, ["armada/a", "armada/b"]);

    assert_eq!(crate::kept_builds::kept_tried(support.path()).unwrap(), ["armada/a", "armada/b"]);
    assert_eq!(fs::read_to_string(support.path().join(KEPT_TRIED)).unwrap(), "new\n");
    let report = building.report(Some(checkout.path()), support.path(), SystemTime::now()).await.unwrap();
    assert!(report.trying.is_empty());
    assert!(crate::kept_builds::kept_tried(support.path()).unwrap().is_empty(), "a second Keep tells nobody");
}

#[tokio::test]
async fn a_pick_writes_the_picked_file_and_the_status_clears_the_old_snapshot_and_starts_the_wrapper() {
    let (checkout, support) = rig();
    kept_in(support.path(), "com.test.fleet", &["armada/a"], &[]);
    snapshot_in(support.path(), &["armada/a"]);

    let snapshotting = Building::default()
        .snapshot(Some(checkout.path()), support.path(), "armada/b", true, SystemTime::now())
        .expect("started");

    assert_eq!(snapshotting.picked, ["armada/a", "armada/b"], "the running build's Jobs, then the pick");
    assert_eq!(fs::read_to_string(support.path().join(PICKED)).unwrap(), "armada/a\narmada/b\n");
    assert_eq!(fs::read_to_string(support.path().join("snapshot.status")).unwrap(), "running\nsnapshot\n");
    assert!(!support.path().join(SNAPSHOT).exists(), "the old snapshot is no longer offered");
    assert_eq!(wait_for(&checkout.path().join("ran")).await.trim(), "snapshot");

    let taken_back = Building::default()
        .snapshot(Some(checkout.path()), support.path(), "armada/a", false, SystemTime::now() + Duration::from_secs(21 * 60))
        .expect("a snapshot that died is not under way");
    assert_eq!(taken_back.picked, ["armada/b"]);
}

#[tokio::test]
async fn a_pick_is_offered_on_main_and_refused_with_no_checkout_while_a_restart_or_a_snapshot_is_under_way() {
    let (checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();

    assert_eq!(building.snapshot(None, support.path(), "armada/a", true, now), Err(NotSnapshotted::NotOffered(NotOffered::NoWrapper)));
    assert!(!support.path().join(PICKED).exists(), "nothing was written");

    fs::write(support.path().join("restart-source"), "/Users/user/elsewhere\n").unwrap();
    assert!(building.snapshot(Some(checkout.path()), support.path(), "armada/a", true, now).is_ok(), "the tree the build came from is not asked");
    fs::remove_file(support.path().join("restart-source")).unwrap();
    fs::remove_file(support.path().join("snapshot.status")).unwrap();
    fs::remove_file(support.path().join(PICKED)).unwrap();

    fs::write(support.path().join("restart-build.status"), "running\npreview\n").unwrap();
    assert_eq!(
        building.snapshot(Some(checkout.path()), support.path(), "armada/a", true, now),
        Err(NotSnapshotted::Restarting(BuildSource::Preview))
    );
    fs::remove_file(support.path().join("restart-build.status")).unwrap();
    assert!(!support.path().join(PICKED).exists(), "a refusal picks nothing");

    fs::write(support.path().join("snapshot.status"), "running\nsnapshot\n").unwrap();
    assert_eq!(building.snapshot(Some(checkout.path()), support.path(), "armada/a", true, now), Err(NotSnapshotted::Snapshotting));
    assert!(!support.path().join(PICKED).exists(), "a refusal picks nothing");

    // A failed one is over, and a new pick starts another.
    fs::write(support.path().join("snapshot.status"), format!("failed\nsnapshot\n{SHA}\nconflict\n")).unwrap();
    assert!(building.snapshot(Some(checkout.path()), support.path(), "armada/a", true, now).is_ok());
}

#[tokio::test]
async fn a_wrapper_that_will_not_start_leaves_no_snapshot_status_behind() {
    let (checkout, support) = rig();
    Command::new("chmod").arg("-x").arg(checkout.path().join("scripts/restart-build")).status().expect("chmod");

    let refused = Building::default().snapshot(Some(checkout.path()), support.path(), "armada/a", true, SystemTime::now());

    assert!(matches!(refused, Err(NotSnapshotted::NotStarted(_))), "{refused:?}");
    assert!(!support.path().join("snapshot.status").exists());
}

#[tokio::test]
async fn a_swap_marks_it_running_and_starts_the_running_build_with_build_swap() {
    let (_checkout, support) = rig();
    kept_in(support.path(), "com.test.fleet", &[], &[]);
    snapshot_in(support.path(), &[]);

    let swapping = Building::default().swap(support.path(), "com.test.fleet", SystemTime::now()).expect("started");

    assert_eq!(swapping.commit, format!("snap{SHA}"));
    assert_eq!(fs::read_to_string(support.path().join(SWAP_STATUS)).unwrap(), "running\n");
    assert_eq!(wait_for(&support.path().join("ran")).await.trim(), "build swap", "run by the build running now, which settles it");
}

#[tokio::test]
async fn a_swap_is_refused_with_no_snapshot_or_one_that_is_running_or_that_launchd_does_not_run() {
    let (_checkout, support) = rig();
    let building = Building::default();
    let refused = building.swap(support.path(), "com.test.fleet", SystemTime::now());
    assert!(matches!(refused, Err(NotSwapped::NotSwappable(_))), "{refused:?}");

    kept_in(support.path(), "com.test.fleet", &[], &[]);
    let refused = building.swap(support.path(), "com.test.fleet", SystemTime::now());
    assert!(matches!(refused, Err(NotSwapped::NotSwappable(_))), "kept builds but no snapshot: {refused:?}");

    snapshot_in(support.path(), &[]);
    fs::write(support.path().join(SNAPSHOT), "new\n").unwrap();
    let refused = building.swap(support.path(), "com.test.fleet", SystemTime::now());
    assert!(matches!(refused, Err(NotSwapped::NotSwappable(_))), "the snapshot is the build running: {refused:?}");

    fs::write(support.path().join(SNAPSHOT), "snap\n").unwrap();
    fs::write(support.path().join("com.test.fleet.plist"), "<string>/elsewhere/armada</string>").unwrap();
    let refused = building.swap(support.path(), "com.test.fleet", SystemTime::now());
    assert!(matches!(refused, Err(NotSwapped::NotSwappable(_))), "{refused:?}");
    assert!(!support.path().join(SWAP_STATUS).exists());
    assert!(!support.path().join("ran").exists(), "nothing was started");
}

#[tokio::test]
async fn a_swap_is_refused_while_a_restart_a_revert_or_a_swap_is_under_way() {
    let (_checkout, support) = rig();
    kept_in(support.path(), "com.test.fleet", &[], &[]);
    snapshot_in(support.path(), &[]);
    let building = Building::default();
    let now = SystemTime::now();

    fs::write(support.path().join("restart-build.status"), "running\npreview\n").unwrap();
    assert_eq!(building.swap(support.path(), "com.test.fleet", now), Err(NotSwapped::Restarting(BuildSource::Preview)));
    fs::remove_file(support.path().join("restart-build.status")).unwrap();

    fs::write(support.path().join(STATUS), "running\n").unwrap();
    assert_eq!(building.swap(support.path(), "com.test.fleet", now), Err(NotSwapped::Reverting));
    fs::remove_file(support.path().join(STATUS)).unwrap();

    fs::write(support.path().join(SWAP_STATUS), "running\n").unwrap();
    assert_eq!(building.swap(support.path(), "com.test.fleet", now), Err(NotSwapped::Swapping));
    assert!(!support.path().join("ran").exists(), "nothing was started");

    // One that died more than two minutes ago is not under way.
    assert!(building.swap(support.path(), "com.test.fleet", now + Duration::from_secs(3 * 60)).is_ok());
}

#[tokio::test]
async fn a_swap_that_will_not_start_leaves_no_status_behind() {
    let (_checkout, support) = rig();
    kept_in(support.path(), "com.test.fleet", &[], &[]);
    snapshot_in(support.path(), &[]);
    Command::new("chmod").arg("-x").arg(support.path().join(BUILDS).join("new").join("armada")).status().expect("chmod");

    let refused = Building::default().swap(support.path(), "com.test.fleet", SystemTime::now());

    assert!(matches!(refused, Err(NotSwapped::NotStarted(_))), "{refused:?}");
    assert!(!support.path().join(SWAP_STATUS).exists());
}

#[tokio::test]
async fn origin_lists_the_branches_on_origin_main_lacks_less_the_picked() {
    let (checkout, support) = rig();
    let root = checkout.path();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", "main"]);
    git(root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    for branch in ["native/refresh-set", "feat/a", "feat/b"] {
        git(root, &["checkout", "-q", "-b", branch, "main"]);
        git(root, &["commit", "-q", "--allow-empty", "-m", branch]);
        git(root, &["update-ref", &format!("refs/remotes/origin/{branch}"), "HEAD"]);
    }
    let building = Building::default();
    let read = || building.report(Some(checkout.path()), support.path(), SystemTime::now());
    let mut listed: Vec<_> = read().await.unwrap().origin.into_iter().map(|one| one.branch).collect();
    assert_eq!(listed.remove(0), "native/refresh-set", "the set is first");
    listed.sort();
    assert_eq!(listed, ["feat/a", "feat/b"]);

    fs::write(support.path().join(PICKED), "feat/a\nnative/refresh-set\n").unwrap();
    let listed: Vec<_> = read().await.unwrap().origin.into_iter().map(|one| one.branch).collect();
    assert_eq!(listed, ["feat/b"], "a picked branch is a chip, not a row");
}
