//! One look of the build-output sweep: free space decides whether anything is dropped.

use std::cell::Cell;
use std::path::PathBuf;
use std::time::Duration;

use adapters::leasing::Trim;

use crate::sweeping::{look, Reach};

const GIB: u64 = 1024 * 1024 * 1024;

fn a_main_checkout_with_a_build() -> PathBuf {
    let root = std::env::temp_dir().join(format!("armada-sweeping-{}", std::process::id()));
    let debug = root.join("crates/bridge-native/target/debug");
    std::fs::create_dir_all(debug.join("deps")).unwrap();
    std::fs::create_dir_all(debug.join("incremental/native-1")).unwrap();
    std::fs::write(debug.join("../CACHEDIR.TAG"), "Signature: 8a477f597d28d172789f06886806bc55\n").unwrap();
    let long_ago = std::time::SystemTime::now() - Duration::from_secs(10 * 3600);
    for dir in [debug.join("deps"), debug.join("incremental/native-1"), debug.join("incremental")] {
        std::fs::File::open(dir).unwrap().set_modified(long_ago).unwrap();
    }
    root
}

fn trim() -> Trim {
    Trim::SHIPPED
}

#[test]
fn a_look_under_the_floor_drops_a_nested_builds_incremental_caches_and_one_over_it_drops_nothing() {
    let root = a_main_checkout_with_a_build();
    let incremental = root.join("crates/bridge-native/target/debug/incremental");
    let reach = Reach { roots: vec![(root.clone(), "main".into())], floor: 100 * GIB };

    let over = look(&reach, false, trim(), &|_| Some(150 * GIB));
    assert!(over.lines.is_empty());
    assert!(incremental.exists());

    let reads = Cell::new(0u64);
    let under = look(&reach, false, trim(), &|_| {
        reads.set(reads.get() + 1);
        // The look's own reading, the pass's first, then one after its act.
        Some(if reads.get() <= 2 { 40 * GIB } else { 130 * GIB })
    });
    assert!(!incremental.exists(), "the main checkout's own caches go");
    assert_eq!(under.lines.len(), 1, "{:?}", under.lines);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_floor_of_zero_never_reclaims() {
    let root = a_main_checkout_with_a_build();
    let reach = Reach { roots: vec![(root.clone(), "main".into())], floor: 0 };

    look(&reach, false, trim(), &|_| Some(1));

    assert!(root.join("crates/bridge-native/target/debug/incremental").exists());
    std::fs::remove_dir_all(&root).ok();
}
