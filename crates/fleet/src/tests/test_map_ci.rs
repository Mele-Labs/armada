//! The laps main's CI run recorded, fetched from a fake forge: kept as edges at the commit the file
//! says, kept once, silent where there is none, and dropped for a commit main has left.
//! `crate::test_map_ci`.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{LapsOnForge, NotAsked};
use core_model::{EdgeGrain, EdgeSource, ManifestId};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::test_map_ci::{Pulled, ASKS};
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const TIP: &str = "4f2a9c1e07b35d68a1c4e9f02b7d5a3c8e6f1b24";

fn a_fleet() -> (Arc<Hosted>, TempDir, ManifestId, String) {
    let home = TempDir::new();
    let fleet = Arc::new(crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[])));
    let served = fleet.repositories().first().expect("served");
    let (repository, root) = (served.manifest().id().clone(), served.root().to_string());
    (fleet, home, repository, root)
}

fn recorded(commit: &str) -> ipc::RecordedLaps {
    ipc::RecordedLaps {
        commit: commit.to_string(),
        check: "test".to_string(),
        source: EdgeSource::Measured.into(),
        laps: vec![
            ipc::RecordedLap {
                member: "store".into(),
                test: "reads_a_row".into(),
                grain: EdgeGrain::Test.into(),
                files: vec!["crates/store/src/row.rs".into(), "crates/store/src/open.rs".into()],
            },
            ipc::RecordedLap {
                member: String::new(),
                test: "tests/store.test.ts".into(),
                grain: EdgeGrain::File.into(),
                files: vec!["src/store.ts".into()],
            },
        ],
    }
}

fn uploaded(commit: &str) -> Result<LapsOnForge, NotAsked> {
    Ok(LapsOnForge::Found(ipc::encode(&recorded(commit)).expect("encodes").into_bytes()))
}

async fn edges_held(fleet: &Hosted, repository: &ManifestId) -> bool {
    let paths = vec!["crates/store/src/row.rs".to_string()];
    fleet.store().lock().await.tests_reaching(repository, &paths).expect("reads").holds_edges
}

#[tokio::test]
async fn an_uploaded_file_is_kept_as_edges_at_the_commit_it_was_read_at() {
    let (fleet, _home, repository, root) = a_fleet();
    fleet.vcs().laps_on_the_forge(uploaded(TIP));

    let pulled = fleet.test_map_laps_pulled(&repository, &root, TIP).await;

    assert_eq!(pulled, Pulled::Kept { edges: 3 });
    assert_eq!(fleet.vcs().asked_for_laps_of(), vec![TIP.to_string()]);
    let store = fleet.store().lock().await;
    let files = store.files_a_test_reaches(&repository, "test", "store", "reads_a_row").expect("reads");
    assert_eq!(files.len(), 2);
    for edge in &files {
        assert_eq!(
            (edge.source, edge.grain, edge.commit.as_str(), edge.stale),
            (EdgeSource::Measured, EdgeGrain::Test, TIP, false),
            "the commit is the file's: this directory is not at it"
        );
    }
    let file_grain = store.files_a_test_reaches(&repository, "test", "", "tests/store.test.ts").expect("reads");
    assert_eq!((file_grain.len(), file_grain[0].grain), (1, EdgeGrain::File));
}

#[tokio::test]
async fn a_file_already_kept_is_not_written_again() {
    let (fleet, _home, repository, root) = a_fleet();
    fleet.vcs().laps_on_the_forge(uploaded(TIP));
    assert_eq!(fleet.test_map_laps_pulled(&repository, &root, TIP).await, Pulled::Kept { edges: 3 });
    // A write would make every edge fresh again.
    let marked = fleet
        .store()
        .lock()
        .await
        .mark_test_map_stale(&repository, &["crates/store/src/row.rs".to_string()])
        .expect("marked");
    assert_eq!(marked, 1);

    assert_eq!(fleet.test_map_laps_pulled(&repository, &root, TIP).await, Pulled::AlreadyKept);

    let paths = vec!["crates/store/src/row.rs".to_string()];
    let reaching = fleet.store().lock().await.tests_reaching(&repository, &paths).expect("reads");
    assert!(reaching.edges.iter().all(|edge| edge.stale), "the second pull left the stale mark alone");
}

#[tokio::test]
async fn nothing_uploaded_yet_or_ever_says_nothing_and_keeps_nothing() {
    let (fleet, _home, repository, root) = a_fleet();
    for (held, said) in [(Ok(LapsOnForge::NotYet), Pulled::NotYet), (Ok(LapsOnForge::Never), Pulled::Never)] {
        fleet.vcs().laps_on_the_forge(held);
        assert_eq!(fleet.test_map_laps_pulled(&repository, &root, TIP).await, said);
    }
    assert!(!edges_held(&fleet, &repository).await, "no edge, and so a map nothing has fed");
}

#[tokio::test]
async fn a_run_still_going_is_asked_again_and_a_settled_answer_is_not() {
    let (fleet, _home, repository, root) = a_fleet();
    fleet.vcs().laps_on_the_forge(Ok(LapsOnForge::NotYet));

    let pulled = fleet.test_map_laps_asked_until_settled(&repository, &root, TIP, Duration::ZERO).await;
    assert_eq!(pulled, Pulled::NotYet);
    assert_eq!(fleet.vcs().asked_for_laps_of().len(), ASKS as usize, "every ask there is, then it stops");

    fleet.vcs().laps_on_the_forge(uploaded(TIP));
    let pulled = fleet.test_map_laps_asked_until_settled(&repository, &root, TIP, Duration::ZERO).await;
    assert_eq!(pulled, Pulled::Kept { edges: 3 });
    assert_eq!(fleet.vcs().asked_for_laps_of().len(), ASKS as usize + 1, "one more, and it came");

    let (other, _other_home, other_repository, other_root) = a_fleet();
    other.vcs().laps_on_the_forge(Err(NotAsked("could not find any workflows named checks.yml".into())));
    let pulled = other.test_map_laps_asked_until_settled(&other_repository, &other_root, TIP, Duration::ZERO).await;
    assert_eq!(pulled, Pulled::NotAsked("could not find any workflows named checks.yml".into()));
    assert_eq!(other.vcs().asked_for_laps_of().len(), 1, "a forge that cannot be asked costs one ask");
}

#[tokio::test]
async fn laps_for_a_tip_main_has_left_are_not_even_asked_for() {
    let (fleet, _home, repository, root) = a_fleet();
    fleet.vcs().laps_on_the_forge(uploaded(TIP));
    fleet.writes().main_seen(repository.as_str(), TIP);
    fleet.writes().main_seen(repository.as_str(), "b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0");

    assert_eq!(fleet.test_map_laps_pulled(&repository, &root, TIP).await, Pulled::Superseded);
    assert!(fleet.vcs().asked_for_laps_of().is_empty());
    assert!(!edges_held(&fleet, &repository).await);
}

#[tokio::test]
async fn a_file_that_is_not_this_commits_laps_is_refused_and_keeps_nothing() {
    let (fleet, _home, repository, root) = a_fleet();
    fleet.vcs().laps_on_the_forge(uploaded("0123456789abcdef0123456789abcdef01234567"));
    let pulled = fleet.test_map_laps_pulled(&repository, &root, TIP).await;
    let Pulled::Unreadable(said) = pulled else { panic!("{pulled:?}") };
    assert!(said.contains("0123456789abcdef"), "{said}");

    fleet.vcs().laps_on_the_forge(Ok(LapsOnForge::Found(b"not json".to_vec())));
    assert!(matches!(fleet.test_map_laps_pulled(&repository, &root, TIP).await, Pulled::Unreadable(_)));
    assert!(!edges_held(&fleet, &repository).await);
}

/// Main's tip moving is what starts the asking: seen once, then moved, and the laps arrive without
/// anything else calling for them.
#[tokio::test]
async fn main_moving_on_fetches_the_laps_of_its_new_tip() {
    let (fleet, _home, repository, _root) = a_fleet();
    fleet.vcs().laps_on_the_forge(uploaded(TIP));
    fleet.writes().main_seen(repository.as_str(), "b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0");
    assert!(fleet.writes().main_seen(repository.as_str(), TIP), "main moved");

    fleet.clashes_checked().await;

    for _ in 0..100 {
        if edges_held(&fleet, &repository).await {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(edges_held(&fleet, &repository).await, "kept by a task of its own");
    assert_eq!(fleet.vcs().asked_for_laps_of(), vec![TIP.to_string()]);
}
