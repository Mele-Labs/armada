//! The swap back to the build before: two links and two launchd jobs, refused before anything
//! changes where it would move nothing. `docs/concepts/fleet.md`, *Kept builds*.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use std::time::{Duration, SystemTime};

use crate::kept_builds::{
    answers_health, kept_at, rehearse, revert, revert_begun, revert_ended, revert_status_in, revertible, settle, undoing, RevertStatus,
    Reverted, Services, Settled, BUILDS, CURRENT, PREVIOUS, STATUS,
};
use crate::tests::tmp::TempDir;

const LABEL: &str = "com.test.fleet";

/// launchd and the store copy, recorded.
#[derive(Default)]
struct Recorded {
    did: Mutex<Vec<String>>,
    /// Why the next Fleet asked after does not answer, once.
    down_once: Mutex<Option<String>>,
}

impl Services for Recorded {
    fn rebooted(&self, label: &str, _plist: &Path) -> Result<(), String> {
        self.did.lock().unwrap().push(format!("reboot {label}"));
        Ok(())
    }
    fn store_copied(&self, db: &Path, to: &Path) -> Result<(), String> {
        std::fs::copy(db, to).map_err(|why| why.to_string())?;
        self.did.lock().unwrap().push(String::from("copy"));
        Ok(())
    }
    fn fleet_came_up(&self, _support: &Path, _old: Option<u32>) -> Result<(), String> {
        if let Some(why) = self.down_once.lock().unwrap().take() {
            self.did.lock().unwrap().push(String::from("down"));
            return Err(why);
        }
        self.did.lock().unwrap().push(String::from("up"));
        Ok(())
    }
}

fn build(builds: &Path, name: &str, bridge: &str) {
    build_of(builds, name, bridge, &[]);
}

/// A build holding the preview's branches `jobs`, as `scripts/keep-build` writes the rows.
fn build_of(builds: &Path, name: &str, bridge: &str, jobs: &[&str]) {
    std::fs::create_dir_all(builds.join(name)).unwrap();
    let mut text = format!("built_at = \"2026-11-05T16:20:00Z\"\ncommit = \"{name}\"\nsource = \"main\"\nbridge = \"{bridge}\"\n");
    for branch in jobs {
        text.push_str(&format!("\n[[job]]\nbranch = \"{branch}\"\ntip = \"0123\"\n"));
    }
    std::fs::write(builds.join(name).join("build.toml"), text).unwrap();
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
    assert_eq!(did, ["copy".to_string(), format!("reboot {LABEL}"), "up".to_string(), format!("reboot {LABEL}.bridge")]);
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
    std::fs::write(support.join(format!("{LABEL}.plist")), "<string>/Users/user/.cargo/bin/armada</string>").unwrap();
    let services = Recorded::default();

    let refused = revert(&support, LABEL, "1", &services).expect_err("refused");

    assert!(refused.contains("does not run"), "{refused}");
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("new".to_string(), "old".to_string()));
    assert!(services.did.lock().unwrap().is_empty());
}

#[test]
fn a_build_names_the_branches_of_its_job_rows_and_a_main_build_has_none() {
    let (_home, support) = kept("b2", "b1");
    let builds = support.join(BUILDS);
    build_of(&builds, "new", "b2", &["armada/a", "armada/b"]);

    assert_eq!(kept_at(&builds, CURRENT).unwrap().jobs, ["armada/a", "armada/b"]);
    assert!(kept_at(&builds, PREVIOUS).unwrap().jobs.is_empty());
}

#[test]
fn a_revert_takes_out_the_branches_only_the_running_build_holds_and_brings_back_the_reverse() {
    let (_home, support) = kept("b2", "b1");
    let builds = support.join(BUILDS);
    build_of(&builds, "new", "b2", &["armada/a", "armada/both"]);
    build_of(&builds, "old", "b1", &["armada/both", "armada/c"]);
    let (from, to) = revertible(&support, LABEL).expect("revertible");

    let undo = undoing(&from, &to);

    assert_eq!(undo.takes_out, ["armada/a"]);
    assert_eq!(undo.brings_back, ["armada/c"]);
    assert_eq!((undo.built_at.as_str(), undo.commit.as_str()), ("2026-11-05T16:20:00Z", "old"));
}

#[test]
fn bridge_reopens_where_the_hashes_differ_or_either_is_missing() {
    let reopens = |new: &str, old: &str, drop_new: bool, drop_old: bool| {
        let (_home, support) = kept(new, old);
        let builds = support.join(BUILDS);
        for (name, dropped) in [("new", drop_new), ("old", drop_old)] {
            if dropped {
                let record = builds.join(name).join("build.toml");
                let text = std::fs::read_to_string(&record).unwrap();
                std::fs::write(&record, text.lines().filter(|line| !line.starts_with("bridge")).collect::<Vec<_>>().join("\n")).unwrap();
            }
        }
        let (from, to) = revertible(&support, LABEL).expect("revertible");
        undoing(&from, &to).bridge_reopens
    };
    assert!(reopens("b2", "b1", false, false), "differ");
    assert!(!reopens("same", "same", false, false), "equal");
    assert!(reopens("same", "same", true, false), "the running build lacks it");
    assert!(reopens("same", "same", false, true), "the previous build lacks it");
    assert!(reopens("same", "same", true, true), "both lack it");
}

#[test]
fn revertible_is_the_check_revert_makes_and_names_why_not() {
    let (_home, support) = kept("b2", "b1");
    assert!(revertible(&support, LABEL).is_ok());

    std::fs::write(support.join(format!("{LABEL}.plist")), "<string>/Users/user/.cargo/bin/armada</string>").unwrap();
    assert!(revertible(&support, LABEL).unwrap_err().contains("does not run"));

    std::fs::remove_file(support.join(BUILDS).join(PREVIOUS)).unwrap();
    assert!(revertible(&support, LABEL).unwrap_err().contains("No build before this one"));

    std::fs::remove_file(support.join(BUILDS).join(CURRENT)).unwrap();
    assert!(revertible(&support, LABEL).unwrap_err().contains("nothing to revert from"));
}

#[test]
fn a_revert_under_way_is_running_until_it_is_old_enough_to_be_a_dead_one() {
    let (_home, support) = kept("b2", "b1");
    revert_begun(&support).unwrap();
    let now = SystemTime::now();

    assert_eq!(revert_status_in(&support, now), Some(RevertStatus::Running));
    assert_eq!(revert_status_in(&support, now + Duration::from_secs(119)), Some(RevertStatus::Running));
    assert_eq!(revert_status_in(&support, now + Duration::from_secs(3 * 60)), None);
}

#[test]
fn a_revert_that_took_removes_its_status_and_one_that_failed_keeps_one_line_until_the_build_moves() {
    let (_home, support) = kept("b2", "b1");
    revert_begun(&support).unwrap();
    let builds = support.join(BUILDS);
    let took = Reverted {
        from: kept_at(&builds, CURRENT).unwrap(),
        to: kept_at(&builds, PREVIOUS).unwrap(),
        bridge_reopened: false,
        store_copy: None,
    };

    revert_ended(&support, &Ok(took));
    assert!(!support.join(STATUS).exists());

    revert_ended(&support, &Err(String::from("launchd would not load\n/x/y.plist")));
    assert_eq!(std::fs::read_to_string(support.join(STATUS)).unwrap(), "failed\nnew\nlaunchd would not load /x/y.plist\n");
    assert_eq!(
        revert_status_in(&support, SystemTime::now()),
        Some(RevertStatus::Failed(String::from("launchd would not load /x/y.plist")))
    );

    // A restart onto another build since makes the failure history.
    std::fs::remove_file(builds.join(CURRENT)).unwrap();
    std::os::unix::fs::symlink("old", builds.join(CURRENT)).unwrap();
    assert_eq!(revert_status_in(&support, SystemTime::now()), None);
}

#[test]
fn a_rehearsal_opens_a_copy_of_the_store_and_leaves_none_behind() {
    let (_home, support) = kept("b2", "b1");
    std::fs::remove_file(support.join("armada.db")).unwrap();
    drop(store::Store::open(&support.join("armada.db")).expect("a store"));
    let before = std::fs::read(support.join("armada.db")).unwrap();

    rehearse(&support, &Recorded::default()).expect("rehearsed");

    assert_eq!(std::fs::read(support.join("armada.db")).unwrap(), before);
    let left: Vec<_> = std::fs::read_dir(support.join("backups")).unwrap().map(|one| one.unwrap().file_name()).collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn a_rehearsal_on_a_store_this_build_cannot_open_is_refused_and_the_store_is_untouched() {
    let (_home, support) = kept("b2", "b1");

    let refused = rehearse(&support, &Recorded::default()).expect_err("refused");

    assert!(refused.contains("could not open a copy of the store"), "{refused}");
    assert_eq!(std::fs::read_to_string(support.join("armada.db")).unwrap(), "store");
}

#[test]
fn a_rehearsal_with_no_store_has_nothing_to_open() {
    let (_home, support) = kept("b2", "b1");
    std::fs::remove_file(support.join("armada.db")).unwrap();
    let services = Recorded::default();

    rehearse(&support, &services).expect("nothing to open");

    assert!(services.did.lock().unwrap().is_empty());
}

#[test]
fn a_fleet_that_answers_settles_and_nothing_moves() {
    let (_home, support) = kept("b2", "b1");
    let services = Recorded::default();

    let settled = settle(&support, LABEL, Some(1), "1", &services).expect("settled");

    assert_eq!(settled, Settled::Healthy);
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("new".to_string(), "old".to_string()));
    assert_eq!(*services.did.lock().unwrap(), ["up"]);
}

#[test]
fn a_fleet_that_does_not_answer_is_marked_and_swapped_back_leaving_bridge_alone() {
    let (_home, support) = kept("b2", "b1");
    let services = Recorded { down_once: Mutex::new(Some(String::from("No new Fleet answered"))), ..Recorded::default() };

    let settled = settle(&support, LABEL, Some(1), "1", &services).expect("swapped back");

    let Settled::SwappedBack { why, reverted } = settled else { panic!("not swapped back: {settled:?}") };
    assert_eq!(why, "No new Fleet answered");
    assert!(!reverted.bridge_reopened);
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("old".to_string(), "new".to_string()));
    let builds = support.join(BUILDS);
    assert_eq!(kept_at(&builds, PREVIOUS).unwrap().unhealthy.as_deref(), Some("No new Fleet answered"));
    assert_eq!(*services.did.lock().unwrap(), ["down".to_string(), "copy".to_string(), format!("reboot {LABEL}"), "up".to_string()]);
}

#[test]
fn a_build_swapped_back_from_is_not_offered_to_revert_onto() {
    let (_home, support) = kept("b2", "b1");
    let services = Recorded { down_once: Mutex::new(Some(String::from("No new Fleet answered"))), ..Recorded::default() };
    settle(&support, LABEL, Some(1), "1", &services).expect("swapped back");

    let refused = revertible(&support, LABEL).expect_err("refused");

    assert!(refused.contains("did not come up: No new Fleet answered"), "{refused}");
}

#[test]
fn a_fleet_answers_health_only_with_200() {
    let answering = |status: &'static str| {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut stream, _) = listener.accept().unwrap();
            let mut asked = [0u8; 256];
            let _ = stream.read(&mut asked);
            let _ = stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\n\r\n").as_bytes());
        });
        port
    };
    assert!(answers_health(answering("200 OK")));
    assert!(!answers_health(answering("503 Service Unavailable")));
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    assert!(!answers_health(closed));
}
