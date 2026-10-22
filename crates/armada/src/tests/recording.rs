//! `armada test-map record`: the laps a collector hands back are written, with the commit they
//! were read at, to a file `ipc::decode` reads. The collector is a fake behind [`Pick`]: nothing
//! here builds this repository with coverage on.

use std::sync::Arc;

use adapter_traits::{CollectorSource, Lap, LapCost, LapGrain, NotRecorded, TestCollector, TestsToRecord};
use checks_runner::CheckWidth;
use testkit::FakeCollector;

use crate::recording::{collector_for, record, NotRecorded as Refused};
use crate::tests::TempDir;

/// Lets the test keep a handle on the collector the command was handed.
struct Shared(Arc<FakeCollector>);

impl TestCollector for Shared {
    fn name(&self) -> &str {
        self.0.name()
    }
    fn source(&self) -> CollectorSource {
        self.0.source()
    }
    fn needs_rebuild(&self) -> bool {
        self.0.needs_rebuild()
    }
    fn lap_cost(&self) -> LapCost {
        self.0.lap_cost()
    }
    fn record(&self, check: &str, dir: &str, tests: &TestsToRecord) -> Result<Vec<Lap>, NotRecorded> {
        self.0.record(check, dir, tests)
    }
}

fn git(root: &std::path::Path, args: &[&str]) -> String {
    let done = std::process::Command::new("git")
        .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "{args:?}: {}", String::from_utf8_lossy(&done.stderr));
    String::from_utf8_lossy(&done.stdout).trim().to_string()
}

/// A committed repository whose Check `test` runs `run`.
fn a_repository_whose_test_runs(run: &str) -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        &format!(
            "version: 1\nid: a-test-project\nchecks:\n  test:\n    run: {run}\n    \
             narrow:\n      run: {run} -p xtask\n      each: \"-p {{}}\"\n      under: crates\n      except:\n        - acceptance\n"
        ),
    );
    git(dir.path(), &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    git(dir.path(), &["add", "armada.yml"]);
    git(dir.path(), &["commit", "--quiet", "-m", "start"]);
    dir
}

fn laps() -> Vec<Lap> {
    vec![
        Lap {
            member: "store".into(),
            test: "reads_a_row".into(),
            grain: LapGrain::Test,
            files: vec!["crates/store/src/row.rs".into()],
        },
        Lap { member: String::new(), test: "tests/store.test.ts".into(), grain: LapGrain::File, files: Vec::new() },
    ]
}

#[test]
fn the_laps_are_written_with_the_commit_they_were_read_at_and_read_back() {
    let repository = a_repository_whose_test_runs("cargo nextest run --workspace");
    let commit = git(repository.path(), &["rev-parse", "HEAD"]);
    let out = TempDir::new();
    let file = out.path().join("below/laps.json");
    let fake = Arc::new(FakeCollector::handing_back("fixed", CollectorSource::Measured, laps()));
    let held = fake.clone();
    let pick = move |_: &config::Check, _: CheckWidth| Some(Box::new(Shared(held.clone())) as Box<dyn TestCollector>);

    let recorded = record(repository.path(), "test", &file, CheckWidth::of(2), &pick).expect("recorded");

    assert_eq!((recorded.laps, recorded.commit.as_str()), (2, commit.as_str()));
    let read: ipc::RecordedLaps =
        ipc::decode("recorded laps", &std::fs::read(&file).expect("written")).expect("decodes");
    assert_eq!((read.commit.as_str(), read.check.as_str()), (commit.as_str(), "test"));
    assert_eq!(read.source, core_model::EdgeSource::Measured.into());
    assert_eq!(read.laps.len(), 2);
    assert_eq!((read.laps[0].test.as_str(), read.laps[0].grain), ("reads_a_row", core_model::EdgeGrain::Test.into()));
    assert_eq!(read.laps[0].files, vec!["crates/store/src/row.rs".to_string()]);
    assert_eq!(read.laps[1].grain, core_model::EdgeGrain::File.into());
    assert!(read.laps[1].files.is_empty(), "a test that reached nothing is still a lap");
    assert_eq!(
        fake.asked(),
        vec![("test".to_string(), repository.path().to_string_lossy().to_string(), TestsToRecord::All)],
        "every test of the Check, run in the repository"
    );
}

#[test]
fn a_check_no_collector_answers_for_writes_nothing() {
    let repository = a_repository_whose_test_runs("pnpm test");
    let out = TempDir::new();
    let file = out.path().join("laps.json");

    let refused = record(repository.path(), "test", &file, CheckWidth::of(2), &collector_for).expect_err("refused");

    assert_eq!(refused, Refused::NoCollector { check: "test".into(), run: "pnpm test".into() });
    assert!(!file.exists());
    assert!(refused.to_string().contains("pnpm test"), "{refused}");
}

#[test]
fn a_check_the_manifest_does_not_declare_is_named_back() {
    let repository = a_repository_whose_test_runs("cargo nextest run --workspace");
    let out = TempDir::new();
    let refused = record(repository.path(), "nope", &out.path().join("laps.json"), CheckWidth::of(2), &collector_for)
        .expect_err("refused");
    assert!(refused.to_string().contains("`nope` is not a Check declared in"), "{refused}");
}

#[test]
fn a_collector_that_records_nothing_is_reported_in_its_own_words() {
    struct Broken;
    impl TestCollector for Broken {
        fn name(&self) -> &str {
            "broken"
        }
        fn source(&self) -> CollectorSource {
            CollectorSource::Measured
        }
        fn needs_rebuild(&self) -> bool {
            true
        }
        fn lap_cost(&self) -> LapCost {
            LapCost::Run
        }
        fn record(&self, _: &str, _: &str, _: &TestsToRecord) -> Result<Vec<Lap>, NotRecorded> {
            Err(NotRecorded("llvm-profdata is not installed".into()))
        }
    }
    let repository = a_repository_whose_test_runs("cargo nextest run --workspace");
    let out = TempDir::new();
    let file = out.path().join("laps.json");
    let pick = |_: &config::Check, _: CheckWidth| Some(Box::new(Broken) as Box<dyn TestCollector>);

    let refused = record(repository.path(), "test", &file, CheckWidth::of(2), &pick).expect_err("refused");

    assert_eq!(refused.to_string(), "the broken collector recorded nothing: llvm-profdata is not installed");
    assert!(!file.exists(), "a failed record leaves no file for Fleet to find");
}

#[test]
fn a_check_that_runs_nextest_gets_the_coverage_collector_and_no_other_does() {
    let rust = a_repository_whose_test_runs("cargo nextest run --workspace --exclude acceptance");
    let node = a_repository_whose_test_runs("pnpm test");
    let in_rust = crate::manifests::Manifests::load(rust.path()).expect("loads");
    let in_node = crate::manifests::Manifests::load(node.path()).expect("loads");

    let picked = collector_for(in_rust.root().check("test").expect("declared"), CheckWidth::of(4)).expect("picked");
    assert_eq!((picked.name(), picked.source()), ("nextest coverage", CollectorSource::Measured));
    assert!(picked.needs_rebuild(), "so it records only where a rebuild costs nobody a turn");
    assert!(collector_for(in_node.root().check("test").expect("declared"), CheckWidth::of(4)).is_none());
}
