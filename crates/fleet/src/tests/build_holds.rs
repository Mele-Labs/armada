//! The running build's branches and origin's, named by the Job that held each, ended Jobs included.
//! What the files say of them is `tests::building`'s to assert.

use std::fs;
use std::process::Command;

use store::{AttachmentState, Holder, SessionState};

use super::build_notices::{git, Rig, a_rig};

/// `scripts/restart-build` in the rig's checkout, so Fleet offers its build, and `builds/current`
/// holding `branches`, each at its local tip.
fn running_with(rig: &Rig, branches: &[&str]) {
    let root = rig.home.path();
    let wrapper = root.join("scripts").join("restart-build");
    fs::create_dir_all(wrapper.parent().unwrap()).unwrap();
    fs::write(&wrapper, "#!/bin/sh\n").unwrap();
    Command::new("chmod").arg("+x").arg(&wrapper).status().expect("chmod");
    let builds = rig.support().join(crate::kept_builds::BUILDS);
    fs::create_dir_all(builds.join("new")).unwrap();
    let mut text = format!("built_at = \"2026-11-14T10:00:00Z\"\ncommit = \"{}\"\nsource = \"preview\"\n", rig.build);
    for branch in branches {
        text.push_str(&format!("\n[[job]]\nbranch = \"{branch}\"\ntip = \"{}\"\n", git(root, &["rev-parse", branch])));
    }
    fs::write(builds.join("new").join("build.toml"), text).unwrap();
    std::os::unix::fs::symlink("new", builds.join(crate::kept_builds::CURRENT)).unwrap();
}

/// A Job titled `title` that held `branch` and ended, its `branch` row given back as ending leaves it.
async fn ended_holding(rig: &Rig, title: &str, branch: &str) {
    let id = rig.a_hosted_session().await;
    rig.holds_branch(&id, branch).await;
    let at = rig.fleet.now().as_str().to_string();
    let mut store = rig.fleet.store().lock().await;
    store.settle_held(&Holder::session(&id), None, AttachmentState::GivenBack, &at).unwrap();
    let mut kept = store.session(&id).unwrap().expect("kept");
    kept.title = Some(title.into());
    kept.state = SessionState::Ended;
    store.keep_session(&kept).unwrap();
}

#[tokio::test]
async fn a_branch_an_ended_job_held_is_named_by_that_job_in_holds_and_origin() {
    let rig = a_rig();
    let root = rig.home.path();
    let long = git(root, &["rev-parse", "armada/long"]);
    git(root, &["update-ref", "refs/remotes/origin/armada/long", &long]);
    running_with(&rig, &["armada/work", "armada/other"]);
    ended_holding(&rig, "Chrome sketch mocks", "armada/work").await;
    ended_holding(&rig, "Long work", "armada/long").await;

    let report = rig.fleet.fleet_build().await.expect("a build");

    let holds: Vec<_> = report.holds.iter().map(|one| (one.branch.as_str(), one.title.as_deref())).collect();
    assert_eq!(holds, [("armada/work", Some("Chrome sketch mocks")), ("armada/other", None)]);
    let origin: Vec<_> = report.origin.iter().map(|one| (one.branch.as_str(), one.title.as_deref())).collect();
    assert_eq!(origin, [("armada/long", Some("Long work"))]);
    assert!(report.trying.contains(&String::from("armada/work")), "trying still names live Jobs alone");
}
