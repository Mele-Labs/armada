//! Fleet feeding the test map from main moving and from a sleeping bay: which stale tests are
//! read again, where, in what order, and what is never run. The collectors are fakes; what a
//! real one makes of a checkout is `adapters`' to assert.

use std::path::Path;
use std::sync::{Arc, Mutex};

use adapter_traits::{CollectorSource, Lap, LapCost, LapGrain, NotRecorded, TestCollector, TestsToRecord};
use adapters::Feeder;
use api::Queries as _;
use core_model::{FieldValue, Level, ManifestId};
use testkit::{FakeCollector, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::tmp::TempDir;

type Rig = Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>;

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

fn lap(test: &str, files: &[&str]) -> Lap {
    Lap {
        member: String::new(),
        test: test.into(),
        grain: LapGrain::Test,
        files: files.iter().map(|file| file.to_string()).collect(),
    }
}

/// A repository holding `f.txt` and `g.txt`, served by a Fleet, with the commit it starts at.
struct Repo {
    home: TempDir,
    fleet: Arc<Rig>,
    id: ManifestId,
    start: String,
}

fn a_repository() -> Repo {
    let home = TempDir::new();
    git(home.path(), &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    for name in ["f.txt", "g.txt"] {
        std::fs::write(home.path().join(name), format!("{name}\n")).unwrap();
    }
    git(home.path(), &["add", "f.txt", "g.txt"]);
    git(home.path(), &["commit", "--quiet", "-m", "start"]);
    let fleet = Arc::new(crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[])));
    let id = fleet.repositories().first().expect("served").manifest().id().clone();
    let start = git(home.path(), &["rev-parse", "HEAD"]);
    Repo { home, fleet, id, start }
}

impl Repo {
    fn root(&self) -> String {
        self.home.path().to_string_lossy().to_string()
    }

    /// Main moves on by a commit that changes `f.txt`, checked out here.
    fn moves_on(&self) -> String {
        std::fs::write(self.home.path().join("f.txt"), "f, changed\n").unwrap();
        git(self.home.path(), &["commit", "--quiet", "-am", "change f"]);
        git(self.home.path(), &["rev-parse", "HEAD"])
    }

    /// A commit changing `f.txt` that this checkout is not on, as a main the checkout has not
    /// caught up to.
    fn moves_on_elsewhere(&self) -> String {
        git(self.home.path(), &["checkout", "--quiet", "-b", "ahead"]);
        let tip = self.moves_on();
        git(self.home.path(), &["checkout", "--quiet", "main"]);
        tip
    }

    /// `collector` records every test it knows at the starting commit, so the map holds its
    /// edges, and `planted` is what Fleet is then given to read the Check with.
    async fn recorded_once(&self, check: &str, collector: &Arc<FakeCollector>, planted: Arc<dyn TestCollector>) {
        self.fleet
            .record_laps(&self.id, check, &self.root(), collector.clone(), TestsToRecord::All)
            .await
            .expect("recorded");
        self.fleet.feed().add(&self.id, Feeder::new(check, planted, None));
    }

    async fn stale(&self) -> Vec<(String, String)> {
        let stale = self.fleet.store().lock().await.stale_tests(&self.id).unwrap();
        stale.into_iter().map(|one| (one.check, one.test)).collect()
    }

    async fn edge_commits(&self, check: &str, test: &str) -> Vec<(String, bool)> {
        let files = self.fleet.store().lock().await.files_a_test_reaches(&self.id, check, "", test).unwrap();
        files.into_iter().map(|edge| (edge.commit, edge.stale)).collect()
    }

    async fn sleeping(&self, on: bool) {
        let mut store = self.fleet.store().lock().await;
        match on {
            true => store.begin_sleep("2026-10-22T00:00:00.000Z").unwrap(),
            false => store.end_sleep().unwrap(),
        }
    }

    fn held_bays(&self) -> usize {
        self.fleet.vcs().slot_holders(&self.root()).iter().flatten().count()
    }
}

fn static_collector() -> Arc<FakeCollector> {
    Arc::new(
        FakeCollector::handing_back(
            "imports",
            CollectorSource::Static,
            vec![lap("a.test", &["f.txt", "a.test"]), lap("b.test", &["g.txt", "b.test"])],
        )
        .costing(LapCost::Nothing)
        .answering_by_name(),
    )
}

fn measured_collector() -> Arc<FakeCollector> {
    Arc::new(
        FakeCollector::handing_back(
            "coverage",
            CollectorSource::Measured,
            vec![lap("m1", &["f.txt"]), lap("m2", &["f.txt"]), lap("m3", &["g.txt"])],
        )
        .needing_a_rebuild()
        .answering_by_name(),
    )
}

#[tokio::test]
async fn main_moving_reads_again_only_the_cheap_tests_whose_files_changed() {
    let repo = a_repository();
    let imports = static_collector();
    repo.recorded_once("web", &imports, imports.clone()).await;
    let tip = repo.moves_on();

    repo.fleet.main_moved_fed(repo.id.as_str(), &repo.start, &tip).await;

    assert_eq!(
        imports.asked().split_off(1),
        vec![("web".to_string(), repo.root(), TestsToRecord::Named(vec!["a.test".into()]))],
        "a.test reaches f.txt, b.test does not, and the checkout is main's own"
    );
    assert_eq!(repo.edge_commits("web", "a.test").await, vec![(tip.clone(), false); 2], "read at the new tip");
    assert_eq!(repo.edge_commits("web", "b.test").await, vec![(repo.start.clone(), false); 2], "untouched");
    assert!(repo.stale().await.is_empty());
    assert_eq!(repo.held_bays(), 0, "main's own checkout is read, so no bay was taken");
}

/// What the map holds for `f.txt`, as `(check, test)` pairs.
async fn reaching_f(repo: &Repo) -> Vec<(String, String)> {
    let reaching = repo.fleet.get_tests_reaching(None, vec!["f.txt".to_string()]).await.unwrap();
    let mut found: Vec<(String, String)> = reaching
        .checks
        .iter()
        .flat_map(|group| group.tests.iter().map(|test| (group.check.clone(), test.test.clone())))
        .collect();
    found.sort();
    found
}

#[tokio::test]
async fn a_cheap_test_that_comes_back_with_no_lap_is_gone_and_its_edges_with_it() {
    let repo = a_repository();
    let before = Arc::new(
        FakeCollector::handing_back(
            "imports",
            CollectorSource::Static,
            vec![lap("a.test", &["f.txt"]), lap("c.test", &["f.txt"])],
        )
        .costing(LapCost::Nothing),
    );
    // The file for c.test was deleted: the collector is asked for it and finds no lap.
    let now = Arc::new(
        FakeCollector::handing_back("imports", CollectorSource::Static, vec![lap("a.test", &["f.txt"])])
            .costing(LapCost::Nothing)
            .answering_by_name(),
    );
    repo.recorded_once("web", &before, now.clone()).await;
    let tip = repo.moves_on();

    repo.fleet.main_moved_fed(repo.id.as_str(), &repo.start, &tip).await;

    assert_eq!(reaching_f(&repo).await, [("web".to_string(), "a.test".to_string())], "c.test is forgotten");
    assert!(repo.stale().await.is_empty(), "and is not asked for again");
}

#[tokio::test]
async fn a_measured_test_that_is_gone_is_forgotten_by_the_sleeping_pass_and_the_rest_are_measured() {
    let repo = a_repository();
    let before = measured_collector();
    // m2 was removed from the suite.
    let now = Arc::new(
        FakeCollector::handing_back("coverage", CollectorSource::Measured, vec![lap("m1", &["f.txt"])])
            .needing_a_rebuild()
            .answering_by_name(),
    );
    a_stale_map(&repo, &before, now.clone()).await;
    repo.sleeping(true).await;

    repo.fleet.sleeping_bay_recorded().await;

    assert_eq!(reaching_f(&repo).await, [("rust".to_string(), "m1".to_string())]);
    assert!(repo.stale().await.is_empty());
}

#[tokio::test]
async fn main_moving_marks_what_a_rebuilding_collector_read_stale_and_never_runs_it() {
    let repo = a_repository();
    let coverage = measured_collector();
    repo.recorded_once("rust", &coverage, coverage.clone()).await;
    let tip = repo.moves_on();

    repo.fleet.main_moved_fed(repo.id.as_str(), &repo.start, &tip).await;

    assert_eq!(coverage.asked().len(), 1, "only the recording that made the edges: it was not run for the move");
    assert_eq!(repo.stale().await, [("rust".to_string(), "m1".to_string()), ("rust".to_string(), "m2".to_string())]);
    let reaching = repo.fleet.get_tests_reaching(None, vec!["f.txt".to_string()]).await.unwrap();
    assert!(
        reaching.checks[0].tests.iter().all(|test| test.stale && test.source == core_model::EdgeSource::Measured.into()),
        "a stale edge is still an answer, said as stale: {reaching:?}"
    );
}

#[tokio::test]
async fn a_main_this_checkout_has_not_caught_up_to_is_read_in_a_free_bay_given_back_after() {
    let repo = a_repository();
    let imports = static_collector();
    repo.recorded_once("web", &imports, imports.clone()).await;
    let tip = repo.moves_on_elsewhere();

    repo.fleet.main_moved_fed(repo.id.as_str(), &repo.start, &tip).await;

    let asked = imports.asked().split_off(1);
    assert_eq!(asked.len(), 1);
    assert!(asked[0].1.contains(".armada/slots/slot-"), "read in a bay, not in {}", asked[0].1);
    assert_eq!(repo.held_bays(), 0, "the bay is back in the pool");
    assert_eq!(repo.fleet.vcs().parked_slots().len(), 1);
}

#[tokio::test]
async fn a_tip_seen_twice_is_one_move_and_the_first_sight_is_none() {
    let repo = a_repository();
    let id = repo.id.as_str();

    repo.fleet.main_tip_seen(id, "1111");
    assert!(repo.fleet.feed().moves().is_empty(), "the first sight only sets what later moves are measured from");

    repo.fleet.main_tip_seen(id, "2222");
    repo.fleet.main_tip_seen(id, "3333");
    assert_eq!(
        repo.fleet.feed().moves(),
        [(id.to_string(), "1111".to_string(), "3333".to_string())],
        "two moves before a pass are one, from the first tip to the last"
    );
    assert!(repo.fleet.feed().moves().is_empty(), "taken once");

    repo.fleet.main_tip_seen(id, "3333");
    assert!(repo.fleet.feed().moves().is_empty(), "the same tip is no move");
}

#[tokio::test]
async fn the_loops_tick_feeds_a_seen_move_on_a_task_of_its_own() {
    let repo = a_repository();
    let imports = static_collector();
    repo.recorded_once("web", &imports, imports.clone()).await;
    repo.fleet.main_tip_seen(repo.id.as_str(), &repo.start);
    let tip = repo.moves_on();
    repo.fleet.main_tip_seen(repo.id.as_str(), &tip);

    repo.fleet.test_map_moves_fed();

    for _ in 0..200 {
        if imports.asked().len() == 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert_eq!(imports.asked().len(), 2, "a.test was read again without anyone awaiting it");
    assert!(repo.fleet.feed().moves().is_empty(), "the move was taken");
}

/// A collector that calls `then` each time it is asked, before answering as `inner` does.
struct Spy {
    inner: Arc<FakeCollector>,
    then: Box<dyn Fn() + Send + Sync>,
}

impl TestCollector for Spy {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn source(&self) -> CollectorSource {
        self.inner.source()
    }

    fn needs_rebuild(&self) -> bool {
        self.inner.needs_rebuild()
    }

    fn lap_cost(&self) -> LapCost {
        self.inner.lap_cost()
    }

    fn record(&self, check: &str, dir: &str, tests: &TestsToRecord) -> Result<Vec<Lap>, NotRecorded> {
        (self.then)();
        self.inner.record(check, dir, tests)
    }
}

/// The measured collector's edges made stale by a move of main: m1 and m2 reach `f.txt`.
async fn a_stale_map(repo: &Repo, coverage: &Arc<FakeCollector>, planted: Arc<dyn TestCollector>) {
    repo.recorded_once("rust", coverage, planted).await;
    let tip = repo.moves_on();
    repo.fleet.main_moved_fed(repo.id.as_str(), &repo.start, &tip).await;
    assert_eq!(repo.stale().await.len(), 2);
}

#[tokio::test]
async fn a_sleeping_bay_measures_the_stale_tests_one_at_a_time_holding_a_place() {
    let repo = a_repository();
    let coverage = measured_collector();
    // Read the places and the bays held while it runs.
    let seen: Arc<Mutex<Vec<(usize, usize)>>> = Arc::default();
    let spied = Arc::new(Spy {
        inner: coverage.clone(),
        then: {
            let (fleet, seen, root) = (repo.fleet.clone(), seen.clone(), repo.root());
            Box::new(move || {
                let held = fleet.vcs().slot_holders(&root).iter().flatten().count();
                seen.lock().unwrap().push((fleet.places_held(), held));
            })
        },
    });
    a_stale_map(&repo, &coverage, spied).await;
    repo.sleeping(true).await;

    repo.fleet.sleeping_bay_recorded().await;

    let asked = coverage.asked().split_off(1);
    assert_eq!(
        asked.iter().map(|(_, _, tests)| tests.clone()).collect::<Vec<_>>(),
        [TestsToRecord::Named(vec!["m1".into()]), TestsToRecord::Named(vec!["m2".into()])],
        "one test a call, not the suite"
    );
    assert!(asked.iter().all(|(check, dir, _)| check == "rust" && dir.contains(".armada/slots/slot-")), "{asked:?}");
    assert_eq!(*seen.lock().unwrap(), [(1, 1), (1, 1)], "a place and a bay were held for each");
    assert_eq!((repo.fleet.places_held(), repo.held_bays()), (0, 0), "both given back");
    assert!(repo.stale().await.is_empty());
    assert_eq!(repo.fleet.vcs().parked_slots().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn sleep_going_off_between_tests_stops_the_pass_and_gives_the_bay_back() {
    let repo = a_repository();
    let coverage = measured_collector();
    let spied = Arc::new(Spy {
        inner: coverage.clone(),
        then: {
            let fleet = repo.fleet.clone();
            Box::new(move || {
                tokio::runtime::Handle::current().block_on(async { fleet.store().lock().await.end_sleep().unwrap() })
            })
        },
    });
    a_stale_map(&repo, &coverage, spied).await;
    repo.sleeping(true).await;

    repo.fleet.sleeping_bay_recorded().await;

    assert_eq!(coverage.asked().len(), 2, "the recording that made the edges, and the first test only");
    assert_eq!(repo.stale().await, [("rust".to_string(), "m2".to_string())], "the second was left stale");
    assert_eq!((repo.fleet.places_held(), repo.held_bays()), (0, 0), "the place and the bay are back");
}

#[tokio::test]
async fn with_no_free_bay_a_sleeping_pass_records_nothing_and_waits_for_none() {
    let repo = a_repository();
    let coverage = measured_collector();
    a_stale_map(&repo, &coverage, coverage.clone()).await;
    let root = repo.root();
    let pool = repo.fleet.repositories().first().expect("served").manifest().worktrees().get();
    for slot in 1..=pool {
        repo.fleet.vcs().hold_slot(&root, slot, "a-session");
    }
    repo.sleeping(true).await;

    repo.fleet.sleeping_bay_recorded().await;

    assert_eq!(coverage.asked().len(), 1, "no test was measured");
    assert_eq!(repo.stale().await.len(), 2);
    assert_eq!(repo.fleet.places_held(), 0);
}

#[tokio::test]
async fn with_sleep_off_nothing_is_leased_or_measured() {
    let repo = a_repository();
    let coverage = measured_collector();
    a_stale_map(&repo, &coverage, coverage.clone()).await;

    repo.fleet.sleeping_bay_recorded().await;

    assert_eq!(coverage.asked().len(), 1);
    assert_eq!(repo.held_bays(), 0);
    assert!(repo.fleet.vcs().parked_slots().is_empty(), "no bay was leased to be given back");
    assert_eq!(repo.stale().await.len(), 2);
}

#[tokio::test]
async fn a_map_with_no_stale_edge_takes_no_bay_so_a_new_repository_is_not_seeded_from_here() {
    let repo = a_repository();
    let coverage = measured_collector();
    repo.fleet.feed().add(&repo.id, Feeder::new("rust", coverage.clone(), None));
    repo.sleeping(true).await;

    repo.fleet.sleeping_bay_recorded().await;

    assert!(coverage.asked().is_empty(), "a map holding no edges is the forge's to seed");
    assert!(repo.fleet.vcs().parked_slots().is_empty());
}

#[tokio::test]
async fn a_sleeping_pass_leaves_a_cheap_collectors_stale_tests_to_main_moving() {
    let repo = a_repository();
    let imports = static_collector();
    repo.recorded_once("web", &imports, imports.clone()).await;
    repo.moves_on();
    // Marked stale, as a move of main does, without the move reading them again.
    repo.fleet
        .store()
        .lock()
        .await
        .mark_test_map_stale(&repo.id, &["f.txt".to_string()])
        .unwrap();
    repo.sleeping(true).await;

    repo.fleet.sleeping_bay_recorded().await;

    assert_eq!(imports.asked().len(), 1, "only the recording that made the edges");
    assert_eq!(repo.fleet.vcs().parked_slots().len(), 0, "no bay for a collector that costs nothing");
}

#[tokio::test]
async fn a_recording_is_logged_with_the_repository_the_collector_the_tests_and_how_long() {
    let repo = a_repository();
    let envelope = repo.fleet.feed_envelope(
        Level::Info,
        "test map recorded",
        &repo.id,
        [
            ("collector", FieldValue::Str("coverage".into())),
            ("tests", FieldValue::List(vec![FieldValue::Str("m1".into())])),
            ("took_ms", FieldValue::Int(3)),
        ],
    );
    let fields = envelope.fields();
    assert_eq!(envelope.msg(), "test map recorded");
    assert_eq!(fields["repository"], FieldValue::Str(repo.id.as_str().to_string()));
    assert_eq!(fields["collector"], FieldValue::Str("coverage".into()));
    assert!(fields.contains_key("tests") && fields.contains_key("took_ms"));
}
