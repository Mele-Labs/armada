//! `armada check <name> --changed`, over a Manifest written for the test.

use std::time::Duration;

use checks_runner::Priority;

use crate::declared::{execute, Asked, NotDeclared, Ran, Reached, Registry};
use crate::reaching::asked_changed;
use crate::tests::TempDir;

const BUDGET: Duration = Duration::from_secs(30);

/// A Manifest whose `test` narrows by package under `crates`, as this
/// repository's does, with a whole run that fails so a run that should have
/// been narrowed cannot pass by accident.
fn a_narrowing_repository() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           test:\n    run: /usr/bin/false\n    \
             narrow:\n      run: /bin/echo narrowed\n      each: \"-p {}\"\n      \
               under: crates\n      except:\n        - acceptance\n    \
             when:\n      - \"crates/**\"\n      - \"Cargo.lock\"\n",
    );
    dir
}

async fn changed(dir: &TempDir, paths: &[&str]) -> Ran {
    let paths: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
    execute(
        dir.path(),
        Registry::Checks,
        "test",
        Asked::Changed(&paths),
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect("`test` is declared")
}

/// **The merge line's narrowing**: every changed path the Check covers names a
/// package, so the narrowed command runs, and a path it does not cover is no
/// reason to run it whole.
#[tokio::test]
async fn a_change_the_narrowing_can_name_runs_narrowed() {
    let dir = a_narrowing_repository();
    let ran = changed(
        &dir,
        &[
            "crates/fleet/src/lib.rs",
            "crates/api/Cargo.toml",
            "crates/acceptance/tests/a.rs",
            "docs/a.md",
        ],
    )
    .await;
    assert_eq!(ran.command, "/bin/echo narrowed -p api -p fleet");
    assert_eq!(
        ran.narrowed,
        Some(Reached::To("-p api -p fleet".to_string()))
    );
    assert_eq!(ran.status(), 0, "{:?}", ran.attempt);
}

#[tokio::test]
async fn a_covered_path_it_cannot_name_runs_it_whole() {
    let dir = a_narrowing_repository();
    let ran = changed(&dir, &["crates/fleet/src/lib.rs", "Cargo.lock"]).await;
    assert_eq!(ran.command, "/usr/bin/false");
    assert_eq!(ran.narrowed, Some(Reached::Whole));
    assert_eq!(ran.status(), 1);
}

/// What the whole run excludes, alone, is nothing to run — and is said, so a
/// pass is never read as a run.
#[tokio::test]
async fn a_change_reaching_only_the_exclusion_runs_nothing_and_passes() {
    let dir = a_narrowing_repository();
    let ran = changed(&dir, &["crates/acceptance/tests/a.rs"]).await;
    assert_eq!(ran.narrowed, Some(Reached::Nothing));
    assert_eq!(ran.status(), 0);
}

// ------------------------------------------------------ an agent's reading

/// The same layout with a `one_test`, which is what lets a Check run by name,
/// and a workspace on disk for the source facts to be read from.
fn a_workspace_with_one_test() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           test:\n    run: /usr/bin/false\n    \
             narrow:\n      run: /bin/echo narrowed\n      each: \"-p {}\"\n      \
               under: crates\n      except:\n        - acceptance\n    \
             one_test:\n      run: /bin/echo ran {}\n    \
             when:\n      - \"crates/**\"\n",
    );
    dir.write(
        "crates/fleet/src/lib.rs",
        "pub mod other;\n#[cfg(test)]\nmod tests;\n",
    );
    dir.write("crates/fleet/src/other.rs", "pub fn other() {}\n");
    dir.write("crates/fleet/src/tests.rs", "mod session_stop;\n");
    dir.write(
        "crates/fleet/src/tests/session_stop.rs",
        "#[test]\nfn stops() {}\n",
    );
    dir.write("crates/acceptance/tests/board.rs", "#[test]\nfn the_board() {}\n");
    dir
}

async fn asked(dir: &TempDir, asked: Asked<'_>) -> Result<Ran, NotDeclared> {
    execute(
        dir.path(),
        Registry::Checks,
        "test",
        asked,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
}

fn paths(of: &[&str]) -> Vec<String> {
    of.iter().map(|path| path.to_string()).collect()
}

/// **Only an agent on a shared machine gets the narrower reading.** CI says it
/// is alone on its machine and the merge line asks at normal priority, and both
/// keep every member the paths name. Nothing named is no change, and runs whole.
#[test]
fn only_an_agent_on_a_shared_machine_is_asked_for_the_touched_tests() {
    let one = paths(&["crates/fleet/src/tests/session_stop.rs"]);
    assert!(matches!(
        asked_changed(&one, Priority::Low, false),
        Asked::Touched(_)
    ));
    for (priority, sole_tenant) in [
        (Priority::Low, true),
        (Priority::Normal, false),
        (Priority::Normal, true),
    ] {
        assert!(
            matches!(asked_changed(&one, priority, sole_tenant), Asked::Changed(_)),
            "{priority:?} {sole_tenant}"
        );
    }
    assert!(matches!(
        asked_changed(&[], Priority::Low, false),
        Asked::Changed(_)
    ));
}

/// One new test file runs that module in its crate: no other member, no
/// `narrow.run`, no dependents.
#[tokio::test]
async fn a_changed_test_file_runs_its_module_in_its_crate_alone() {
    let dir = a_workspace_with_one_test();
    let changed = paths(&["crates/fleet/src/tests/session_stop.rs"]);
    let ran = asked(&dir, Asked::Touched(&changed)).await.expect("declared");
    assert_eq!(ran.command, "/bin/echo ran tests::session_stop -p fleet");
    assert_eq!(
        ran.narrowed,
        Some(Reached::Touched("tests::session_stop in -p fleet".to_string()))
    );
    assert_eq!(ran.status(), 0, "{:?}", ran.attempt);
}

/// A change touching no test says so and runs nothing: it does not fall back to
/// the Check whole, which here exits 1.
#[tokio::test]
async fn a_change_touching_no_test_runs_nothing_and_is_not_widened() {
    let dir = a_workspace_with_one_test();
    let changed = paths(&["crates/fleet/src/other.rs", "crates/fleet/src/lib.rs"]);
    let ran = asked(&dir, Asked::Touched(&changed)).await.expect("declared");
    assert_eq!(ran.narrowed, Some(Reached::NoTests));
    assert!(ran.command.is_empty(), "{}", ran.command);
    assert_eq!(ran.status(), 0);
}

/// **The merge line's reading does not move.** The same paths, asked the way
/// CI and the merge line ask, still name every member they touch.
#[tokio::test]
async fn the_same_paths_asked_the_merge_lines_way_still_run_the_whole_member() {
    let dir = a_workspace_with_one_test();
    let changed = paths(&["crates/fleet/src/tests/session_stop.rs"]);
    let ran = asked(&dir, Asked::Changed(&changed)).await.expect("declared");
    assert_eq!(ran.command, "/bin/echo narrowed -p fleet");
    assert_eq!(ran.narrowed, Some(Reached::To("-p fleet".to_string())));
}

/// A Check that cannot run a test by name has no touched reading and is
/// narrowed as before.
#[tokio::test]
async fn a_check_without_a_one_test_is_narrowed_as_the_merge_line_narrows_it() {
    let dir = a_narrowing_repository();
    let changed = paths(&["crates/fleet/src/tests/session_stop.rs"]);
    let ran = asked(&dir, Asked::Touched(&changed)).await.expect("declared");
    assert_eq!(ran.command, "/bin/echo narrowed -p fleet");
}

// -------------------------------------------------------- one test by name

/// One test compiles the crate that holds it and none other.
#[tokio::test]
async fn a_test_by_name_runs_in_the_crate_that_declares_it() {
    let dir = a_workspace_with_one_test();
    let ran = asked(&dir, Asked::Tests(&["stops"])).await.expect("declared");
    assert_eq!(ran.command, "/bin/echo ran stops -p fleet");
}

/// A name nothing declares is refused, saying so, rather than run against
/// every member; and one only an excepted member declares says whose it is.
#[tokio::test]
async fn a_test_nothing_declares_is_refused_and_not_run_everywhere() {
    let dir = a_workspace_with_one_test();
    let nowhere = asked(&dir, Asked::Tests(&["no_such_test"]))
        .await
        .expect_err("nothing declares it");
    assert!(
        nowhere.to_string().contains("nothing under `crates` declares"),
        "{nowhere}"
    );
    let left_out = asked(&dir, Asked::Tests(&["the_board"]))
        .await
        .expect_err("an excepted member declares it");
    let said = left_out.to_string();
    assert!(said.contains("acceptance"), "{said}");
}

/// What Fleet is told a run was narrowed to, which the Session's thread row draws: the packages
/// the change reached, in the order the run names them, and nothing where it ran whole or not at all.
#[tokio::test]
async fn a_changed_run_names_the_packages_it_reached_as_its_target() {
    let dir = a_narrowing_repository();
    let ran = changed(&dir, &["crates/fleet/src/lib.rs", "crates/ipc/src/lib.rs", "docs/a.md"]).await;
    assert_eq!(ran.target.as_deref(), Some("--changed fleet, ipc"));

    let whole = changed(&dir, &["crates/fleet/src/lib.rs", "Cargo.lock"]).await;
    assert_eq!(whole.target, None);
    let nothing = changed(&dir, &["crates/acceptance/tests/a.rs"]).await;
    assert_eq!(nothing.target, None);
}

/// A test filter is the target verbatim, and a plain run has none.
#[tokio::test]
async fn a_filtered_run_has_its_filter_as_its_target_and_a_plain_run_none() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           test:\n    run: /usr/bin/true\n    one_test:\n      run: /bin/echo only {}\n",
    );
    let run = |asked| {
        execute(dir.path(), Registry::Checks, "test", asked, BUDGET, None, Priority::Normal)
    };
    let filtered = run(Asked::Tests(&["store::reads_the_last_row"])).await.expect("declared");
    assert_eq!(filtered.target.as_deref(), Some("store::reads_the_last_row"));
    assert_eq!(run(Asked::Whole).await.expect("declared").target, None);
}

// ------------------------------------------------------------ the test map

/// One edge, as Fleet's map answers it.
fn edge(test: &str, file: &str, grain: core_model::EdgeGrain, stale: bool) -> ipc::ReachingTest {
    ipc::ReachingTest {
        test: test.to_string(),
        file: file.to_string(),
        source: core_model::EdgeSource::Measured.into(),
        grain: grain.into(),
        commit: "c0ffee".to_string(),
        stale,
    }
}

fn group(check: &str, member: &str, tests: Vec<ipc::ReachingTest>) -> ipc::ReachingCheck {
    ipc::ReachingCheck { check: check.to_string(), member: member.to_string(), tests }
}

/// An agent's `--changed`, with `answer` as the map Fleet holds. The map is
/// asked for this repository's Manifest id and the paths as the change named them.
async fn mapped(dir: &TempDir, changed: &[String], answer: Option<ipc::TestsReaching>) -> Ran {
    let asked_for = changed.to_vec();
    let read = move |id: &str, paths: &[String]| {
        assert_eq!(id, "a-test-project");
        assert_eq!(paths, asked_for.as_slice());
        answer.clone()
    };
    crate::declared::execute_told(
        dir.path(),
        Registry::Checks,
        "test",
        Asked::Touched(changed),
        BUDGET,
        None,
        Priority::Normal,
        None,
        Some(&read),
    )
    .await
    .expect("declared")
}

/// **The map's tests join the touched ones, in one run.** A source file no test
/// sits in runs what the map says reaches it, a second Check's edges are not
/// this one's, and a name the change touched is not said twice.
#[tokio::test]
async fn the_maps_tests_are_added_to_the_touched_ones_in_one_run() {
    let dir = a_workspace_with_one_test();
    let changed = paths(&["crates/fleet/src/tests/session_stop.rs", "crates/fleet/src/other.rs"]);
    let test = core_model::EdgeGrain::Test;
    let answer = ipc::TestsReaching {
        checks: vec![
            group("acceptance", "", vec![edge("the_board", "crates/fleet/src/other.rs", test, false)]),
            group(
                "test",
                "fleet",
                vec![
                    edge("tests::other_reach::reads", "crates/fleet/src/other.rs", test, false),
                    edge("tests::session_stop", "crates/fleet/src/other.rs", test, false),
                ],
            ),
        ],
        unreached: Vec::new(),
    };
    let ran = mapped(&dir, &changed, Some(answer)).await;
    assert_eq!(
        ran.narrowed,
        Some(Reached::Touched("tests::other_reach::reads tests::session_stop in -p fleet".to_string()))
    );
    assert!(ran.command.starts_with("/bin/echo ran ") && ran.command.ends_with(" -p fleet"), "{}", ran.command);
    assert_eq!(ran.mapped.added, paths(&["tests::other_reach::reads"]));
    assert_eq!(ran.mapped.before(), paths(&["the test map added tests::other_reach::reads"]));
    assert_eq!(ran.status(), 0, "{:?}", ran.attempt);
}

/// A test in a member `narrow.except` bars is dropped, as a touched one is, and
/// a change the map reaches only through one runs nothing.
#[tokio::test]
async fn a_maps_test_in_a_barred_member_is_dropped() {
    let dir = a_workspace_with_one_test();
    let changed = paths(&["crates/fleet/src/other.rs"]);
    let answer = ipc::TestsReaching {
        checks: vec![group(
            "test",
            "acceptance",
            vec![edge("the_board", "crates/fleet/src/other.rs", core_model::EdgeGrain::Test, false)],
        )],
        unreached: Vec::new(),
    };
    let ran = mapped(&dir, &changed, Some(answer)).await;
    assert_eq!(ran.narrowed, Some(Reached::NoTests));
    assert!(ran.command.is_empty(), "{}", ran.command);
    assert!(ran.mapped.added.is_empty());
}

/// **No Fleet, or a map with no edge for any asked path, is today's run**, and
/// says nothing more: not even the paths the empty map calls unreached.
#[tokio::test]
async fn no_fleet_or_an_empty_map_runs_as_without_a_map() {
    let dir = a_workspace_with_one_test();
    let changed = paths(&["crates/fleet/src/tests/session_stop.rs", "crates/fleet/src/other.rs"]);
    let today = asked(&dir, Asked::Touched(&changed)).await.expect("declared");
    let no_fleet = mapped(&dir, &changed, None).await;
    let empty = mapped(
        &dir,
        &changed,
        Some(ipc::TestsReaching { checks: Vec::new(), unreached: changed.clone() }),
    )
    .await;
    for ran in [&no_fleet, &empty] {
        assert_eq!(ran.command, today.command);
        assert_eq!(ran.narrowed, today.narrowed);
        assert_eq!(ran.mapped, crate::test_map::Mapped::default());
    }
}

/// **A stale edge is still an answer, said as stale.** A test read only from
/// stale edges is named; one a fresh edge also names is not; a file-grain edge
/// runs the tests in that file.
#[tokio::test]
async fn tests_read_from_stale_edges_run_and_are_said() {
    let dir = a_workspace_with_one_test();
    dir.write("crates/fleet/src/tests.rs", "mod session_stop;\nmod other_reach;\n");
    dir.write("crates/fleet/src/tests/other_reach.rs", "#[test]\nfn reads() {}\n");
    let changed = paths(&["crates/fleet/src/other.rs"]);
    let (test, file) = (core_model::EdgeGrain::Test, core_model::EdgeGrain::File);
    let answer = ipc::TestsReaching {
        checks: vec![group(
            "test",
            "fleet",
            vec![
                edge("tests::session_stop", "crates/fleet/src/other.rs", test, true),
                edge("crates/fleet/src/tests/other_reach.rs", "crates/fleet/src/other.rs", file, false),
                edge("tests::other_reach", "crates/fleet/src/other.rs", test, true),
            ],
        )],
        unreached: Vec::new(),
    };
    let ran = mapped(&dir, &changed, Some(answer)).await;
    assert_eq!(
        ran.narrowed,
        Some(Reached::Touched("tests::other_reach tests::session_stop in -p fleet".to_string()))
    );
    assert_eq!(ran.mapped.stale, paths(&["tests::session_stop"]));
    assert_eq!(ran.mapped.before()[1], "stale in the test map: tests::session_stop");
}

/// **A changed file no test reaches is said**, after the verdict, where the map
/// has no edge for it and today's reading finds no test in it. A path the Check
/// does not cover and a test file are not said.
#[tokio::test]
async fn a_changed_file_no_test_reaches_is_said() {
    let dir = a_workspace_with_one_test();
    dir.write("crates/fleet/src/new.rs", "pub fn new() {}\n");
    let changed = paths(&[
        "crates/fleet/src/new.rs",
        "crates/fleet/src/other.rs",
        "crates/fleet/src/tests/session_stop.rs",
        "docs/a.md",
    ]);
    let answer = ipc::TestsReaching {
        checks: vec![group(
            "acceptance",
            "",
            vec![edge("the_board", "crates/fleet/src/other.rs", core_model::EdgeGrain::Test, false)],
        )],
        unreached: paths(&["crates/fleet/src/new.rs", "crates/fleet/src/tests/session_stop.rs", "docs/a.md"]),
    };
    let ran = mapped(&dir, &changed, Some(answer)).await;
    assert_eq!(ran.mapped.unreached, paths(&["crates/fleet/src/new.rs"]));
    assert_eq!(ran.mapped.after(), paths(&["no test reaches crates/fleet/src/new.rs"]));

    // And where nothing runs at all, it is still said.
    let alone = paths(&["crates/fleet/src/new.rs"]);
    let answer = ipc::TestsReaching {
        checks: vec![group(
            "acceptance",
            "",
            vec![edge("the_board", "crates/fleet/src/other.rs", core_model::EdgeGrain::Test, false)],
        )],
        unreached: alone.clone(),
    };
    let ran = mapped(&dir, &alone, Some(answer)).await;
    assert_eq!(ran.narrowed, Some(Reached::NoTests));
    assert_eq!(ran.mapped.unreached, alone);
}

/// **The merge line's reading never asks the map.** What gates runs what the
/// Manifest says.
#[tokio::test]
async fn the_merge_lines_reading_never_asks_the_map() {
    let dir = a_workspace_with_one_test();
    let changed = paths(&["crates/fleet/src/other.rs"]);
    let read = |_: &str, _: &[String]| -> Option<ipc::TestsReaching> {
        panic!("the map was asked for the merge line's reading")
    };
    let ran = crate::declared::execute_told(
        dir.path(),
        Registry::Checks,
        "test",
        Asked::Changed(&changed),
        BUDGET,
        None,
        Priority::Normal,
        None,
        Some(&read),
    )
    .await
    .expect("declared");
    assert_eq!(ran.narrowed, Some(Reached::To("-p fleet".to_string())));
    assert_eq!(ran.mapped, crate::test_map::Mapped::default());
}
