//! Running only what a name or a change points at: the members holding a test
//! of a name, and the test modules a change touched. Files are written to a
//! temporary workspace, since the facts read are in source.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use core_model::Narrowing;

use crate::holding::holders;
use crate::named::{one_test_in, tests_in, touched_run, touched_run_with, OneTestRefused, TouchedRun};
use crate::narrow::one_test;
use crate::touched::touched;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A workspace for one test, removed on drop.
struct Workspace(PathBuf);

impl Workspace {
    fn with(files: &[(&str, &str)]) -> Workspace {
        let root = std::env::temp_dir().join(format!(
            "armada-touched-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for (path, text) in files {
            let at = root.join(path);
            std::fs::create_dir_all(at.parent().expect("a parent")).expect("the parent");
            std::fs::write(at, text).expect("the file");
        }
        Workspace(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn paths(of: &[&str]) -> Vec<String> {
    of.iter().map(|path| path.to_string()).collect()
}

/// This repository's `test` layout: packages under `crates`, `-p` each, and the
/// whole run's exclusion restated.
fn by_package() -> Narrowing {
    Narrowing::declared(
        "cargo nextest run -p xtask".to_string(),
        "-p {}".to_string(),
        None,
        Some("crates".to_string()),
        vec!["acceptance".to_string()],
    )
}

const RUN: &str = "cargo nextest run -E test(/(^|::){}(::|$)/)";

/// A workspace like the real one: `fleet` has a `tests` module declared in
/// `lib.rs` and a source file with its own inline tests, `ipc` has neither.
fn a_workspace() -> Workspace {
    Workspace::with(&[
        (
            "crates/fleet/src/lib.rs",
            "pub mod narrow;\n#[cfg(test)]\nmod tests;\n",
        ),
        (
            "crates/fleet/src/narrow.rs",
            "pub fn narrowed() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn it() {}\n}\n",
        ),
        (
            "crates/fleet/src/tests.rs",
            "mod session_stop;\nmod deep;\n",
        ),
        (
            "crates/fleet/src/tests/session_stop.rs",
            "#[test]\nfn stops() {}\n",
        ),
        ("crates/fleet/src/tests/deep/mod.rs", "mod down;\n"),
        (
            "crates/fleet/src/tests/deep/down.rs",
            "#[test]\nfn d() {}\n",
        ),
        ("crates/ipc/src/lib.rs", "pub fn plain() {}\n"),
        (
            "crates/ipc/src/door.rs",
            "pub fn every_operation_the_door_offers_is_tagged() {}\n",
        ),
        (
            "crates/acceptance/tests/board.rs",
            "#[test]\nfn the_board() {}\n",
        ),
        (
            "crates/config/tests/shipped.rs",
            "// a note\n#[test]\nfn loads() {}\n\n#[tokio::test(flavor = \"x\")]\n#[ignore]\nasync fn waits() {}\n\nfn helper() {}\n",
        ),
        (
            "crates/fleet/target/debug/build/x.rs",
            "fn a_test_in_a_build_directory() {}\n",
        ),
    ])
}

// ----------------------------------------------------------- one test by name

#[test]
fn a_name_is_held_by_the_members_that_declare_it() {
    let workspace = a_workspace();
    let found = |test: &str| holders(workspace.path(), "crates", test);
    assert_eq!(found("stops"), ["fleet"]);
    assert_eq!(found("every_operation_the_door_offers_is_tagged"), ["ipc"]);
    // A module counts as well as a function, which `one_test` also runs.
    assert_eq!(found("session_stop"), ["fleet"]);
    // Through a path, and through nextest's `binary_id name` pair.
    assert_eq!(found("tests::session_stop::stops"), ["fleet"]);
    assert_eq!(found("fleet tests::session_stop::stops"), ["fleet"]);
}

#[test]
fn a_name_is_a_whole_word_and_no_build_directory_is_source() {
    let workspace = a_workspace();
    assert!(holders(workspace.path(), "crates", "stop").is_empty());
    assert!(holders(workspace.path(), "crates", "stops_").is_empty());
    assert!(holders(workspace.path(), "crates", "a_test_in_a_build_directory").is_empty());
    assert!(holders(workspace.path(), "crates", "not an identifier!").is_empty());
    assert!(holders(workspace.path(), "no-such-directory", "stops").is_empty());
}

#[test]
fn one_test_names_the_members_it_compiles() {
    let workspace = a_workspace();
    assert_eq!(
        one_test_in(workspace.path(), RUN, Some(&by_package()), "stops"),
        Ok("cargo nextest run -E test(/(^|::)stops(::|$)/) -p fleet".to_string())
    );
}

/// Several names are one command, so one build: one alternation, and every
/// member holding any of them.
#[test]
fn several_names_are_one_run_over_every_member_holding_one() {
    let workspace = a_workspace();
    assert_eq!(
        tests_in(
            workspace.path(),
            RUN,
            Some(&by_package()),
            &["stops", "every_operation_the_door_offers_is_tagged", "session_stop"],
        ),
        Ok("cargo nextest run -E \
            test(/(^|::)(?:stops|every_operation_the_door_offers_is_tagged|session_stop)(::|$)/) \
            -p fleet -p ipc"
            .to_string())
    );
}

/// One name nothing holds refuses the run and is the one named.
#[test]
fn one_unheld_name_among_several_refuses_the_run() {
    let workspace = a_workspace();
    assert_eq!(
        tests_in(workspace.path(), RUN, Some(&by_package()), &["stops", "no_such_test"]),
        Err(OneTestRefused::NoMember {
            test: "no_such_test".to_string(),
            under: "crates".to_string(),
            excepted: Vec::new(),
        })
    );
    assert_eq!(
        tests_in(workspace.path(), RUN, Some(&by_package()), &[]),
        Err(OneTestRefused::Blank)
    );
}

/// What the whole run excludes is not named by one test either, and is said.
#[test]
fn a_test_only_an_excluded_member_declares_is_refused_with_why() {
    let workspace = a_workspace();
    assert_eq!(
        one_test_in(workspace.path(), RUN, Some(&by_package()), "the_board"),
        Err(OneTestRefused::NoMember {
            test: "the_board".to_string(),
            under: "crates".to_string(),
            excepted: paths(&["acceptance"]),
        })
    );
}

/// Nothing is run rather than everything: no member declares it.
#[test]
fn a_name_nothing_declares_is_refused_and_never_widened() {
    let workspace = a_workspace();
    assert_eq!(
        one_test_in(workspace.path(), RUN, Some(&by_package()), "no_such_test"),
        Err(OneTestRefused::NoMember {
            test: "no_such_test".to_string(),
            under: "crates".to_string(),
            excepted: Vec::new(),
        })
    );
}

#[test]
fn a_check_with_no_layout_runs_its_one_test_as_written() {
    let workspace = a_workspace();
    let flat = Narrowing::declared("run".to_string(), "{}".to_string(), None, None, Vec::new());
    for narrowing in [None, Some(&flat)] {
        assert_eq!(
            one_test_in(workspace.path(), RUN, narrowing, "anything"),
            Ok(one_test(RUN, "anything").expect("a name"))
        );
    }
    assert_eq!(
        one_test_in(workspace.path(), RUN, Some(&by_package()), "  "),
        Err(OneTestRefused::Blank)
    );
}

// --------------------------------------------------------- a change's tests

fn touching(workspace: &Workspace, changed: &[&str]) -> (Vec<String>, Vec<String>) {
    let found = touched(workspace.path(), &by_package(), &paths(changed));
    (found.members, found.names)
}

#[test]
fn a_test_module_file_runs_as_its_module_path() {
    let workspace = a_workspace();
    assert_eq!(
        touching(&workspace, &["crates/fleet/src/tests/session_stop.rs"]),
        (paths(&["fleet"]), paths(&["tests::session_stop"]))
    );
}

#[test]
fn a_nested_test_module_keeps_every_segment_and_mod_rs_is_its_directory() {
    let workspace = a_workspace();
    assert_eq!(
        touching(
            &workspace,
            &[
                "crates/fleet/src/tests/deep/down.rs",
                "crates/fleet/src/tests/deep/mod.rs"
            ]
        ),
        (
            paths(&["fleet"]),
            paths(&["tests::deep", "tests::deep::down"])
        )
    );
}

#[test]
fn a_source_file_with_an_inline_test_module_runs_that_module() {
    let workspace = a_workspace();
    assert_eq!(
        touching(&workspace, &["crates/fleet/src/narrow.rs"]),
        (paths(&["fleet"]), paths(&["narrow::tests"]))
    );
}

/// Declaring `mod tests;` is not the tests, so adding a `mod` line to `lib.rs`
/// does not run a crate's whole suite.
#[test]
fn a_file_that_only_declares_its_tests_touches_none() {
    let workspace = a_workspace();
    assert_eq!(
        touching(
            &workspace,
            &["crates/fleet/src/lib.rs", "crates/ipc/src/door.rs"]
        ),
        (Vec::new(), Vec::new())
    );
}

#[test]
fn an_integration_test_runs_the_functions_it_marks() {
    let workspace = a_workspace();
    assert_eq!(
        touching(&workspace, &["crates/config/tests/shipped.rs"]),
        (paths(&["config"]), paths(&["loads", "waits"]))
    );
}

#[test]
fn a_member_the_check_excepts_and_a_deleted_file_touch_nothing() {
    let workspace = a_workspace();
    assert_eq!(
        touching(
            &workspace,
            &[
                "crates/acceptance/tests/board.rs",
                "crates/fleet/src/tests/gone.rs",
                "xtask/src/rules.rs",
                "docs/a.md",
                "crates/fleet/src/tests/notes.txt",
            ]
        ),
        (Vec::new(), Vec::new())
    );
}

#[test]
fn the_command_names_every_touched_test_in_every_touched_member() {
    let workspace = a_workspace();
    let changed = paths(&[
        "crates/fleet/src/tests/session_stop.rs",
        "crates/fleet/src/narrow.rs",
        "crates/config/tests/shipped.rs",
    ]);
    let TouchedRun::Tests { command, said, .. } =
        touched_run(workspace.path(), Some(RUN), Some(&by_package()), &changed)
    else {
        panic!("a change touching tests runs them");
    };
    assert_eq!(
        command,
        "cargo nextest run -E test(/(^|::)(?:loads|narrow::tests|tests::session_stop|waits)(::|$)/) -p config -p fleet"
    );
    assert!(said.ends_with("in -p config -p fleet"), "{said}");
}

/// The escaping is `one_test`'s, so a lone name reads exactly as `one_test`
/// spells it.
#[test]
fn one_touched_test_reads_as_one_test_does() {
    let workspace = a_workspace();
    let changed = paths(&["crates/fleet/src/tests/session_stop.rs"]);
    assert_eq!(
        touched_run(workspace.path(), Some(RUN), Some(&by_package()), &changed),
        TouchedRun::Tests {
            command: format!(
                "{} -p fleet",
                one_test(RUN, "tests::session_stop").expect("a name")
            ),
            said: "tests::session_stop in -p fleet".to_string(),
            mapped: Vec::new(),
        }
    );
}

#[test]
fn a_change_touching_no_test_says_so_and_a_check_without_one_test_has_no_reading() {
    let workspace = a_workspace();
    let plain = paths(&["crates/ipc/src/door.rs"]);
    assert_eq!(
        touched_run(workspace.path(), Some(RUN), Some(&by_package()), &plain),
        TouchedRun::None
    );
    assert_eq!(
        touched_run(workspace.path(), None, Some(&by_package()), &plain),
        TouchedRun::NotApplicable
    );
    assert_eq!(
        touched_run(workspace.path(), Some(RUN), None, &plain),
        TouchedRun::NotApplicable
    );
}

// ------------------------------------------------------------ the test map

fn mapped(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(member, name)| (member.to_string(), name.to_string()))
        .collect()
}

/// **The map's tests join the touched ones in one command**: one build, the
/// names deduplicated, and the map's member compiled beside the touched one.
#[test]
fn the_maps_tests_are_added_to_the_touched_ones_in_one_command() {
    let workspace = a_workspace();
    let changed = paths(&[
        "crates/fleet/src/tests/session_stop.rs",
        "crates/ipc/src/door.rs",
    ]);
    let from_map = mapped(&[
        ("ipc", "tests::door::answers"),
        ("fleet", "tests::session_stop"),
    ]);
    assert_eq!(
        touched_run_with(workspace.path(), Some(RUN), Some(&by_package()), &changed, &from_map),
        TouchedRun::Tests {
            command: "cargo nextest run -E test(/(^|::)(?:tests::door::answers|tests::session_stop)(::|$)/) -p fleet -p ipc"
                .to_string(),
            said: "tests::door::answers tests::session_stop in -p fleet -p ipc".to_string(),
            mapped: paths(&["tests::door::answers"]),
        }
    );
}

/// A file no changed test touches still runs what the map says reaches it,
/// where with no map it ran nothing.
#[test]
fn the_map_alone_runs_what_reaches_a_plain_source_file() {
    let workspace = a_workspace();
    let plain = paths(&["crates/ipc/src/door.rs"]);
    let from_map = mapped(&[("ipc", "tests::door::answers")]);
    let TouchedRun::Tests { command, mapped, .. } =
        touched_run_with(workspace.path(), Some(RUN), Some(&by_package()), &plain, &from_map)
    else {
        panic!("the map's test runs");
    };
    assert!(command.ends_with(" -p ipc"), "{command}");
    assert_eq!(mapped, paths(&["tests::door::answers"]));
}

/// A test in a member `except` bars is dropped, as a touched one is, and a
/// map naming only such tests runs nothing.
#[test]
fn a_maps_test_in_a_barred_member_is_dropped() {
    let workspace = a_workspace();
    let plain = paths(&["crates/ipc/src/door.rs"]);
    let barred = mapped(&[("acceptance", "board_moves"), ("", "nowhere")]);
    assert_eq!(
        touched_run_with(workspace.path(), Some(RUN), Some(&by_package()), &plain, &barred),
        TouchedRun::None
    );
}
