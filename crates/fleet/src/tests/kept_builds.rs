//! The swap back to the build before: two links and two launchd jobs, refused before anything
//! changes where it would move nothing. `docs/concepts/fleet.md`, *Kept builds*.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::kept_builds::{kept_at, revert, Services, BUILDS, CURRENT, PREVIOUS};
use crate::tests::tmp::TempDir;

const LABEL: &str = "com.armada.fleet";

/// launchd and the store copy, recorded.
#[derive(Default)]
struct Recorded {
    did: Mutex<Vec<String>>,
}

impl Services for Recorded {
    fn rebooted(&self, label: &str, _plist: &Path) -> Result<(), String> {
        self.did.lock().unwrap().push(format!("reboot {label}"));
        Ok(())
    }
    fn store_copied(&self, _db: &Path, to: &Path) -> Result<(), String> {
        std::fs::write(to, "copy").map_err(|why| why.to_string())?;
        self.did.lock().unwrap().push(String::from("copy"));
        Ok(())
    }
    fn fleet_came_up(&self, _support: &Path, _old: Option<u32>) -> Result<(), String> {
        self.did.lock().unwrap().push(String::from("up"));
        Ok(())
    }
}

fn build(builds: &Path, name: &str, bridge: &str) {
    std::fs::create_dir_all(builds.join(name)).unwrap();
    std::fs::write(
        builds.join(name).join("build.toml"),
        format!("built_at = \"2026-11-05T16:20:00Z\"\ncommit = \"{name}\"\nsource = \"main\"\nbridge = \"{bridge}\"\n"),
    )
    .unwrap();
}

/// A support folder whose Fleet runs from `builds/current`, on `new`, with `old` before it.
fn kept(new_bridge: &str, old_bridge: &str) -> (TempDir, PathBuf) {
    let home = TempDir::new();
    let support = home.path().join("Armada");
    let builds = support.join(BUILDS);
    build(&builds, "new", new_bridge);
    build(&builds, "old", old_bridge);
    std::os::unix::fs::symlink("new", builds.join(CURRENT)).unwrap();
    std::os::unix::fs::symlink("old", builds.join(PREVIOUS)).unwrap();
    let runs = builds.join(CURRENT).join("armada");
    std::fs::write(support.join(format!("{LABEL}.plist")), format!("<array><string>{}</string>", runs.display())).unwrap();
    std::fs::write(support.join(format!("{LABEL}.bridge.plist")), "bridge").unwrap();
    std::fs::write(support.join("armada.db"), "store").unwrap();
    (home, support)
}

fn on(support: &Path, link: &str) -> String {
    kept_at(&support.join(BUILDS), link).expect("a kept build").name
}

#[test]
fn a_revert_copies_the_store_then_swaps_the_links_and_boots_fleet_then_bridge() {
    let (_home, support) = kept("b2", "b1");
    let services = Recorded::default();

    let reverted = revert(&support, LABEL, "20261105T162000Z", &services).expect("reverted");

    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("old".to_string(), "new".to_string()));
    assert_eq!(reverted.store_copy, Some(support.join("backups/armada-20261105T162000Z.db")));
    assert!(support.join("backups/armada-20261105T162000Z.db").is_file());
    assert!(reverted.bridge_reopened);
    let did = services.did.lock().unwrap().clone();
    assert_eq!(did, ["copy", "reboot com.armada.fleet", "up", "reboot com.armada.fleet.bridge"]);
}

#[test]
fn a_second_revert_swaps_back_onto_the_build_reverted_from() {
    let (_home, support) = kept("b2", "b1");
    revert(&support, LABEL, "1", &Recorded::default()).expect("reverted");

    revert(&support, LABEL, "2", &Recorded::default()).expect("reverted again");

    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("new".to_string(), "old".to_string()));
}

#[test]
fn bridge_is_left_running_where_both_builds_hold_the_same_bridge() {
    let (_home, support) = kept("same", "same");
    let services = Recorded::default();

    let reverted = revert(&support, LABEL, "1", &services).expect("reverted");

    assert!(!reverted.bridge_reopened);
    assert!(!services.did.lock().unwrap().iter().any(|one| one.ends_with(".bridge")));
}

#[test]
fn no_previous_build_is_refused_and_nothing_moves() {
    let (_home, support) = kept("b2", "b1");
    std::fs::remove_file(support.join(BUILDS).join(PREVIOUS)).unwrap();
    let services = Recorded::default();

    let refused = revert(&support, LABEL, "1", &services).expect_err("refused");

    assert!(refused.contains("No build before this one"), "{refused}");
    assert_eq!(on(&support, CURRENT), "new");
    assert!(services.did.lock().unwrap().is_empty());
}

#[test]
fn a_fleet_launchd_does_not_run_from_the_kept_builds_is_refused_and_nothing_moves() {
    let (_home, support) = kept("b2", "b1");
    std::fs::write(support.join(format!("{LABEL}.plist")), "<string>/Users/someone/.cargo/bin/armada</string>").unwrap();
    let services = Recorded::default();

    let refused = revert(&support, LABEL, "1", &services).expect_err("refused");

    assert!(refused.contains("does not run"), "{refused}");
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("new".to_string(), "old".to_string()));
    assert!(services.did.lock().unwrap().is_empty());
}
