//! A repository's test map: edges read back per path with the sharpest source winning, a re-record
//! standing in for the last, stale said as stale, and a path nothing reaches named as such.

use core_model::{EdgeGrain, EdgeSource, ManifestId, TestMapEdge, Timestamp, Ulid};

use crate::tests::{open, TempDir};
use crate::StaleTest;

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn at(second: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-10-21T00:00:{second:02}Z"))
}

fn edge(test: &str, file: &str, source: EdgeSource, commit: &str, second: u32) -> TestMapEdge {
    TestMapEdge {
        repository: repository("01REPO"),
        check: "test".into(),
        member: "store".into(),
        test: test.into(),
        file: file.into(),
        source,
        grain: EdgeGrain::Test,
        commit: commit.into(),
        at: at(second),
        stale: false,
    }
}

fn paths(named: &[&str]) -> Vec<String> {
    named.iter().map(|path| path.to_string()).collect()
}

#[test]
fn a_path_nothing_reaches_is_named_and_an_edge_reads_back_whole_after_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut file_grain = edge("tests/store.test.ts", "src/store.ts", EdgeSource::Static, "abc123", 1);
    file_grain.grain = EdgeGrain::File;
    store
        .record_test_map_edges(&[edge("reads_a_row", "src/row.rs", EdgeSource::Measured, "abc123", 1), file_grain.clone()])
        .expect("kept");
    drop(store);

    let store = open(&dir);
    let reaching = store
        .tests_reaching(&repository("01REPO"), &paths(&["src/row.rs", "src/store.ts", "src/nobody.rs"]))
        .expect("reads");
    assert_eq!(reaching.unreached, paths(&["src/nobody.rs"]));
    assert!(reaching.holds_edges);
    assert_eq!(reaching.edges.len(), 2);
    assert!(reaching.edges.contains(&file_grain), "the file-grain edge reads back as recorded: {:?}", reaching.edges);
    let measured = reaching.edges.iter().find(|edge| edge.file == "src/row.rs").expect("the measured edge");
    assert_eq!((measured.source, measured.grain, measured.commit.as_str(), measured.stale), (EdgeSource::Measured, EdgeGrain::Test, "abc123", false));
    let other = store.tests_reaching(&repository("01OTHER"), &paths(&["src/row.rs"])).expect("reads");
    assert!(other.edges.is_empty(), "another repository's edge leaked in");
    assert!(!other.holds_edges, "a repository with no edge of its own holds none");
    let elsewhere = store.tests_reaching(&repository("01REPO"), &paths(&["src/nobody.rs"])).expect("reads");
    assert_eq!((elsewhere.unreached.len(), elsewhere.holds_edges), (1, true), "every path unreached, in a map that holds edges");
}

/// Two sources that both saw a (test, file) each keep their row, and a read answers the sharper.
#[test]
fn the_sharpest_source_answers_for_a_test_and_file_both_sources_saw() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .record_test_map_edges(&[
            edge("reads_a_row", "src/row.rs", EdgeSource::Static, "abc123", 1),
            edge("reads_a_row", "src/row.rs", EdgeSource::Reading, "abc123", 2),
        ])
        .expect("kept");
    store
        .record_test_map_edges(&[edge("reads_a_row", "src/row.rs", EdgeSource::Measured, "def456", 3)])
        .expect("kept");

    let reaching = store.tests_reaching(&repository("01REPO"), &paths(&["src/row.rs"])).expect("reads");
    assert_eq!(reaching.edges.len(), 1, "one edge per (test, file): {:?}", reaching.edges);
    assert_eq!(reaching.edges[0].source, EdgeSource::Measured);
    assert_eq!(reaching.edges[0].commit, "def456");
    assert!(reaching.unreached.is_empty());
}

/// A source recording the same test again stands in for its last record of it: a file the test
/// stopped reaching is gone, and the other source's edges stay.
#[test]
fn a_re_record_from_one_source_stands_in_for_its_last_and_leaves_the_other_sources() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .record_test_map_edges(&[
            edge("reads_a_row", "src/row.rs", EdgeSource::Measured, "abc123", 1),
            edge("reads_a_row", "src/old.rs", EdgeSource::Measured, "abc123", 1),
            edge("reads_a_row", "src/old.rs", EdgeSource::Static, "abc123", 1),
        ])
        .expect("kept");
    store
        .record_test_map_edges(&[edge("reads_a_row", "src/row.rs", EdgeSource::Measured, "def456", 2)])
        .expect("kept again");

    let files = store
        .files_a_test_reaches(&repository("01REPO"), "test", "store", "reads_a_row")
        .expect("reads");
    let named: Vec<(&str, EdgeSource)> = files.iter().map(|edge| (edge.file.as_str(), edge.source)).collect();
    assert_eq!(named, vec![("src/old.rs", EdgeSource::Static), ("src/row.rs", EdgeSource::Measured)]);
    assert_eq!(files[1].commit, "def456");
}

/// Marking a path stale marks every edge naming it, a stale edge is still answered and says so, and
/// the tests to redo are listed once per source.
#[test]
fn a_stale_edge_is_still_an_answer_and_names_the_test_to_redo() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .record_test_map_edges(&[
            edge("reads_a_row", "src/row.rs", EdgeSource::Measured, "abc123", 1),
            edge("reads_a_row", "src/row.rs", EdgeSource::Static, "abc123", 1),
            edge("opens", "src/open.rs", EdgeSource::Measured, "abc123", 1),
        ])
        .expect("kept");
    let repo = repository("01REPO");
    assert!(store.stale_tests(&repo).expect("reads").is_empty());

    assert_eq!(store.mark_test_map_stale(&repo, &paths(&["src/row.rs", "src/nobody.rs"])).expect("marked"), 2);
    let reaching = store.tests_reaching(&repo, &paths(&["src/row.rs", "src/open.rs"])).expect("reads");
    let by_file: Vec<(&str, bool)> = reaching.edges.iter().map(|edge| (edge.file.as_str(), edge.stale)).collect();
    assert!(by_file.contains(&("src/row.rs", true)) && by_file.contains(&("src/open.rs", false)), "{by_file:?}");
    assert_eq!(
        store.stale_tests(&repo).expect("reads"),
        vec![
            StaleTest { check: "test".into(), member: "store".into(), test: "reads_a_row".into(), source: EdgeSource::Static },
            StaleTest { check: "test".into(), member: "store".into(), test: "reads_a_row".into(), source: EdgeSource::Measured },
        ]
    );

    store
        .record_test_map_edges(&[edge("reads_a_row", "src/row.rs", EdgeSource::Measured, "def456", 2)])
        .expect("kept again");
    assert_eq!(
        store.stale_tests(&repo).expect("reads"),
        vec![StaleTest { check: "test".into(), member: "store".into(), test: "reads_a_row".into(), source: EdgeSource::Static }],
        "a re-record from one source clears only that source's stale mark"
    );
}

#[test]
fn forgetting_a_test_drops_its_edges_from_one_source_in_one_check_and_leaves_the_rest() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut other_check = edge("gone", "src/row.rs", EdgeSource::Static, "abc", 1);
    other_check.check = "lint".into();
    store
        .record_test_map_edges(&[
            edge("gone", "src/row.rs", EdgeSource::Static, "abc", 1),
            edge("gone", "src/open.rs", EdgeSource::Static, "abc", 1),
            edge("gone", "src/row.rs", EdgeSource::Measured, "abc", 1),
            edge("kept", "src/row.rs", EdgeSource::Static, "abc", 1),
            other_check,
        ])
        .expect("kept");

    let forgotten = store
        .forget_test_map_tests(&repository("01REPO"), "test", EdgeSource::Static, &["gone".to_string()])
        .expect("forgotten");

    assert_eq!(forgotten, 2, "both files the static source saw it reach");
    let left = store.tests_reaching(&repository("01REPO"), &paths(&["src/row.rs", "src/open.rs"])).expect("reads");
    let mut names: Vec<(String, String, EdgeSource)> =
        left.edges.iter().map(|edge| (edge.check.clone(), edge.test.clone(), edge.source)).collect();
    names.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        names,
        [
            ("lint".to_string(), "gone".to_string(), EdgeSource::Static),
            ("test".to_string(), "gone".to_string(), EdgeSource::Measured),
            ("test".to_string(), "kept".to_string(), EdgeSource::Static),
        ],
        "the measured edge of the same name, another Check's, and another test all stay"
    );
    assert_eq!(
        store.forget_test_map_tests(&repository("01REPO"), "test", EdgeSource::Static, &["gone".to_string()]).expect("again"),
        0
    );
}
