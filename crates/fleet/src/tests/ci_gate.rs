//! Which check a repository's pull requests are read by. What the forge makes of the name is
//! `adapters`' to assert; under test here is that Fleet asks with the Manifest's word, and with
//! `ci` where the Manifest has none.

use std::sync::Arc;
use std::time::Duration;

use config::Manifest;
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::fittings;
use crate::tests::merging::Fixture;
use crate::tests::tmp::TempDir;

fn a_fleet_whose_manifest_says(home: &TempDir, text: &str) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.starting().manifest =
        Manifest::parse(std::path::Path::new("armada.yml"), text).expect("a manifest");
    Fleet::assembled(fittings)
}

#[tokio::test]
async fn the_listing_and_the_watch_are_asked_with_the_manifests_gate() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_manifest_says(&home, "version: 1\nid: a\nci_gate: build\n");
    let served = fleet.repositories().served()[0].clone();

    fleet.notice_pulls(&served).await;
    let fleet = Arc::new(fleet);
    fleet.pulls_told(&served).await;

    let asked = fleet.vcs().main_ci.gates_asked();
    assert!(asked.len() >= 2, "both readings asked: {asked:?}");
    assert!(asked.iter().all(|gate| gate == "build"), "{asked:?}");
}

#[tokio::test]
async fn a_manifest_that_names_none_is_asked_with_ci() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_manifest_says(&home, "version: 1\nid: a\n");
    let served = fleet.repositories().served()[0].clone();

    fleet.notice_pulls(&served).await;

    assert_eq!(fleet.vcs().main_ci.gates_asked(), ["ci"]);
}
