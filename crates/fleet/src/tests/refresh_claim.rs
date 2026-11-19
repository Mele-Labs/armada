//! A Refresh of both Macs, claimed: what a Mac holds of the other's, what it says of its own, and when each
//! stops standing. Time is handed in, so twenty minutes is sat through in no test.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

use ipc::{BuildSource, BuildStage, FleetBuildReport, Instant, RefreshClaim};

use crate::building::{Building, NotReverted, NotSwapped};
use crate::tests::tmp::TempDir;

const STUDIO: &str = "53747564696f";
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// A checkout holding a wrapper, and a support folder.
fn rig() -> (TempDir, TempDir) {
    let checkout = TempDir::new();
    let scripts = checkout.path().join("scripts");
    fs::create_dir_all(&scripts).expect("scripts");
    let wrapper = scripts.join("restart-build");
    fs::write(&wrapper, "#!/bin/sh\n").expect("a wrapper");
    Command::new("chmod").arg("+x").arg(&wrapper).status().expect("chmod");
    (checkout, TempDir::new())
}

fn claim(stage: Option<BuildStage>) -> RefreshClaim {
    RefreshClaim { mac: STUDIO.into(), name: "Studio".into(), stage, at: Instant::carried("2026-11-19T09:00:00.000Z") }
}

async fn report(building: &Building, checkout: &Path, support: &Path, now: SystemTime) -> FleetBuildReport {
    building.report(Some(checkout), support, now).await.expect("a build")
}

fn minutes(count: u64) -> Duration {
    Duration::from_secs(count * 60)
}

#[tokio::test]
async fn a_refresh_another_mac_told_of_is_reported_with_its_stage_and_lapses_after_twenty_quiet_minutes() {
    let (checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();
    assert_eq!(report(&building, checkout.path(), support.path(), now).await.refresh, None, "nothing claimed yet");

    building.refresh_heard(STUDIO, claim(Some(BuildStage::Merging)), None, now);
    let shown = report(&building, checkout.path(), support.path(), now + minutes(19)).await;
    assert_eq!(shown.refresh, Some(claim(Some(BuildStage::Merging))), "the Mac, its name, the stage and when");
    assert_eq!(shown.restarting, None, "this Mac is not restarting: the claim is additional");

    let lapsed = report(&building, checkout.path(), support.path(), now + minutes(21)).await;
    assert_eq!(lapsed.refresh, None, "twenty quiet minutes on, the Mac that claimed it has gone");
}

#[tokio::test]
async fn a_claim_told_again_takes_the_new_stage_and_starts_the_quiet_time_over() {
    let (checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();
    building.refresh_heard(STUDIO, claim(Some(BuildStage::Merging)), None, now);
    building.refresh_heard(STUDIO, claim(Some(BuildStage::BuildingFleet)), None, now + minutes(15));

    let later = report(&building, checkout.path(), support.path(), now + minutes(30)).await;
    assert_eq!(later.refresh, Some(claim(Some(BuildStage::BuildingFleet))), "fifteen minutes after the last word");
    let gone = report(&building, checkout.path(), support.path(), now + minutes(36)).await;
    assert_eq!(gone.refresh, None);
}

#[tokio::test]
async fn a_claim_its_mac_ended_is_gone_at_once() {
    let (checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();
    building.refresh_heard(STUDIO, claim(None), None, now);
    building.refresh_dropped(STUDIO);
    assert_eq!(report(&building, checkout.path(), support.path(), now).await.refresh, None);
}

#[tokio::test]
async fn a_claim_of_a_mac_that_has_rebuilt_since_it_told_is_forgotten() {
    let (checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();
    building.refresh_heard(STUDIO, claim(None), Some("1111111".into()), now);
    building.refresh_forget_rebuilt(&[(STUDIO.into(), Some("1111111".into()))]);
    assert!(report(&building, checkout.path(), support.path(), now).await.refresh.is_some(), "the same build: still refreshing");
    building.refresh_forget_rebuilt(&[(STUDIO.into(), Some("2222222".into()))]);
    assert_eq!(report(&building, checkout.path(), support.path(), now).await.refresh, None, "it came back on the new build");
}

#[tokio::test]
async fn the_pressing_macs_claim_follows_its_wrappers_stage_and_ends_with_it() {
    let (checkout, support) = rig();
    let building = Building::default();
    let status = support.path().join("restart-build.status");
    let now = SystemTime::now();
    fs::write(&status, "running\npreview\n").unwrap();
    building.refresh_pressed(STUDIO, "Studio", now);

    let started = report(&building, checkout.path(), support.path(), now).await;
    let own = started.refresh.expect("its own claim, mac = itself");
    assert_eq!((own.mac.as_str(), own.name.as_str(), own.stage), (STUDIO, "Studio", None));
    assert_eq!(started.restarting, Some(BuildSource::Preview), "`restarting` works as it did");

    fs::write(&status, "running\npreview\nbuilding_fleet\n").unwrap();
    let building_fleet = report(&building, checkout.path(), support.path(), now).await;
    assert_eq!(building_fleet.refresh.map(|one| one.stage), Some(Some(BuildStage::BuildingFleet)));
    assert_eq!(building_fleet.stage, Some(BuildStage::BuildingFleet));

    fs::remove_file(&status).unwrap();
    assert_eq!(report(&building, checkout.path(), support.path(), now).await.refresh, None, "the build ended");
}

#[tokio::test]
async fn a_failed_refresh_ends_the_claim_and_keeps_its_failure_text() {
    let (checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();
    fs::write(support.path().join("restart-build.status"), "running\npreview\nmerging\n").unwrap();
    building.refresh_pressed(STUDIO, "Studio", now);
    assert!(report(&building, checkout.path(), support.path(), now).await.refresh.is_some());

    fs::write(support.path().join("restart-commit"), format!("{SHA}\n")).unwrap();
    fs::write(support.path().join("restart-build.status"), format!("failed\npreview\n{SHA}\nthe merge stopped on a conflict\n")).unwrap();
    let failed = report(&building, checkout.path(), support.path(), now).await;
    assert_eq!(failed.refresh, None, "a failed build is not a refresh under way");
    assert_eq!(failed.failed.as_deref(), Some("the merge stopped on a conflict"), "`failed` as today");
}

#[tokio::test]
async fn a_wrapper_that_stopped_writing_for_twenty_minutes_ends_the_claim() {
    let (checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();
    fs::write(support.path().join("restart-build.status"), "running\npreview\nmerging\n").unwrap();
    building.refresh_pressed(STUDIO, "Studio", now);
    let later = report(&building, checkout.path(), support.path(), now + minutes(21)).await;
    assert_eq!(later.refresh, None, "twenty minutes on, nothing is working");
}

#[tokio::test]
async fn a_revert_and_a_swap_are_refused_while_another_macs_refresh_stands() {
    let (_checkout, support) = rig();
    let building = Building::default();
    let now = SystemTime::now();
    building.refresh_heard(STUDIO, claim(None), None, now);

    let reverted = building.revert(support.path(), "label", now);
    assert_eq!(reverted.err(), Some(NotReverted::Restarting(BuildSource::Preview)));
    let swapped = building.swap(support.path(), "label", now);
    assert_eq!(swapped.err(), Some(NotSwapped::Restarting(BuildSource::Preview)));
}
