//! The measured collector: what it reads out of an lcov export, and one run on
//! a fixture workspace that needs nextest and the LLVM tools.

use std::path::{Path, PathBuf};

use adapter_traits::{TestCollector, TestsToRecord};
use checks_runner::CheckWidth;
use core_model::Narrowing;

use super::scratch::Scratch;
use crate::test_map::{files_reached, in_repository, llvm_major, measured_in, member_of, tool_folders, FOLDED, RUNNER};
use crate::NextestCoverage;

#[test]
fn a_measured_run_runs_as_many_tests_at_once_as_its_width() {
    let arguments = NextestCoverage::for_check(None, CheckWidth::of(3)).arguments(&TestsToRecord::All);

    let threads = arguments.iter().position(|word| word == "--test-threads").expect("a width is passed");
    assert_eq!(arguments[threads + 1], "3");
}

#[test]
fn an_lcov_export_names_only_the_files_it_ran_a_line_of() {
    let lcov = "SF:/repo/src/ran.rs\nFN:1,f\nDA:1,0\nDA:2,3\nend_of_record\n\
                SF:/repo/src/never.rs\nDA:1,0\nDA:2,0\nend_of_record\n\
                SF:/repo/src/once.rs\nDA:7,1\nDA:8,1\nend_of_record\n";

    assert_eq!(files_reached(lcov), ["/repo/src/ran.rs", "/repo/src/once.rs"]);
}

#[test]
fn a_reached_file_is_named_from_the_repository_root() {
    let root = Path::new("/work/repo");
    let target = root.join("target/test-map");

    assert_eq!(
        in_repository("/work/repo/crates/sums/src/add.rs", root, &target).as_deref(),
        Some("crates/sums/src/add.rs")
    );
}

#[test]
fn a_file_outside_the_repository_or_built_into_target_is_dropped() {
    let root = Path::new("/work/repo");
    let target = root.join("target/test-map");

    for outside in [
        "/Users/user/.cargo/registry/src/index/serde-1.0.0/src/lib.rs",
        "/rustc/48a229ceaefd/library/core/src/ops/function.rs",
        "/work/repository/src/lib.rs",
        "/work/repo/target/test-map/debug/build/sums-1/out/generated.rs",
    ] {
        assert_eq!(in_repository(outside, root, &target), None, "{outside}");
    }
}

#[test]
fn a_test_binary_is_in_the_member_its_package_sits_under() {
    let root = Path::new("/work/repo");

    assert_eq!(member_of(&root.join("crates/fleet"), root, Some("crates")), "fleet");
    assert_eq!(member_of(&root.join("crates/fleet"), root, Some("crates/")), "fleet");
    assert_eq!(member_of(root, root, Some("crates")), "");
    assert_eq!(member_of(&root.join("crates/fleet"), root, None), "");
}

#[test]
fn the_llvm_tools_are_looked_for_beside_the_compiler_then_under_its_prefix() {
    let rustup = tool_folders(Path::new("/home/user/.rustup/toolchains/stable"), "x86_64-unknown-linux-gnu", 22);
    assert_eq!(
        rustup,
        [PathBuf::from("/home/user/.rustup/toolchains/stable/lib/rustlib/x86_64-unknown-linux-gnu/bin")]
    );

    let prefixed = tool_folders(Path::new("/opt/homebrew/Cellar/rust/1.98.1"), "aarch64-apple-darwin", 22);
    assert_eq!(
        prefixed,
        [
            PathBuf::from("/opt/homebrew/Cellar/rust/1.98.1/lib/rustlib/aarch64-apple-darwin/bin"),
            PathBuf::from("/opt/homebrew/opt/llvm@22/bin"),
            PathBuf::from("/opt/homebrew/opt/llvm/bin"),
        ]
    );
}

#[test]
fn the_llvm_major_version_is_read_as_the_compiler_and_the_tools_print_it() {
    assert_eq!(llvm_major("binary: rustc\nLLVM version: 22.1.8\n"), Some(22));
    assert_eq!(llvm_major("Homebrew LLVM version 22.1.8\n  Optimized build.\n"), Some(22));
    assert_eq!(llvm_major("LLVM (http://llvm.org/):\n  LLVM version 21.1.2-rust-1.93.0-stable\n"), Some(21));
    assert_eq!(llvm_major("rustc 1.98.1\n"), None);
}

/// A workspace of one member, `sums`, with a function in each of two files
/// and three tests, each reaching a different set of them.
fn executable(scratch: &Scratch, relative: &str, text: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = scratch.file(relative, text);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("made executable");
    path
}

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .expect("the folder is read")
        .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// Kept raw until the run ended, every test's whole-binary profile filled the
// hosted runner's disk.
#[test]
fn a_finished_test_leaves_one_folded_profile_and_its_own_exit_status() {
    let scratch = Scratch::new();
    let laps = scratch.folder("laps");
    let runner = executable(&scratch, "runner", RUNNER);
    let profdata = executable(
        &scratch,
        "llvm-profdata",
        "#!/bin/sh\nwhile [ \"$1\" != -o ]; do shift; done\necho folded > \"$2\"\n",
    );
    let test = executable(
        &scratch,
        "test-binary",
        "#!/bin/sh\necho raw > \"$(dirname \"$LLVM_PROFILE_FILE\")/1-2.profraw\"\nexit 3\n",
    );

    let status = std::process::Command::new(&runner)
        .arg(&test)
        .env("NEXTEST_TEST_NAME", "tests::adds")
        .env("NEXTEST_BINARY_ID", "sums::reach")
        .env("ARMADA_TEST_MAP_LAPS", &laps)
        .env("ARMADA_TEST_MAP_PROFDATA", &profdata)
        .status()
        .expect("the runner starts");

    assert_eq!(status.code(), Some(3));
    let lap = laps.join(&names_in(&laps)[0]);
    assert_eq!(names_in(&lap), [FOLDED, "test"]);
}

#[test]
fn a_lap_is_read_from_its_folded_profile_and_any_raw_one_written_after_the_fold() {
    let scratch = Scratch::new();
    scratch.file("laps/lap.a/test", "sums::reach\ntests::adds\n/bin/reach\n/work\n");
    scratch.file(&format!("laps/lap.a/{FOLDED}"), "folded");
    scratch.file("laps/lap.a/9-2.profraw", "late");

    let measured = measured_in(&scratch.path().join("laps")).expect("the laps are read");

    let run = &measured[&("sums::reach".to_string(), "tests::adds".to_string())];
    let mut profiles: Vec<String> = run
        .profiles
        .iter()
        .map(|profile| profile.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    profiles.sort();
    assert_eq!(profiles, ["9-2.profraw", FOLDED]);
}

fn fixture() -> Scratch {
    let scratch = Scratch::new();
    scratch.file("Cargo.toml", "[workspace]\nmembers = [\"crates/sums\"]\nresolver = \"2\"\n");
    scratch.file(
        "crates/sums/Cargo.toml",
        "[package]\nname = \"sums\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n",
    );
    scratch.file("crates/sums/src/lib.rs", "pub mod add;\npub mod double;\n");
    scratch.file("crates/sums/src/add.rs", "pub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n");
    scratch.file("crates/sums/src/double.rs", "pub fn double(a: u32) -> u32 {\n    a * 2\n}\n");
    scratch.file(
        "crates/sums/tests/reach.rs",
        "use sums::add::add;\nuse sums::double::double;\n\n\
         #[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n\n\
         #[test]\nfn doubles() {\n    assert_eq!(double(2), 4);\n}\n\n\
         #[test]\nfn doubles_a_sum() {\n    assert_eq!(double(add(1, 2)), 6);\n}\n",
    );
    scratch
}

#[test]
#[ignore = "builds a fixture instrumented: needs cargo-nextest, and llvm-profdata and llvm-cov of the compiler's LLVM"]
fn each_measured_lap_names_exactly_the_files_its_test_reaches() {
    let fixture = fixture();
    let narrowing = Narrowing::declared(
        "cargo nextest run".to_string(),
        "-p {}".to_string(),
        None,
        Some("crates".to_string()),
        Vec::new(),
    );
    let collector = NextestCoverage::for_check(Some(&narrowing), CheckWidth::of(2));

    let laps = collector
        .record("test", &fixture.path().to_string_lossy(), &TestsToRecord::All)
        .expect("the fixture is measured");

    let read: Vec<(String, String, Vec<String>)> =
        laps.into_iter().map(|lap| (lap.member, lap.test, lap.files)).collect();
    let lap = |test: &str, files: &[&str]| {
        ("sums".to_string(), test.to_string(), files.iter().map(|file| file.to_string()).collect::<Vec<_>>())
    };
    assert_eq!(
        read,
        [
            lap("adds", &["crates/sums/src/add.rs", "crates/sums/tests/reach.rs"]),
            lap("doubles", &["crates/sums/src/double.rs", "crates/sums/tests/reach.rs"]),
            lap(
                "doubles_a_sum",
                &["crates/sums/src/add.rs", "crates/sums/src/double.rs", "crates/sums/tests/reach.rs"]
            ),
        ]
    );
}
