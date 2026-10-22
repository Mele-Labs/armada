//! Where a failed Check's log says it failed, from snippets in the tools' own words. The slot is a
//! folder holding the files the snippets name, since a path not on disk in the slot names nothing.

use ipc::CheckBlame;

use super::scratch::Scratch;
use crate::check_blame::blame_of;

fn slot(files: &[&str]) -> Scratch {
    let slot = Scratch::new();
    for one in files {
        slot.file(one, "");
    }
    slot
}

fn at(path: &str, line: u32) -> Option<CheckBlame> {
    Some(CheckBlame { path: path.into(), line: Some(line) })
}

#[test]
fn a_rustc_error_blames_its_arrow_and_a_warning_before_it_does_not() {
    let slot = slot(&["crates/store/src/lib.rs", "crates/fleet/src/daemon.rs"]);
    let log = "\
   Compiling store v0.0.0 (/x/crates/store)
warning: unused import: `Path`
  --> crates/store/src/lib.rs:3:5
   |
3  | use std::path::Path;
   |     ^^^^^^^^^^^^^^^
error[E0308]: mismatched types
  --> crates/fleet/src/daemon.rs:212:17
   |
212|         let n: u32 = \"x\";
   |                --- ^^^ expected `u32`, found `&str`
error: could not compile `fleet` (lib) due to 1 previous error
";
    assert_eq!(blame_of(log, slot.path()), at("crates/fleet/src/daemon.rs", 212));
}

#[test]
fn a_panic_outside_tests_wins_over_the_tests_own() {
    let slot = slot(&["crates/store/src/tests/reads.rs", "crates/store/src/reads.rs"]);
    let log = "\
running 2 tests
thread 'tests::reads::an_empty_store_reads_nothing' panicked at crates/store/src/tests/reads.rs:40:5:
assertion `left == right` failed
thread 'tests::reads::the_last_row_is_read' panicked at crates/store/src/reads.rs:88:31:
called `Option::unwrap()` on a `None` value
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
";
    assert_eq!(blame_of(log, slot.path()), at("crates/store/src/reads.rs", 88));
}

#[test]
fn a_nextest_failure_blames_the_panic_in_its_output_and_not_the_summary() {
    let slot = slot(&["crates/fleet/src/tests/session_checking.rs"]);
    let log = "\
        PASS [   0.012s] fleet tests::session_checking::each_stop_route_is_answered
        FAIL [   0.031s] fleet tests::session_checking::a_failed_run_carries_its_blame
──── STDERR:             fleet tests::session_checking::a_failed_run_carries_its_blame
thread 'tests::session_checking::a_failed_run_carries_its_blame' panicked at crates/fleet/src/tests/session_checking.rs:120:5:
assertion failed: blame.is_some()
────────────
     Summary [   2.104s] 214 tests run: 213 passed, 1 failed, 3 skipped
        FAIL [   0.031s] fleet tests::session_checking::a_failed_run_carries_its_blame
error: test run failed
";
    assert_eq!(blame_of(log, slot.path()), at("crates/fleet/src/tests/session_checking.rs", 120));
}

#[test]
fn a_vitest_failure_blames_the_frame_in_the_code_under_test_and_reads_paths_from_its_package() {
    let desk = "apps/desktop/src/renderer/src/cockpit/deck";
    let slot = slot(&[&format!("{desk}/layout.ts"), &format!("{desk}/layout.test.ts"), "apps/desktop/package.json"]);
    let log = "\
apps/desktop test:  \u{1b}[31mFAIL\u{1b}[39m  src/renderer/src/cockpit/deck/layout.test.ts > a bar stands on its plot
apps/desktop test: TypeError: Cannot read properties of undefined (reading 'rect')
apps/desktop test:  ❯ barOf src/renderer/src/cockpit/deck/layout.ts:142:9
apps/desktop test:  ❯ src/renderer/src/cockpit/deck/layout.test.ts:42:7
apps/desktop test:  Test Files  1 failed | 41 passed (42)
";
    assert_eq!(blame_of(log, slot.path()), at(&format!("{desk}/layout.ts"), 142));
}

#[test]
fn a_vitest_failure_with_only_the_test_named_blames_the_test_at_its_frames_line() {
    let slot = slot(&["packages/screens/src/sessions-wire.test.ts"]);
    let log = "\
 FAIL  src/sessions-wire.test.ts > a check row keeps its blame
AssertionError: expected undefined to deeply equal { path: 'a.ts' }
 ❯ src/sessions-wire.test.ts:291:20
";
    assert_eq!(blame_of(log, slot.path()), at("packages/screens/src/sessions-wire.test.ts", 291));
}

#[test]
fn a_tsc_error_blames_its_file_in_either_spelling() {
    let slot = slot(&["apps/desktop/src/main/onboarding.ts", "packages/protocol/src/wire.ts"]);
    let plain = "apps/desktop typecheck: src/main/onboarding.ts(12,5): error TS2322: Type 'string' is not assignable to type 'number'.\n";
    assert_eq!(blame_of(plain, slot.path()), at("apps/desktop/src/main/onboarding.ts", 12));
    let pretty = "packages/protocol/src/wire.ts:30:3 - error TS2741: Property 'blame' is missing.\n";
    assert_eq!(blame_of(pretty, slot.path()), at("packages/protocol/src/wire.ts", 30));
}

#[test]
fn an_absolute_path_in_the_slot_is_made_relative_and_one_outside_it_names_nothing() {
    let slot = slot(&["crates/store/src/reads.rs"]);
    let root = slot.path().display();
    let inside = format!("thread 'main' panicked at {root}/crates/store/src/reads.rs:7:1:\nboom\n");
    assert_eq!(blame_of(&inside, slot.path()), at("crates/store/src/reads.rs", 7));
    let outside = "\
thread 'main' panicked at /Users/someone/.cargo/registry/src/index/serde-1.0.0/src/de.rs:7:1:
error[E0425]: cannot find value
  --> ../elsewhere/src/lib.rs:3:1
";
    assert_eq!(blame_of(outside, slot.path()), None);
}

#[test]
fn a_log_that_names_no_location_blames_nothing() {
    let slot = slot(&["crates/store/src/reads.rs"]);
    let log = "error: could not find `Cargo.toml` in `/x` or any parent directory\nKilled: 9\n";
    assert_eq!(blame_of(log, slot.path()), None);
    // A file the slot does not hold is not guessed at.
    assert_eq!(blame_of("thread 'main' panicked at crates/gone/src/lib.rs:1:1:\n", slot.path()), None);
}
