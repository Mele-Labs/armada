//! What Fleet drops when the disk runs short, and what it never does: every
//! build folder is found, a free slot sheds its builds by age, and a short disk
//! is reclaimed in a fixed order that stops once there is room.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::leasing::disk::{
    build_folders_in, drop_first, every_checkout, inventory, pressure_plan, reclaim, shed_free_slots,
    staged_plan, Act, BuildFolder, How, Role, MARGIN,
};
use crate::leasing::{Holder, Leased, Pool};
use crate::tests::repo::TempRepo;

const HOUR: Duration = Duration::from_secs(60 * 60);
const GIB: u64 = 1024 * 1024 * 1024;

fn a_repository() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\n", "ignore builds");
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    repo
}

fn lease_for(pool: &Pool, holder: &Holder, branch: &str) -> PathBuf {
    let no_seed = |_: &Path, _: &Path| Err(String::from("nothing to seed from"));
    match pool.try_lease(branch, holder, 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("took no slot: {other:?}"),
    }
}

fn this_process() -> Holder {
    Holder::of(std::process::id()).expect("this process is running")
}

/// Whoever started the test run, a second live process to hold a slot.
fn another_process() -> Holder {
    Holder::of(std::os::unix::process::parent_id()).expect("the parent is running")
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().expect("the path exists")
}

/// A Cargo build folder at `target`: the tag Cargo writes, and one artifact
/// in each place a build writes, all last touched `hours` ago.
fn built(target: &Path, hours: u64) {
    std::fs::create_dir_all(target).unwrap();
    std::fs::write(target.join("CACHEDIR.TAG"), "Signature: 8a477f597d28d172789f06886806bc55\n").unwrap();
    for file in [
        "debug/deps/libfleet-1.rlib",
        "debug/.fingerprint/fleet-1/lib-fleet",
        "debug/incremental/fleet-1/s-1/dep-graph.bin",
    ] {
        let file = target.join(file);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, [0u8; 64]).unwrap();
    }
    let then = SystemTime::now() - HOUR * u32::try_from(hours).unwrap();
    touch_all(&target.join("debug"), then);
}

fn touch_all(at: &Path, then: SystemTime) {
    if at.is_dir() {
        for entry in std::fs::read_dir(at).unwrap().flatten() {
            touch_all(&entry.path(), then);
        }
    }
    std::fs::File::open(at).unwrap().set_modified(then).unwrap();
}

/// A build holds cargo's lock on its profile directory.
fn building_in(target: &Path) -> std::fs::File {
    let lock = target.join("debug/.cargo-lock");
    std::fs::write(&lock, "").unwrap();
    let file = std::fs::File::options().write(true).open(&lock).unwrap();
    file.lock().unwrap();
    file
}

fn said_into(lines: &mut Vec<String>) -> impl FnMut(&str) + '_ {
    move |line| lines.push(line.to_string())
}

#[test]
fn a_nested_build_folder_with_a_cache_tag_is_found_and_a_lookalike_is_not() {
    let repo = TempRepo::with_a_commit();
    let root = repo.root();
    built(&root.join("target"), 1);
    built(&root.join("crates/bridge-native/target"), 1);
    // Named target, no tag: not Cargo's.
    std::fs::create_dir_all(root.join("docs/target/inside")).unwrap();
    // Inside node_modules, and past the third level: not looked at.
    built(&root.join("node_modules/pkg/target"), 1);
    built(&root.join("a/b/c/target"), 1);
    // A build folder inside a build folder is the outer one's.
    built(&root.join("target/debug/nested/target"), 1);

    let mut found = build_folders_in(root);
    found.sort();

    assert_eq!(
        found,
        vec![
            root.join("crates/bridge-native/target"),
            root.join("target"),
        ]
    );
}

#[test]
fn a_worktree_git_cut_outside_armada_is_a_checkout_and_a_slot_is_one_checkout_not_two() {
    let repo = a_repository();
    let root = repo.root();
    let pool = Pool::at(root, 1, "main", Vec::new());
    let slot = lease_for(&pool, &this_process(), "a-branch");
    let outside = tempfile_dir("armada-red-worktree");
    repo.git(&["worktree", "add", "-q", "-b", "elsewhere", outside.to_str().unwrap()]);
    built(&outside.join("target"), 1);

    let checkouts = every_checkout(root);

    assert!(checkouts.contains(&canonical(&outside)), "{checkouts:?}");
    assert!(checkouts.contains(&canonical(root)));
    assert_eq!(
        checkouts.iter().filter(|one| **one == canonical(&slot)).count(),
        1,
        "listed by the slots folder and by git, and counted once"
    );
    std::fs::remove_dir_all(&outside).ok();
}

fn tempfile_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("{name}-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::remove_dir_all(&path).ok();
    path
}

#[test]
fn a_free_slot_unbuilt_for_three_hours_loses_its_incremental_caches_and_nothing_else() {
    let repo = a_repository();
    let root = repo.root();
    let pool = Pool::at(root, 2, "main", Vec::new());
    let held = lease_for(&pool, &another_process(), "held-branch");
    let free = lease_for(&pool, &this_process(), "free-branch");
    pool.release(&free).expect("a clean slot is released");
    built(&free.join("target"), 3);
    built(&held.join("target"), 3);
    built(&root.join("target"), 3);

    let folders = inventory(root, "main", &[]);
    shed_free_slots(&folders, SystemTime::now(), &mut |_| {});

    assert!(!free.join("target/debug/incremental").exists());
    assert!(free.join("target/debug/deps/libfleet-1.rlib").exists());
    assert!(held.join("target/debug/incremental").exists(), "a held slot is not shed");
    assert!(root.join("target/debug/incremental").exists(), "nor is the main checkout");
}

#[test]
fn a_free_slot_unbuilt_for_a_day_loses_its_whole_build_folder_and_says_so() {
    let repo = a_repository();
    let root = repo.root();
    let pool = Pool::at(root, 2, "main", Vec::new());
    let held = lease_for(&pool, &another_process(), "held-branch");
    let free = lease_for(&pool, &this_process(), "free-branch");
    pool.release(&free).expect("a clean slot is released");
    built(&free.join("target"), 25);
    built(&held.join("target"), 25);
    built(&root.join("target"), 25);

    let mut lines = Vec::new();
    let folders = inventory(root, "main", &[]);
    shed_free_slots(&folders, SystemTime::now(), &mut said_into(&mut lines));

    assert!(!free.join("target").exists());
    assert!(held.join("target").exists());
    assert!(root.join("target").exists());
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("slot-"), "{lines:?}");
}

#[test]
fn a_folder_a_person_holds_is_left_by_every_pass() {
    let repo = a_repository();
    let root = repo.root();
    let pool = Pool::at(root, 1, "main", Vec::new());
    let free = lease_for(&pool, &this_process(), "free-branch");
    pool.release(&free).expect("a clean slot is released");
    built(&free.join("target"), 25);

    let folders = inventory(root, "main", &[free.clone()]);
    shed_free_slots(&folders, SystemTime::now(), &mut |_| {});
    reclaim(&folders, SystemTime::now(), 100 * GIB, &mut || Some(1), &mut |_| {});

    assert!(free.join("target/debug/incremental").exists());
}

fn folder(name: &str, role: Role, hours: f64, incremental: bool, landed: bool) -> BuildFolder {
    BuildFolder {
        path: PathBuf::from(format!("/disk/{name}/target")),
        checkout: PathBuf::from(format!("/disk/{name}")),
        role,
        last_built: SystemTime::now() - Duration::from_secs_f64(hours * 3600.0),
        has_incremental: incremental,
        landed,
    }
}

fn plan_of(acts: &[Act]) -> Vec<(How, String)> {
    acts.iter()
        .map(|act| {
            let name = act.folder.to_string_lossy().replace("/disk/", "").replace("/target", "");
            (act.how, name)
        })
        .collect()
}

fn incremental(name: &str) -> (How, String) {
    (How::Incremental, name.to_string())
}

fn whole(name: &str) -> (How, String) {
    (How::Whole, name.to_string())
}

#[test]
fn a_short_disk_is_reclaimed_incremental_first_then_landed_then_free_then_idle_held() {
    let folders = [
        folder("main", Role::Main, 10.0, true, true),
        folder("preview", Role::Preview, 10.0, true, true),
        folder("free-old", Role::FreeSlot, 30.0, true, true),
        folder("free-new", Role::FreeSlot, 3.0, true, false),
        folder("held-idle", Role::HeldSlot, 8.0, true, false),
        folder("held-busy", Role::HeldSlot, 2.0, true, false),
        folder("job-landed", Role::Other, 50.0, true, true),
        folder("job-open", Role::Other, 100.0, true, false),
        folder("job-just-built", Role::Other, 0.5, true, false),
        folder("job-no-cache", Role::Other, 200.0, false, false),
    ];

    let plan = plan_of(&pressure_plan(&folders, SystemTime::now()));

    assert_eq!(
        plan,
        vec![
            // 1. Incremental caches, not built in the last hour, oldest first.
            incremental("job-open"),
            incremental("job-landed"),
            incremental("free-old"),
            incremental("main"),
            incremental("preview"),
            incremental("held-idle"),
            incremental("free-new"),
            incremental("held-busy"),
            // 2. Checkouts whose HEAD the base already has, oldest first.
            whole("job-landed"),
            whole("free-old"),
            // 3. Free slots, least recently built first.
            whole("free-new"),
            // 4. Held slots idle for six hours.
            whole("held-idle"),
        ]
    );
}

#[test]
fn the_main_checkout_and_the_preview_are_never_removed_whole() {
    let folders = [
        folder("main", Role::Main, 500.0, true, true),
        folder("preview", Role::Preview, 500.0, true, true),
    ];

    let plan = plan_of(&pressure_plan(&folders, SystemTime::now()));

    assert_eq!(plan, vec![incremental("main"), incremental("preview")]);
    assert!(staged_plan(&folders, SystemTime::now()).is_empty());
}

#[test]
fn a_pass_stops_at_the_floor_plus_twenty_gibibytes_and_not_before() {
    let repo = TempRepo::with_a_commit();
    let root = repo.root();
    let at = |name: &str, hours| {
        let checkout = root.join(".armada/worktrees").join(name);
        built(&checkout.join("target"), hours);
        BuildFolder {
            path: checkout.join("target"),
            checkout,
            role: Role::Other,
            last_built: SystemTime::now() - HOUR * u32::try_from(hours).unwrap(),
            has_incremental: true,
            landed: true,
        }
    };
    let folders = [at("oldest", 90), at("older", 80), at("old", 70)];
    // The reading before the pass, and after each act.
    let mut script = vec![90 * GIB, 95 * GIB, 100 * GIB + MARGIN].into_iter();

    let mut lines = Vec::new();
    reclaim(
        &folders,
        SystemTime::now(),
        100 * GIB,
        &mut || script.next(),
        &mut said_into(&mut lines),
    );

    let kept = |name: &str| root.join(".armada/worktrees").join(name).join("target/debug/incremental").exists();
    assert!(!kept("oldest") && !kept("older"), "two acts reached the line");
    assert!(kept("old"), "and the third was left");
    for name in ["oldest", "older", "old"] {
        assert!(root.join(".armada/worktrees").join(name).join("target/debug/deps").exists());
    }
    assert_eq!(lines.len(), 1, "one line for the pass: {lines:?}");
}

#[test]
fn a_disk_above_the_floor_loses_nothing() {
    let repo = TempRepo::with_a_commit();
    let target = repo.root().join(".armada/worktrees/a/target");
    built(&target, 90);
    let folders = [BuildFolder {
        path: target.clone(),
        checkout: target.parent().unwrap().to_path_buf(),
        role: Role::Other,
        last_built: SystemTime::now() - HOUR * 90,
        has_incremental: true,
        landed: true,
    }];

    reclaim(&folders, SystemTime::now(), 100 * GIB, &mut || Some(100 * GIB), &mut |_| {});

    assert!(target.join("debug/incremental").exists());
}

#[test]
fn a_whole_removal_under_pressure_says_which_folder_and_a_building_one_is_never_touched() {
    let repo = TempRepo::with_a_commit();
    let root = repo.root();
    let making = |name: &str| {
        let checkout = root.join(".armada/worktrees").join(name);
        built(&checkout.join("target"), 90);
        BuildFolder {
            path: checkout.join("target"),
            checkout,
            role: Role::Other,
            last_built: SystemTime::now() - HOUR * 90,
            has_incremental: true,
            landed: true,
        }
    };
    let busy = making("busy");
    let idle = making("idle");
    let _lock = building_in(&busy.path);
    let mut script = (0..16).map(|_| 10 * GIB);

    let mut lines = Vec::new();
    reclaim(
        &[busy.clone(), idle.clone()],
        SystemTime::now(),
        100 * GIB,
        &mut || script.next(),
        &mut said_into(&mut lines),
    );

    assert!(busy.path.join("debug/incremental").exists());
    assert!(busy.path.join("debug/deps").exists());
    assert!(!idle.path.exists(), "the idle one went whole");
    assert!(
        lines.iter().any(|line| line.contains(idle.path.to_str().unwrap())),
        "{lines:?}"
    );
}

/// A worktree's build folder, built `hours` ago, whose checkout the base has.
fn a_landed_worktree(root: &Path, name: &str, hours: u64) -> BuildFolder {
    let checkout = root.join(".armada/worktrees").join(name);
    built(&checkout.join("target"), hours);
    BuildFolder {
        path: checkout.join("target"),
        checkout,
        role: Role::Other,
        last_built: SystemTime::now() - HOUR * u32::try_from(hours).unwrap(),
        has_incremental: true,
        landed: true,
    }
}

#[test]
fn a_pass_says_what_it_gained_and_names_the_folders_it_left_in_the_order_the_next_would_drop_them() {
    let repo = TempRepo::with_a_commit();
    let root = repo.root();
    let folders = [a_landed_worktree(&root, "oldest", 90), a_landed_worktree(&root, "older", 80), a_landed_worktree(&root, "old", 70)];
    let mut script = vec![90 * GIB, 95 * GIB, 100 * GIB + MARGIN].into_iter();

    let passed = reclaim(&folders, SystemTime::now(), 100 * GIB, &mut || script.next(), &mut |_| {}).expect("a pass ran");

    assert_eq!((passed.acts, passed.freed), (2, 10 * GIB + MARGIN));
    let names = |folders: &[BuildFolder]| folders.iter().map(|folder| folder.path.clone()).collect::<Vec<_>>();
    let [oldest, older, old] = <[PathBuf; 3]>::try_from(names(&folders)).unwrap();
    assert_eq!(passed.left, vec![old, oldest, older], "the third's caches, then each folder whole");
}

#[test]
fn a_disk_with_room_has_no_pass_to_report() {
    let repo = TempRepo::with_a_commit();
    let folders = [a_landed_worktree(&repo.root(), "a", 90)];

    assert_eq!(reclaim(&folders, SystemTime::now(), 100 * GIB, &mut || Some(150 * GIB), &mut |_| {}), None);
}

#[test]
fn dropping_the_first_folders_of_the_plan_takes_what_a_pass_would_and_leaves_the_rest() {
    let repo = TempRepo::with_a_commit();
    let root = repo.root();
    let folders = [a_landed_worktree(&root, "oldest", 90), a_landed_worktree(&root, "older", 80)];

    let dropped = drop_first(&folders, SystemTime::now(), 1);

    assert_eq!(dropped, vec![folders[0].path.clone()]);
    assert!(!folders[0].path.join("debug/incremental").exists(), "the plan's first act on it was its caches");
    assert!(folders[0].path.join("debug/deps").exists());
    assert!(folders[1].path.join("debug/incremental").exists());
}

#[test]
fn dropping_leaves_a_folder_a_build_holds() {
    let repo = TempRepo::with_a_commit();
    let busy = a_landed_worktree(&repo.root(), "busy", 90);
    let _lock = building_in(&busy.path);

    assert!(drop_first(std::slice::from_ref(&busy), SystemTime::now(), 3).is_empty());
    assert!(busy.path.join("debug/incremental").exists());
}
