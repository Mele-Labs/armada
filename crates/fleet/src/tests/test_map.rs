//! Laps a collector hands back become edges of the repository's map, stamped with the collector's
//! source and the commit the directory is at; a measured edge outranks a static one for the same
//! (test, file); and the route groups the answer by Check and member and names the unreached.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{CollectorSource, Lap, LapGrain, TestCollector, TestsToRecord};
use api::Queries;
use core_model::{EdgeGrain, EdgeSource, ManifestId};
use testkit::{FakeCollector, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::test_map::Collectors;
use crate::tests::tmp::TempDir;

fn served_by(fleet: &Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>) -> ManifestId {
    fleet.repositories().first().expect("served").manifest().id().clone()
}

fn git(root: &Path, args: &[&str]) -> String {
    let done = std::process::Command::new("git")
        .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "{args:?}: {}", String::from_utf8_lossy(&done.stderr));
    String::from_utf8_lossy(&done.stdout).trim().to_string()
}

/// A repository with one commit, so a lap has a commit to be read at.
fn a_committed_repository() -> TempDir {
    let home = TempDir::new();
    git(home.path(), &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    std::fs::write(home.path().join("f.txt"), "f\n").unwrap();
    git(home.path(), &["add", "f.txt"]);
    git(home.path(), &["commit", "--quiet", "-m", "start"]);
    home
}

fn lap(member: &str, test: &str, grain: LapGrain, files: &[&str]) -> Lap {
    Lap {
        member: member.into(),
        test: test.into(),
        grain,
        files: files.iter().map(|file| file.to_string()).collect(),
    }
}

fn paths(named: &[&str]) -> Vec<String> {
    named.iter().map(|path| path.to_string()).collect()
}

#[tokio::test]
async fn laps_become_edges_stamped_with_the_collectors_source_and_the_directorys_commit() {
    let home = a_committed_repository();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let repository = served_by(&fleet);
    let dir = home.path().to_string_lossy().to_string();
    let commit = git(home.path(), &["rev-parse", "HEAD"]);
    let collector = Arc::new(FakeCollector::handing_back(
        "fixed",
        CollectorSource::Static,
        vec![
            lap("store", "reads_a_row", LapGrain::Test, &["crates/store/src/row.rs", "crates/store/src/open.rs"]),
            lap("", "tests/store.test.ts", LapGrain::File, &["src/store.ts"]),
        ],
    ));

    let before = fleet.now();
    let kept = fleet
        .record_laps(&repository, "test", &dir, collector.clone(), TestsToRecord::Named(vec!["reads_a_row".into()]))
        .await
        .expect("recorded");
    let after = fleet.now();
    assert_eq!(kept, 3);
    assert_eq!(
        collector.asked(),
        vec![("test".to_string(), dir.clone(), TestsToRecord::Named(vec!["reads_a_row".into()]))],
        "the collector is asked for the Check, in the directory, for the tests named"
    );

    let store = fleet.store().lock().await;
    let files = store.files_a_test_reaches(&repository, "test", "store", "reads_a_row").expect("reads");
    assert_eq!(files.len(), 2);
    for edge in &files {
        assert_eq!((edge.source, edge.grain, edge.commit.as_str(), edge.stale), (EdgeSource::Static, EdgeGrain::Test, commit.as_str(), false));
        assert!(before < edge.at && edge.at < after, "stamped by Fleet's clock while recording: {:?}", edge.at);
    }
    let file_grain = store.files_a_test_reaches(&repository, "test", "", "tests/store.test.ts").expect("reads");
    assert_eq!((file_grain.len(), file_grain[0].grain, file_grain[0].file.as_str()), (1, EdgeGrain::File, "src/store.ts"));
}

/// Two collectors see the same (test, file); the route answers the measured one, says so, and
/// names the path no collector reached.
#[tokio::test]
async fn a_measured_edge_outranks_a_static_one_on_the_route_and_the_unreached_are_named() {
    let home = a_committed_repository();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let repository = served_by(&fleet);
    let dir = home.path().to_string_lossy().to_string();
    let seen = |source| {
        Arc::new(FakeCollector::handing_back(
            "either",
            source,
            vec![lap("store", "reads_a_row", LapGrain::Test, &["crates/store/src/row.rs"])],
        )) as Arc<dyn TestCollector>
    };
    for source in [CollectorSource::Measured, CollectorSource::Static] {
        fleet
            .record_laps(&repository, "test", &dir, seen(source), TestsToRecord::All)
            .await
            .expect("recorded");
    }
    fleet
        .record_laps(
            &repository,
            "typecheck",
            &dir,
            Arc::new(FakeCollector::handing_back(
                "graph",
                CollectorSource::Static,
                vec![lap("", "src/app.test.ts", LapGrain::File, &["src/app.ts"])],
            )),
            TestsToRecord::All,
        )
        .await
        .expect("recorded");

    let reaching = fleet
        .get_tests_reaching(None, paths(&["crates/store/src/row.rs", "src/app.ts", "docs/nothing.md"]))
        .await
        .expect("answered");
    assert_eq!(reaching.unreached, paths(&["docs/nothing.md"]));
    assert!(reaching.holds_edges);
    let groups: Vec<(&str, &str, usize)> = reaching
        .checks
        .iter()
        .map(|group| (group.check.as_str(), group.member.as_str(), group.tests.len()))
        .collect();
    assert_eq!(groups, vec![("test", "store", 1), ("typecheck", "", 1)]);
    let row = &reaching.checks[0].tests[0];
    assert_eq!((row.test.as_str(), row.source.as_wire(), row.grain.as_wire(), row.stale), ("reads_a_row", "measured", "test", false));
    assert_eq!(reaching.checks[1].tests[0].grain.as_wire(), "file");

    let unknown = fleet
        .get_tests_reaching(Some(ipc::ManifestId::carried("01NOBODY")), paths(&["src/app.ts"]))
        .await;
    assert!(unknown.is_err(), "a Manifest this Fleet does not serve is refused");
}

/// A directory with no commit to read at records nothing and says why, and the registry keeps
/// collectors sharpest first.
#[tokio::test]
async fn no_commit_means_no_edges_and_the_registry_is_sharpest_first() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let repository = served_by(&fleet);
    let nowhere = TempDir::new();
    let refused = fleet
        .record_laps(
            &repository,
            "test",
            &nowhere.path().to_string_lossy(),
            Arc::new(FakeCollector::handing_back("fixed", CollectorSource::Measured, vec![])),
            TestsToRecord::All,
        )
        .await;
    assert!(matches!(refused, Err(crate::test_map::NotMapped::NoCommit { .. })), "{refused:?}");

    let mut collectors = Collectors::none();
    for (name, source) in [("graph", CollectorSource::Static), ("model", CollectorSource::Reading), ("coverage", CollectorSource::Measured)] {
        collectors.add(Arc::new(FakeCollector::handing_back(name, source, vec![])));
    }
    let names: Vec<&str> = collectors.iter().map(|collector| collector.name()).collect();
    assert_eq!(names, vec!["coverage", "graph", "model"]);
    assert!(collectors.named("model").is_some() && collectors.named("nobody").is_none());
}
