//! `armada disk reclaim`: one pass on the folders it is pointed at, with no Fleet, and the forms
//! the verb takes.

use std::cell::Cell;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::TempDir;
use crate::cli::{read, Misread, Verb};
use crate::disk::{reach, reclaim_now};

const GIB: u64 = 1024 * 1024 * 1024;

fn asked(line: &str) -> Result<Verb, Misread> {
    read(line.split_whitespace().map(str::to_string))
}

/// A repository root with one build folder, idle for ten hours, with caches to drop.
fn a_repository_with_a_build() -> TempDir {
    let root = TempDir::new();
    root.write(".git/HEAD", "ref: refs/heads/main\n");
    let target = root.path().join("target");
    for file in ["debug/deps/a.rlib", "debug/.fingerprint/a/lib-a", "debug/incremental/a/s-1/dep-graph.bin"] {
        root.write(&format!("target/{file}"), "0");
    }
    root.write("target/CACHEDIR.TAG", "Signature: 8a477f597d28d172789f06886806bc55\n");
    let long_ago = SystemTime::now() - Duration::from_secs(10 * 3600);
    touch_all(&target.join("debug"), long_ago);
    root
}

fn touch_all(at: &std::path::Path, then: SystemTime) {
    if at.is_dir() {
        for entry in std::fs::read_dir(at).unwrap().flatten() {
            touch_all(&entry.path(), then);
        }
    }
    std::fs::File::open(at).unwrap().set_modified(then).unwrap();
}

#[test]
fn a_pass_under_the_floor_drops_the_build_caches_and_one_over_it_drops_nothing() {
    let root = a_repository_with_a_build();
    let roots = [(root.path().to_path_buf(), "main".to_string())];
    let caches = root.path().join("target/debug/incremental");
    let mut said = Vec::new();

    reclaim_now(&roots, 100 * GIB, &|_| Some(150 * GIB), &mut |line| said.push(line.to_string()));
    assert!(caches.exists() && said.is_empty(), "free is over the floor: {said:?}");

    let reads = Cell::new(0u64);
    reclaim_now(&roots, 100 * GIB, &|_| {
        reads.set(reads.get() + 1);
        // The check, the pass's first reading, then one after its act.
        Some(if reads.get() <= 2 { 40 * GIB } else { 130 * GIB })
    }, &mut |line| said.push(line.to_string()));

    assert!(!caches.exists(), "the caches went");
    assert!(root.path().join("target/debug/deps").exists(), "and the build did not");
    assert_eq!(said.len(), 1, "one line for the pass: {said:?}");
}

#[test]
fn a_floor_of_zero_is_off() {
    let root = a_repository_with_a_build();

    reclaim_now(&[(root.path().to_path_buf(), "main".to_string())], 0, &|_| Some(1), &mut |_| {});

    assert!(root.path().join("target/debug/incremental").exists());
}

#[test]
fn the_repository_here_is_looked_at_once_beside_those_a_fleet_serves() {
    let root = a_repository_with_a_build();
    root.write("nested/deeper/keep", "");
    let other = PathBuf::from("/somewhere/else");

    let roots = reach(&root.path().join("nested/deeper"), &[root.path().to_path_buf(), other.clone()]);

    assert_eq!(roots, vec![(root.path().to_path_buf(), "main".to_string()), (other, "main".to_string())]);
}

#[test]
fn disk_takes_reclaim_and_its_help_and_says_so_for_anything_else() {
    assert_eq!(asked("disk reclaim"), Ok(Verb::DiskReclaim { help: false }));
    assert_eq!(asked("disk reclaim --help"), Ok(Verb::DiskReclaim { help: true }));
    let said = |line: &str| asked(line).expect_err("refused").to_string();
    assert!(said("disk").contains("it takes `disk reclaim`"));
    assert!(said("disk drop").contains("`armada disk drop`"));
    assert!(crate::cli::Usage.to_string().contains("armada disk reclaim"));
}
