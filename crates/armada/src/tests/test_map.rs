//! What Fleet's test map adds to an agent's `armada check <name> --changed`, read from a map
//! the test writes, never the machine's Fleet. `crate::test_map`.

use checks_runner::Priority;

use crate::declared::{Asked, Ran, Reached, Registry};
use crate::tests::reaching::{a_workspace_with_one_test, asked, paths, BUDGET};
use crate::tests::TempDir;

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
        holds_edges: true,
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
        holds_edges: true,
    };
    let ran = mapped(&dir, &changed, Some(answer)).await;
    assert_eq!(ran.narrowed, Some(Reached::NoTests));
    assert!(ran.command.is_empty(), "{}", ran.command);
    assert!(ran.mapped.added.is_empty());
}

/// **No Fleet, or a map that holds no edge for the repository, is today's
/// run**, and says nothing more: not even the paths the empty map calls unreached.
#[tokio::test]
async fn no_fleet_or_an_empty_map_runs_as_without_a_map() {
    let dir = a_workspace_with_one_test();
    let changed = paths(&["crates/fleet/src/tests/session_stop.rs", "crates/fleet/src/other.rs"]);
    let today = asked(&dir, Asked::Touched(&changed)).await.expect("declared");
    let no_fleet = mapped(&dir, &changed, None).await;
    let empty = mapped(
        &dir,
        &changed,
        Some(ipc::TestsReaching { checks: Vec::new(), unreached: changed.clone(), holds_edges: false }),
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
        holds_edges: true,
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
        holds_edges: true,
    };
    let ran = mapped(&dir, &changed, Some(answer)).await;
    assert_eq!(ran.mapped.unreached, paths(&["crates/fleet/src/new.rs"]));
    assert_eq!(ran.mapped.after(), paths(&["no test reaches crates/fleet/src/new.rs"]));
}

/// **Every changed path unreached, in a map that holds edges, is said**, and
/// nothing runs: the answer names no edge for any of them, and the map is not
/// empty, so each is a file nothing is known to reach.
#[tokio::test]
async fn every_changed_path_unreached_in_a_map_that_holds_edges_is_said() {
    let dir = a_workspace_with_one_test();
    dir.write("crates/fleet/src/new.rs", "pub fn new() {}\n");
    let changed = paths(&["crates/fleet/src/new.rs", "crates/fleet/src/other.rs"]);
    let answer = ipc::TestsReaching { checks: Vec::new(), unreached: changed.clone(), holds_edges: true };
    let ran = mapped(&dir, &changed, Some(answer)).await;
    assert_eq!(ran.narrowed, Some(Reached::NoTests));
    assert!(ran.command.is_empty(), "{}", ran.command);
    assert_eq!(ran.mapped.unreached, changed);
    assert_eq!(
        ran.mapped.after(),
        paths(&["no test reaches crates/fleet/src/new.rs", "no test reaches crates/fleet/src/other.rs"])
    );
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

/// **A long change is asked over several requests**, every path in one of
/// them, in order, so a large diff never meets a listener's request limit.
#[test]
fn a_long_change_is_asked_in_several_requests_and_loses_no_path() {
    let many: Vec<String> = (0..400).map(|n| format!("crates/fleet/src/tests/some_long_module_{n}.rs")).collect();
    let runs = crate::test_map::requests(&many);
    assert!(runs.len() > 1, "{} paths went in one request", many.len());
    assert_eq!(runs.concat(), many);
    for run in &runs {
        let encoded = ipc::door::encoded(&run.join("\n")).len();
        assert!(encoded <= 4096, "a request carried {encoded} bytes of paths");
    }
}
