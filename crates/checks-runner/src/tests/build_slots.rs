//! The machine's one pool of compiler jobs: its size, what a crash leaves in
//! it, and that a Check is handed it.
//!
//! Each test makes a fifo of its own under the temp directory, named for the
//! test, so none shares one.

use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::time::Duration;

use crate::run::{run_until, Writing};
use crate::{BuildSlots, Priority};

fn fifo_for(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("build-slots-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory to hold the fifo");
    dir.join("slots.fifo")
}

/// What a build that took `count` tokens and was killed leaves behind: they
/// are out of the fifo and nobody will write them back.
fn take(slots: &BuildSlots, count: usize) {
    let flags = slots.flags();
    let path = flags.rsplit("fifo:").next().expect("a path");
    let mut reader = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .expect("the fifo opens");
    let mut buffer = vec![0u8; count];
    reader
        .read_exact(&mut buffer)
        .expect("that many tokens were there");
}

#[test]
fn a_pool_holds_one_fewer_than_the_cores_so_a_lone_build_gets_them_all() {
    let mut slots = BuildSlots::open(&fifo_for("size"), 12).expect("opens");
    // The build's own implicit job is the twelfth.
    assert_eq!(slots.size(), 11);
    assert_eq!(slots.settle(|| true).expect("settles").free, 11);
}

#[test]
fn the_flags_name_the_fifo_the_way_cargo_reads_them() {
    let path = fifo_for("flags");
    let slots = BuildSlots::open(&path, 4).expect("opens");
    assert_eq!(
        slots.flags(),
        format!("-j --jobserver-auth=fifo:{}", path.display())
    );
}

#[test]
fn a_path_with_a_space_is_refused_because_makeflags_splits_on_it() {
    let path = fifo_for("space").with_file_name("a slots.fifo");
    assert!(BuildSlots::open(&path, 4).is_err());
}

#[test]
fn a_fifo_left_by_a_fleet_that_crashed_is_kept_while_builds_still_hold_tokens() {
    let path = fifo_for("crash");
    let first = BuildSlots::open(&path, 12).expect("opens");
    take(&first, 4);
    // A running build's open descriptor is what keeps the fifo's contents.
    let _build = OpenOptions::new().read(true).write(true).open(&path).expect("opens");
    drop(first);
    let mut again = BuildSlots::open_asking(&path, 12, || true).expect("opens again");
    // Four are held by builds this Fleet did not start: handing them out again
    // would put 15 jobs on 12 cores.
    assert_eq!(again.settle(|| true).expect("settles").free, 7);
}

#[test]
fn a_fifo_found_with_no_build_running_is_filled_whatever_it_held() {
    let path = fifo_for("quiet");
    let first = BuildSlots::open(&path, 12).expect("opens");
    take(&first, 4);
    drop(first);
    let mut again = BuildSlots::open_asking(&path, 12, || false).expect("opens again");
    assert_eq!(again.settle(|| true).expect("settles").free, 11);
}

#[test]
fn tokens_a_killed_build_took_stay_out_while_a_compiler_runs() {
    let mut slots = BuildSlots::open(&fifo_for("busy"), 12).expect("opens");
    take(&slots, 3);
    for _ in 0..5 {
        let looked = slots.settle(|| true).expect("settles");
        assert_eq!((looked.free, looked.restored), (8, 0));
    }
}

#[test]
fn tokens_lost_to_a_killed_build_come_back_after_two_quiet_looks() {
    let mut slots = BuildSlots::open(&fifo_for("lost"), 12).expect("opens");
    take(&slots, 3);
    // One quiet look could be the gap between a cargo exiting and its tokens
    // landing in the fifo.
    assert_eq!(slots.settle(|| false).expect("settles").restored, 0);
    let second = slots.settle(|| false).expect("settles");
    assert_eq!((second.free, second.restored), (8, 3));
    assert_eq!(slots.settle(|| true).expect("settles").free, 11);
}

#[test]
fn a_compiler_seen_between_two_quiet_looks_starts_the_count_again() {
    let mut slots = BuildSlots::open(&fifo_for("again"), 12).expect("opens");
    take(&slots, 3);
    slots.settle(|| false).expect("settles");
    slots.settle(|| true).expect("settles");
    assert_eq!(slots.settle(|| false).expect("settles").restored, 0);
}

#[test]
fn a_full_pool_does_not_ask_whether_a_compiler_runs() {
    let mut slots = BuildSlots::open(&fifo_for("full"), 12).expect("opens");
    let looked = slots
        .settle(|| panic!("a full pool has nothing to explain"))
        .expect("settles");
    assert_eq!((looked.restored, looked.trimmed), (0, 0));
}

#[test]
fn a_late_return_that_overfills_the_pool_is_trimmed() {
    let path = fifo_for("over");
    let mut slots = BuildSlots::open(&path, 12).expect("opens");
    // A build that outlived the fifo it took from writing back to this one.
    let mut writer = OpenOptions::new()
        .write(true)
        .read(true)
        .open(&path)
        .expect("opens");
    std::io::Write::write_all(&mut writer, b"+++").expect("writes");
    let looked = slots.settle(|| true).expect("settles");
    assert_eq!((looked.free, looked.trimmed), (14, 3));
    assert_eq!(slots.settle(|| true).expect("settles").free, 11);
}

#[test]
fn a_fifo_that_was_deleted_is_made_again_and_filled() {
    let path = fifo_for("gone");
    let mut slots = BuildSlots::open(&path, 12).expect("opens");
    std::fs::remove_file(&path).expect("deleted");
    let looked = slots.settle(|| true).expect("settles");
    assert!(looked.recreated);
    assert_eq!(looked.restored, 11);
    assert_eq!(slots.settle(|| true).expect("settles").free, 11);
}

async fn makeflags_seen_by_a_check(env: &[(String, String)]) -> String {
    let ran = run_until(
        "/bin/sh -c 'echo $MAKEFLAGS'",
        std::path::Path::new("/"),
        Duration::from_secs(30),
        Writing::Nowhere,
        env,
        std::future::pending(),
        Priority::Normal,
    )
    .await;
    ran.output.stdout.trim().to_string()
}

#[tokio::test]
async fn a_check_is_handed_the_pool_it_was_offered() {
    let slots = BuildSlots::open(&fifo_for("offered"), 12).expect("opens");
    crate::offer(slots.flags());
    assert_eq!(makeflags_seen_by_a_check(&[]).await, slots.flags());
}

#[tokio::test]
async fn a_checks_own_environment_still_wins_over_the_pool() {
    crate::offer("-j --jobserver-auth=fifo:/nowhere".to_string());
    let own = [("MAKEFLAGS".to_string(), "mine".to_string())];
    assert_eq!(makeflags_seen_by_a_check(&own).await, "mine");
}

#[test]
fn a_build_with_no_pool_gets_a_width_and_one_with_a_pool_gets_nothing() {
    let width = crate::CheckWidth::of(3);
    assert_eq!(
        crate::build_cap(width),
        Some(("CARGO_BUILD_JOBS".to_string(), "3".to_string()))
    );
    crate::offer("-j --jobserver-auth=fifo:/nowhere".to_string());
    assert_eq!(crate::build_cap(width), None);
}

#[test]
fn a_listing_names_a_compiler_by_the_last_part_of_its_path() {
    let named = crate::build_slots::names_a_compiler_for_test;
    assert!(named("/bin/zsh\n/Users/user/.rustup/toolchains/stable/bin/rustc\n"));
    assert!(named("cargo\n"));
    // A tool of cargo's that holds no token of its own, and a path that only
    // passes through a directory of the name.
    assert!(!named("/Users/user/.cargo/bin/cargo-nextest\n/Users/user/cargo/bin/ls\n"));
    assert!(!named(""));
}
