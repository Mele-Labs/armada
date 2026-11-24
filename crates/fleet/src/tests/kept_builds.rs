//! The swap back to the build before: two links and the launchd jobs, refused before anything
//! changes where it would move nothing. `docs/concepts/fleet.md`, *Kept builds*.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use std::time::{Duration, SystemTime};

use crate::kept_builds::{
    answers_health, kept_at, kept_tried, picked_changed, picked_in, rehearse, revert, revert_begun, revert_ended, revert_status_in,
    revertible, settle, stale_registrations, swap_ended, swap_onto, swap_status_in, swappable, trying_in, undoing, RevertStatus, Reverted, Services, Settled,
    BUILDS, CURRENT, PICKED, PREVIOUS, SNAPSHOT, STATUS, APP, FLEET, PLACED, fleet_program, placed,
};
use crate::tests::tmp::TempDir;

const LABEL: &str = "com.test.fleet";

/// launchd and the store copy, recorded.
#[derive(Default)]
struct Recorded {
    did: Mutex<Vec<String>>,
    /// Why the next Fleet asked after does not answer, once.
    down_once: Mutex<Option<String>>,
    /// The build `current` named each time Launch Services was pointed.
    pointed: Mutex<Vec<String>>,
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
    fn launch_services_pointed(&self, support: &Path) {
        self.pointed.lock().unwrap().push(on(support, CURRENT));
    }
}

fn build(builds: &Path, name: &str) {
    build_of(builds, name, &[]);
}

/// A build holding the preview's branches `jobs`, as `scripts/keep-build` writes the rows.
fn build_of(builds: &Path, name: &str, jobs: &[&str]) {
    let bundled = builds.join(name).join(APP).join("Contents/MacOS");
    std::fs::create_dir_all(&bundled).unwrap();
    std::fs::write(bundled.join("armada"), format!("fleet of {name}")).unwrap();
    std::fs::write(builds.join(name).join(APP).join("Contents/Info.plist"), format!("plist of {name}")).unwrap();
    let mut text = format!("built_at = \"2026-11-05T16:20:00Z\"\ncommit = \"{name}\"\nsource = \"main\"\n");
    for branch in jobs {
        text.push_str(&format!("\n[[job]]\nbranch = \"{branch}\"\ntip = \"0123\"\n"));
    }
    std::fs::write(builds.join(name).join("build.toml"), text).unwrap();
}

/// A support folder whose Fleet runs from the copy placed from `current`, on `new`, with `old` before it.
fn kept() -> (TempDir, PathBuf) {
    let home = TempDir::new();
    let support = home.path().join("Armada");
    let builds = support.join(BUILDS);
    build(&builds, "new");
    build(&builds, "old");
    std::os::unix::fs::symlink("new", builds.join(CURRENT)).unwrap();
    std::os::unix::fs::symlink("old", builds.join(PREVIOUS)).unwrap();
    placed(&support, "new").unwrap();
    let runs = fleet_program(&support);
    std::fs::write(support.join(format!("{LABEL}.plist")), format!("<array><string>{}</string>", runs.display())).unwrap();
    std::fs::write(support.join("armada.db"), "store").unwrap();
    (home, support)
}

fn on(support: &Path, link: &str) -> String {
    kept_at(&support.join(BUILDS), link).expect("a kept build").name
}

#[test]
fn a_revert_copies_the_store_then_swaps_the_links_and_boots_fleet() {
    let (_home, support) = kept();
    let services = Recorded::default();

    let reverted = revert(&support, LABEL, "20261105T162000Z", &services).expect("reverted");

    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("old".to_string(), "new".to_string()));
    assert_eq!(reverted.store_copy, Some(support.join("backups/armada-20261105T162000Z.db")));
    assert!(support.join("backups/armada-20261105T162000Z.db").is_file());
    let did = services.did.lock().unwrap().clone();
    assert_eq!(did, ["copy".to_string(), format!("reboot {LABEL}"), "up".to_string()]);
}

/// `kept`, with the native Bridge's job in place and each build's record carrying its `native` key
/// (`None` writes none, as a build kept without a native binary has none).
fn kept_with_native(new_native: Option<&str>, old_native: Option<&str>) -> (TempDir, PathBuf) {
    let (home, support) = kept();
    std::fs::write(support.join(format!("{LABEL}.native.plist")), "native").unwrap();
    for (name, key) in [("new", new_native), ("old", old_native)] {
        if let Some(key) = key {
            let record = support.join(BUILDS).join(name).join("build.toml");
            let mut text = std::fs::read_to_string(&record).unwrap();
            text.push_str(&format!("native = \"{key}\"\n"));
            std::fs::write(&record, text).unwrap();
        }
    }
    (home, support)
}

#[test]
fn a_revert_boots_the_native_bridge_after_fleet_where_its_app_differs() {
    let (_home, support) = kept_with_native(Some("n2"), Some("n1"));
    let services = Recorded::default();

    let reverted = revert(&support, LABEL, "1", &services).expect("reverted");

    assert!(reverted.native_reopened);
    let did = services.did.lock().unwrap().clone();
    assert_eq!(did, ["copy".to_string(), format!("reboot {LABEL}"), "up".to_string(), format!("reboot {LABEL}.native")]);
}

#[test]
fn the_native_bridge_is_left_open_where_both_builds_keep_the_same_app() {
    let (_home, support) = kept_with_native(Some("n"), Some("n"));
    let services = Recorded::default();

    let reverted = revert(&support, LABEL, "1", &services).expect("reverted");

    assert!(!reverted.native_reopened);
    assert!(!services.did.lock().unwrap().iter().any(|one| one.ends_with(".native")));
}

#[test]
fn the_native_bridge_is_booted_only_onto_a_build_that_keeps_one() {
    // The build reverted to has no app: the job would start on an `Armada.app` that is not there.
    let (_home, support) = kept_with_native(Some("n2"), None);
    let services = Recorded::default();
    assert!(!revert(&support, LABEL, "1", &services).expect("reverted").native_reopened);
    assert!(!services.did.lock().unwrap().iter().any(|one| one.ends_with(".native")));

    // The build reverted to has one and the running build had none.
    let (_home, support) = kept_with_native(None, Some("n1"));
    assert!(revert(&support, LABEL, "1", &Recorded::default()).expect("reverted").native_reopened);
}

#[test]
fn a_mac_that_never_opened_the_native_bridge_has_no_job_to_boot() {
    let (_home, support) = kept_with_native(Some("n2"), Some("n1"));
    std::fs::remove_file(support.join(format!("{LABEL}.native.plist"))).unwrap();
    let services = Recorded::default();

    assert!(!revert(&support, LABEL, "1", &services).expect("reverted").native_reopened);
    assert!(!services.did.lock().unwrap().iter().any(|one| one.ends_with(".native")));
}

#[test]
fn a_swap_onto_a_snapshot_boots_the_native_bridge_too_but_a_swap_back_leaves_it_alone() {
    let (_home, support) = with_snapshot(&["armada/a"]);
    std::fs::write(support.join(format!("{LABEL}.native.plist")), "native").unwrap();
    for (name, key) in [("new", "n1"), ("snap", "n2")] {
        let record = support.join(BUILDS).join(name).join("build.toml");
        let mut text = std::fs::read_to_string(&record).unwrap();
        text.insert_str(text.find("\n[").map_or(text.len(), |at| at + 1), &format!("native = \"{key}\"\n"));
        std::fs::write(&record, text).unwrap();
    }
    let services = Recorded::default();
    swap_onto(&support, LABEL, "1", &services).expect("swapped");
    assert!(services.did.lock().unwrap().contains(&format!("reboot {LABEL}.native")));

    // The same swap, with a Fleet that does not answer, is swapped back from and Bridge stays.
    let (_home, support) = with_snapshot(&["armada/a"]);
    std::fs::write(support.join(format!("{LABEL}.native.plist")), "native").unwrap();
    let services = Recorded::default();
    *services.down_once.lock().unwrap() = Some(String::from("Fleet did not answer"));
    swap_onto(&support, LABEL, "1", &services).expect("swapped back");
    assert!(!services.did.lock().unwrap().iter().any(|one| one.ends_with(".native")));
}

#[test]
fn a_revert_places_the_build_reverted_to_at_the_one_path_launchd_runs() {
    let (_home, support) = kept();
    let program = fleet_program(&support);
    assert_eq!(std::fs::read_to_string(&program).unwrap(), "fleet of new");
    let running = std::fs::File::open(&program).unwrap();
    let extra = support.join(FLEET).join(APP).join("Contents/_CodeSignature/CodeResources");
    std::fs::create_dir_all(extra.parent().unwrap()).unwrap();
    std::fs::write(&extra, "a file only the running bundle has").unwrap();

    revert(&support, LABEL, "1", &Recorded::default()).expect("reverted");

    assert_eq!(program, support.join("fleet/Armada Fleet.app/Contents/MacOS/armada"), "one path, whichever build");
    assert_eq!(std::fs::read_to_string(&program).unwrap(), "fleet of old");
    assert_eq!(std::fs::read_to_string(support.join(FLEET).join(APP).join("Contents/Info.plist")).unwrap(), "plist of old");
    assert!(!extra.exists(), "a file the placed bundle does not have is removed");
    assert_eq!(std::fs::read_to_string(support.join(FLEET).join(PLACED)).unwrap(), "old\n");
    assert!(!std::fs::symlink_metadata(&program).unwrap().file_type().is_symlink(), "a real file, since macOS records the resolved path");
    let mut was = String::new();
    std::io::Read::read_to_string(&mut &running, &mut was).unwrap();
    assert_eq!(was, "fleet of new", "the running Fleet keeps the file it was started from");
    let leftovers: Vec<_> = std::fs::read_dir(support.join(FLEET)).unwrap().map(|one| one.unwrap().file_name()).collect();
    assert_eq!(leftovers.len(), 2, "{leftovers:?}");
}

#[test]
fn a_build_kept_without_fleets_bundle_is_refused_and_nothing_moves() {
    let (_home, support) = kept();
    std::fs::remove_dir_all(support.join(BUILDS).join("old").join(APP)).unwrap();
    let services = Recorded::default();

    let refused = revert(&support, LABEL, "1", &services).expect_err("refused");

    assert!(refused.contains("without its Armada Fleet.app"), "{refused}");
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("new".to_string(), "old".to_string()));
    assert_eq!(std::fs::read_to_string(fleet_program(&support)).unwrap(), "fleet of new");
    assert!(services.did.lock().unwrap().is_empty());
}

#[test]
fn a_second_revert_swaps_back_onto_the_build_reverted_from() {
    let (_home, support) = kept();
    revert(&support, LABEL, "1", &Recorded::default()).expect("reverted");

    revert(&support, LABEL, "2", &Recorded::default()).expect("reverted again");

    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("new".to_string(), "old".to_string()));
}

#[test]
fn the_old_electron_bridge_job_is_never_booted_even_where_a_record_carries_its_hashes() {
    let (_home, support) = kept_with_native(Some("n"), Some("n"));
    std::fs::write(support.join(format!("{LABEL}.bridge.plist")), "bridge").unwrap();
    for (name, hash) in [("new", "b2"), ("old", "b1")] {
        let record = support.join(BUILDS).join(name).join("build.toml");
        let mut text = std::fs::read_to_string(&record).unwrap();
        text.push_str(&format!("bridge = \"{hash}\"\nbridge_shell = \"{hash}\"\nelectron = \"{hash}\"\npage_reload = true\n"));
        std::fs::write(&record, text).unwrap();
    }
    let services = Recorded::default();

    revert(&support, LABEL, "1", &services).expect("reverted");

    assert_eq!(*services.did.lock().unwrap(), ["copy".to_string(), format!("reboot {LABEL}"), "up".to_string()]);
}

#[test]
fn a_record_kept_with_electron_rows_still_reads() {
    let (_home, support) = kept();
    let builds = support.join(BUILDS);
    let record = builds.join("new").join("build.toml");
    let mut text = std::fs::read_to_string(&record).unwrap();
    text.insert_str(text.find('\n').unwrap() + 1, "bridge = \"b2\"\nbridge_shell = \"s2\"\nelectron = \"40.1.0\"\npage_reload = true\n");
    std::fs::write(&record, text).unwrap();

    let read = kept_at(&builds, CURRENT).expect("an old record still reads");

    assert_eq!((read.name.as_str(), read.commit.as_str(), read.source.as_str()), ("new", "new", "main"));
}

#[test]
fn no_previous_build_is_refused_and_nothing_moves() {
    let (_home, support) = kept();
    std::fs::remove_file(support.join(BUILDS).join(PREVIOUS)).unwrap();
    let services = Recorded::default();

    let refused = revert(&support, LABEL, "1", &services).expect_err("refused");

    assert!(refused.contains("No build before this one"), "{refused}");
    assert_eq!(on(&support, CURRENT), "new");
    assert!(services.did.lock().unwrap().is_empty());
}

#[test]
fn a_fleet_launchd_does_not_run_from_the_kept_builds_is_refused_and_nothing_moves() {
    let (_home, support) = kept();
    std::fs::write(support.join(format!("{LABEL}.plist")), "<string>/Users/user/.cargo/bin/armada</string>").unwrap();
    let services = Recorded::default();

    let refused = revert(&support, LABEL, "1", &services).expect_err("refused");

    assert!(refused.contains("does not run"), "{refused}");
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("new".to_string(), "old".to_string()));
    assert!(services.did.lock().unwrap().is_empty());
}

#[test]
fn a_build_names_the_branches_of_its_job_rows_and_a_main_build_has_none() {
    let (_home, support) = kept();
    let builds = support.join(BUILDS);
    build_of(&builds, "new", &["armada/a", "armada/b"]);

    assert_eq!(kept_at(&builds, CURRENT).unwrap().jobs, ["armada/a", "armada/b"]);
    assert!(kept_at(&builds, PREVIOUS).unwrap().jobs.is_empty());
}

#[test]
fn a_revert_takes_out_the_branches_only_the_running_build_holds_and_brings_back_the_reverse() {
    let (_home, support) = kept();
    let builds = support.join(BUILDS);
    build_of(&builds, "new", &["armada/a", "armada/both"]);
    build_of(&builds, "old", &["armada/both", "armada/c"]);
    let (from, to) = revertible(&support, LABEL).expect("revertible");

    let undo = undoing(&from, &to);

    assert_eq!(undo.takes_out, ["armada/a"]);
    assert_eq!(undo.brings_back, ["armada/c"]);
    assert_eq!((undo.built_at.as_str(), undo.commit.as_str()), ("2026-11-05T16:20:00Z", "old"));
}

#[test]
fn bridge_reopens_follows_the_native_apps_key_where_the_build_reverted_to_keeps_one() {
    let reopens = |new: Option<&str>, old: Option<&str>| {
        let (_home, support) = kept_with_native(new, old);
        let (from, to) = revertible(&support, LABEL).expect("revertible");
        undoing(&from, &to).bridge_reopens
    };
    assert!(reopens(Some("n2"), Some("n1")), "differ");
    assert!(!reopens(Some("n"), Some("n")), "equal");
    assert!(reopens(None, Some("n1")), "the running build keeps none");
    assert!(!reopens(Some("n2"), None), "the previous build keeps none to boot onto");
    assert!(!reopens(None, None), "neither keeps one");
}

#[test]
fn revertible_is_the_check_revert_makes_and_names_why_not() {
    let (_home, support) = kept();
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
    let (_home, support) = kept();
    revert_begun(&support).unwrap();
    let now = SystemTime::now();

    assert_eq!(revert_status_in(&support, now), Some(RevertStatus::Running));
    assert_eq!(revert_status_in(&support, now + Duration::from_secs(119)), Some(RevertStatus::Running));
    assert_eq!(revert_status_in(&support, now + Duration::from_secs(3 * 60)), None);
}

#[test]
fn a_revert_that_took_removes_its_status_and_one_that_failed_keeps_one_line_until_the_build_moves() {
    let (_home, support) = kept();
    revert_begun(&support).unwrap();
    let builds = support.join(BUILDS);
    let took = Reverted {
        from: kept_at(&builds, CURRENT).unwrap(),
        to: kept_at(&builds, PREVIOUS).unwrap(),
        native_reopened: false,
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
    let (_home, support) = kept();
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
    let (_home, support) = kept();

    let refused = rehearse(&support, &Recorded::default()).expect_err("refused");

    assert!(refused.contains("could not open a copy of the store"), "{refused}");
    assert_eq!(std::fs::read_to_string(support.join("armada.db")).unwrap(), "store");
}

#[test]
fn a_rehearsal_with_no_store_has_nothing_to_open() {
    let (_home, support) = kept();
    std::fs::remove_file(support.join("armada.db")).unwrap();
    let services = Recorded::default();

    rehearse(&support, &services).expect("nothing to open");

    assert!(services.did.lock().unwrap().is_empty());
}

#[test]
fn a_fleet_that_answers_settles_and_nothing_moves() {
    let (_home, support) = kept();
    let services = Recorded::default();

    let settled = settle(&support, LABEL, Some(1), "1", &services).expect("settled");

    assert_eq!(settled, Settled::Healthy);
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("new".to_string(), "old".to_string()));
    assert_eq!(*services.did.lock().unwrap(), ["up"]);
}

#[test]
fn a_fleet_that_does_not_answer_is_marked_and_swapped_back_leaving_the_native_bridge_alone() {
    let (_home, support) = kept();
    let services = Recorded { down_once: Mutex::new(Some(String::from("No new Fleet answered"))), ..Recorded::default() };

    let settled = settle(&support, LABEL, Some(1), "1", &services).expect("swapped back");

    let Settled::SwappedBack { why, reverted } = settled else { panic!("not swapped back: {settled:?}") };
    assert_eq!(why, "No new Fleet answered");
    assert!(!reverted.native_reopened);
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("old".to_string(), "new".to_string()));
    let builds = support.join(BUILDS);
    assert_eq!(kept_at(&builds, PREVIOUS).unwrap().unhealthy.as_deref(), Some("No new Fleet answered"));
    assert_eq!(*services.did.lock().unwrap(), ["down".to_string(), "copy".to_string(), format!("reboot {LABEL}"), "up".to_string()]);
}

#[test]
fn a_build_swapped_back_from_is_not_offered_to_revert_onto() {
    let (_home, support) = kept();
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

/// `kept` with a third build, `snap`, made by `scripts/restart --keep-only` from a preview of `jobs`.
fn with_snapshot(jobs: &[&str]) -> (TempDir, PathBuf) {
    let (home, support) = kept();
    let builds = support.join(BUILDS);
    build_of(&builds, "snap", jobs);
    let record = builds.join("snap").join("build.toml");
    let text = std::fs::read_to_string(&record).unwrap().replace("source = \"main\"", "source = \"preview\"\ntree = \"/repo/.armada/preview\"");
    std::fs::write(&record, text).unwrap();
    std::fs::write(support.join(SNAPSHOT), "snap\n").unwrap();
    (home, support)
}

#[test]
fn a_swap_moves_onto_the_snapshot_keeping_the_running_build_as_previous() {
    let (_home, support) = with_snapshot(&["armada/a"]);
    std::fs::write(support.join("snapshot-held"), "snap\nmain1\narmada/a\t0123\n").unwrap();
    let services = Recorded::default();

    let settled = swap_onto(&support, LABEL, "1", &services).expect("swapped");

    assert_eq!(settled, Settled::Healthy);
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("snap".to_string(), "new".to_string()));
    assert!(!support.join(SNAPSHOT).exists());
    let did = services.did.lock().unwrap().clone();
    assert_eq!(did, ["copy".to_string(), format!("reboot {LABEL}"), "up".to_string()]);
    assert_eq!(std::fs::read_to_string(support.join("restart-commit")).unwrap(), "snap\n");
    assert_eq!(std::fs::read_to_string(support.join("restart-source")).unwrap(), "/repo/.armada/preview\n");
    assert!(std::fs::read_to_string(support.join("preview-held")).unwrap().starts_with("snap\n"));
}

#[test]
fn a_swap_whose_fleet_does_not_answer_is_marked_and_swapped_back_leaving_the_native_bridge_alone() {
    let (_home, support) = with_snapshot(&["armada/a"]);
    let services = Recorded { down_once: Mutex::new(Some(String::from("No new Fleet answered"))), ..Recorded::default() };

    let settled = swap_onto(&support, LABEL, "1", &services).expect("swapped back");

    let Settled::SwappedBack { why, reverted } = settled else { panic!("not swapped back: {settled:?}") };
    assert_eq!(why, "No new Fleet answered");
    assert!(!reverted.native_reopened);
    assert_eq!((on(&support, CURRENT), on(&support, PREVIOUS)), ("new".to_string(), "snap".to_string()));
    assert_eq!(kept_at(&support.join(BUILDS), PREVIOUS).unwrap().unhealthy.as_deref(), Some("No new Fleet answered"));
    assert!(!services.did.lock().unwrap().iter().any(|one| one.ends_with(".native")));
    assert_eq!(std::fs::read_to_string(support.join("restart-commit")).unwrap(), "new\n");
}

#[test]
fn a_swap_with_no_snapshot_or_onto_the_running_build_is_refused_and_nothing_moves() {
    let (_home, support) = kept();
    let services = Recorded::default();
    assert!(swap_onto(&support, LABEL, "1", &services).unwrap_err().contains("No build is kept to swap onto"));

    std::fs::write(support.join(SNAPSHOT), "new\n").unwrap();
    assert!(swappable(&support, LABEL).unwrap_err().contains("is the one running"));
    assert!(services.did.lock().unwrap().is_empty());
    assert_eq!(on(&support, CURRENT), "new");
}

#[test]
fn a_swap_undone_by_itself_is_kept_as_a_failure_until_the_build_moves() {
    let (_home, support) = with_snapshot(&[]);
    let services = Recorded { down_once: Mutex::new(Some(String::from("No new Fleet answered"))), ..Recorded::default() };
    let outcome = swap_onto(&support, LABEL, "1", &services);

    swap_ended(&support, &outcome);

    assert_eq!(
        swap_status_in(&support, SystemTime::now()),
        Some(RevertStatus::Failed(String::from("Swapped back, since the new Fleet did not answer: No new Fleet answered")))
    );
}

#[test]
fn the_picked_branches_start_as_the_running_builds_and_follow_each_pick() {
    let (_home, support) = kept();
    build_of(&support.join(BUILDS), "new", &["armada/a"]);
    assert_eq!(picked_in(&support), ["armada/a"]);

    assert_eq!(picked_changed(&support, "armada/b", true).unwrap(), ["armada/a", "armada/b"]);
    assert_eq!(picked_changed(&support, "armada/a", false).unwrap(), ["armada/b"]);
    assert_eq!(picked_changed(&support, "armada/b", false).unwrap(), Vec::<String>::new());

    assert_eq!(std::fs::read_to_string(support.join(PICKED)).unwrap(), "");
    assert!(picked_in(&support).is_empty(), "an empty file is none picked, not the running build's");
}

#[test]
fn the_running_builds_branches_are_tried_until_keep_is_pressed_on_it() {
    let (_home, support) = kept();
    build_of(&support.join(BUILDS), "new", &["armada/a"]);
    assert_eq!(trying_in(&support), ["armada/a"]);

    assert_eq!(kept_tried(&support).unwrap(), ["armada/a"]);

    assert!(trying_in(&support).is_empty());
}

#[test]
fn launch_services_is_pointed_at_the_build_each_revert_or_swap_leaves_current() {
    let (_home, support) = kept();
    let services = Recorded::default();
    revert(&support, LABEL, "1", &services).expect("reverted");
    assert_eq!(*services.pointed.lock().unwrap(), ["old"]);

    let (_home, support) = with_snapshot(&[]);
    let services = Recorded::default();
    swap_onto(&support, LABEL, "1", &services).expect("swapped");
    assert_eq!(*services.pointed.lock().unwrap(), ["snap"]);

    let (_home, support) = with_snapshot(&[]);
    let services = Recorded::default();
    *services.down_once.lock().unwrap() = Some(String::from("Fleet did not answer"));
    swap_onto(&support, LABEL, "1", &services).expect("swapped back");
    assert_eq!(*services.pointed.lock().unwrap(), ["new"], "once, after the swap back");
}

#[test]
fn every_bundle_registered_under_builds_is_stale_but_the_kept_ones_and_whats_inside_them() {
    let builds = Path::new("/u/Armada/builds");
    let current = builds.join("c/Armada.app");
    let running = builds.join("b/Armada.app");
    let dump = "\
path:                       /u/Armada/builds/a/Armada.app (0x4344)
path:                       /u/Armada/builds/a/Armada.app/Contents/Helpers/Armada Voice.app (0x4618)
path:                       /u/Armada/builds/a/Armada.app (0x4345)
path:                       /u/Armada/builds/b/Armada.app (0x4350)
path:                       /u/Armada/builds/c/Armada Fleet.app (0x48cc)
path:                       /u/Armada/builds/c/Armada.app (0x4958)
path:                       /u/Armada/builds/c/Armada.app/Contents/Helpers/Armada Voice.app (0x4959)
path:                       /u/Armada/builds/c/Armada.app2 (0x4960)
path:                       /u/Armada/fleet/Armada Fleet.app (0x4914)
path:                       /private/tmp/armada-red/e2e/out/Armada.app (0x4578)
name:                       /u/Armada/builds/z/Armada.app (0x1)
";
    let stale = stale_registrations(dump, builds, &[current, running]);
    assert_eq!(
        stale,
        [
            builds.join("a/Armada.app"),
            builds.join("a/Armada.app/Contents/Helpers/Armada Voice.app"),
            builds.join("c/Armada Fleet.app"),
            builds.join("c/Armada.app2"),
        ]
    );
}
