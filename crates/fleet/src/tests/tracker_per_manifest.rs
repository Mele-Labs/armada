//! One Fleet serving two repositories whose Manifests chose different trackers: each is asked
//! with its own choice, and what one lists never reaches the other. What a provider's words mean
//! is `adapters`' to assert; under test here is that Fleet asks per repository.

use std::sync::Arc;

use api::Issues;
use ipc::ManifestId;
use testkit::{FakeHarness, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

const FIRST: &str = "01FIXTUREMANIFEST";
const SECOND: &str = "01SECONDMANIFEST";

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The fixture's repository, which names no tracker, and a second that names Jira.
fn two_repositories(home: &TempDir, trackers: Arc<FakeTrackers>) -> Hosted {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.trackers = trackers;
    let workflows = fittings.starting().workflows.clone();
    let fleet = Fleet::assembled(fittings);
    let root = home.path().join("second");
    let text = format!(
        "version: 1\nid: {SECOND}\ntracker:\n  provider: jira\n  site: acme\n  project: ABC\n"
    );
    let manifest = config::Manifest::parse(&root.join("armada.yml"), &text).expect("loads");
    fleet
        .repositories()
        .add(Located {
            root: root.to_string_lossy().to_string(),
            records_root: home.path().join("second-records").to_string_lossy().to_string(),
            set_up: Some(SetUp::of(manifest, workflows)),
        })
        .expect("served");
    fleet
}

#[tokio::test]
async fn each_repository_lists_from_the_tracker_its_manifest_chose() {
    let home = TempDir::new();
    let forge = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(1, "On the forge")]));
    let jira = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(2, "In Jira")]));
    let trackers = Arc::new(
        FakeTrackers::empty()
            .tracker(forge.clone())
            .tracker_named("jira", jira.clone()),
    );
    let fleet = two_repositories(&home, trackers.clone());

    let first = fleet
        .list_issues(ManifestId::carried(FIRST), true)
        .await
        .expect("listed");
    let second = fleet
        .list_issues(ManifestId::carried(SECOND), true)
        .await
        .expect("listed");

    assert_eq!(first.issues.len(), 1);
    assert_eq!(first.issues[0].title, "On the forge");
    assert_eq!(second.issues.len(), 1);
    assert_eq!(second.issues[0].title, "In Jira");
    assert_eq!((forge.listings(), jira.listings()), (1, 1));
}

#[tokio::test]
async fn the_choice_fleet_asks_with_is_the_block_as_the_manifest_wrote_it() {
    let home = TempDir::new();
    let trackers = Arc::new(FakeTrackers::empty());
    let fleet = two_repositories(&home, trackers.clone());

    fleet
        .list_issues(ManifestId::carried(FIRST), true)
        .await
        .expect("listed");
    fleet
        .list_issues(ManifestId::carried(SECOND), true)
        .await
        .expect("listed");

    let asked = trackers.choices_asked();
    assert_eq!(asked.len(), 2);
    assert_eq!(asked[0].provider(), None, "no block is the forge's own");
    assert!(asked[0].settings().is_empty());
    assert_eq!(asked[1].provider(), Some("jira"));
    let keys: Vec<(&str, &str)> = asked[1]
        .settings()
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    assert_eq!(keys, [("project", "ABC"), ("site", "acme")]);
}
