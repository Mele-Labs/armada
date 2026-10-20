//! A repository marked Keep on this Mac: its run memory place is `this_mac`, another is refused
//! with the reason, and its tiles stay in Kit's folder on this Mac. `docs/concepts/kit.md`,
//! *Kept on this Mac*.

use api::{Commands, Queries};
use ipc::{FlightControl, FlightControlKind, FlightControls, KeptOnThisMac, RunMemoryPlace, RunPlace, SaveRepositoryFlightControls};
use testkit::FakeWorkProduct;

use crate::flight_controls::repository::FOLDER;
use crate::run_memory::kept_here::KEPT_HERE;
use crate::run_memory::places;
use crate::tests::tmp::TempDir;

fn synced(root: &str, folder: &std::path::Path) -> RunMemoryPlace {
    RunMemoryPlace {
        repository: root.to_string(),
        place: RunPlace::Synced,
        synced_folder: Some(folder.to_string_lossy().to_string()),
        cloud: None,
    }
}

fn kept(root: &str, kept: bool) -> KeptOnThisMac {
    KeptOnThisMac { repository: root.to_string(), kept }
}

fn a_tile() -> SaveRepositoryFlightControls {
    let tile = FlightControl {
        id: "specs".into(),
        name: None,
        glyph: None,
        key: None,
        hidden: false,
        text: Some("bin/rspec".into()),
        on_text: None,
        off_text: None,
        kind: Some(FlightControlKind::Shell),
    };
    SaveRepositoryFlightControls { repository: String::new(), controls: FlightControls { version: 1, job: vec![tile], deck: Vec::new() } }
}

#[tokio::test]
async fn a_repository_kept_on_this_mac_takes_this_mac_only_and_keeps_its_tiles_there() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    let elsewhere = TempDir::new();
    let folder = elsewhere.path().join("Drive");
    std::fs::create_dir_all(&folder).expect("a synced folder");

    assert_eq!(fleet.get_kept_on_this_mac(root.clone()).await.expect("read"), kept(&root, false), "absent a mark");
    fleet.set_run_memory_place(synced(&root, &folder)).await.expect("chosen");

    let marked = fleet.set_kept_on_this_mac(kept(&root, true)).await.expect("marked");
    assert_eq!(marked, kept(&root, true));
    assert_eq!(fleet.get_kept_on_this_mac(root.clone()).await.expect("read"), kept(&root, true));
    let place = fleet.get_run_memory_place(root.clone()).await.expect("read");
    assert_eq!(place.place, RunPlace::ThisMac, "marking moves the place");

    let refused = fleet.set_run_memory_place(synced(&root, &folder)).await.expect_err("refused");
    let api::Refusal::Unacceptable(error) = refused else { panic!("a 422") };
    assert_eq!(error.code, KEPT_HERE);
    assert!(error.message.contains("is kept on this Mac"), "{}", error.message);
    assert_eq!(fleet.get_run_memory_place(root.clone()).await.expect("read").place, RunPlace::ThisMac, "the place stays");

    let saved = fleet
        .repository_flight_controls_saved(SaveRepositoryFlightControls { repository: root.clone(), ..a_tile() })
        .await
        .expect("saved");
    assert_eq!(saved.place, RunPlace::ThisMac);
    assert!(places::file_for(&home.path().join("kit"), FOLDER, &root).exists(), "in Kit's folder on this Mac");
    assert!(!places::file_for(&folder, FOLDER, &root).exists(), "never in the synced folder");
}

#[tokio::test]
async fn clearing_the_mark_moves_nothing_and_offers_every_place_again() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    let elsewhere = TempDir::new();
    let folder = elsewhere.path().join("Drive");
    std::fs::create_dir_all(&folder).expect("a synced folder");

    fleet.set_kept_on_this_mac(kept(&root, true)).await.expect("marked");
    assert_eq!(fleet.set_kept_on_this_mac(kept(&root, false)).await.expect("cleared"), kept(&root, false));
    assert_eq!(fleet.get_run_memory_place(root.clone()).await.expect("read").place, RunPlace::ThisMac);
    let chosen = fleet.set_run_memory_place(synced(&root, &folder)).await.expect("chosen once the mark is cleared");
    assert_eq!(chosen.place, RunPlace::Synced);
}

/// The mark can be set after the choice's first check and before its write, during a network probe.
#[tokio::test]
async fn a_mark_set_after_the_first_check_still_refuses_the_write() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();

    fleet.refused_while_kept_here(&root, RunPlace::Cloud).await.expect("not yet marked");
    fleet.set_kept_on_this_mac(kept(&root, true)).await.expect("marked");

    let refused = fleet.place_recorded(&root, RunPlace::Cloud, None, Some("store")).await.expect_err("refused at the write");
    let api::Refusal::Unacceptable(error) = refused else { panic!("a 422") };
    assert_eq!(error.code, KEPT_HERE);
    assert_eq!(fleet.get_run_memory_place(root.clone()).await.expect("read").place, RunPlace::ThisMac, "the place stays");
    fleet.place_recorded(&root, RunPlace::ThisMac, None, None).await.expect("this_mac is always allowed");
}

#[tokio::test]
async fn a_root_fleet_does_not_serve_is_not_found() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let refused = fleet.set_kept_on_this_mac(kept("/work/elsewhere", true)).await.expect_err("refused");
    assert!(matches!(refused, api::Refusal::NotFound(_)), "{refused:?}");
}
