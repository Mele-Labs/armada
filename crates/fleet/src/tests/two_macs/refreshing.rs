//! A Refresh both Macs pressed on Studio is claimed, so Air shows it running while Studio merges and pushes.
//! The claim crosses the link at once and again on each stage, ends with the build, and keeps a second press
//! out on either Mac. The wrapper is a stand-in: the test writes the status file a real one would.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use ipc::{BuildSource, BuildStage, FleetBuildReport, MacLink, RefreshClaim, UpdateMac};

use super::job_moves::known_to;
use super::updating::{post, post_as, wrapper_on};
use super::{until, OneMac, Ticking};

struct Pair {
    studio: OneMac,
    air: OneMac,
}

impl Pair {
    fn status_of(mac: &OneMac) -> PathBuf {
        crate::runtime::support_dir(&mac.rig.fleet.host().home).join("restart-build.status")
    }

    /// What `mac`'s Fleet answers `get_fleet_build`.
    async fn report(mac: &OneMac) -> FleetBuildReport {
        let raw = mac.read("/fleet/build").await;
        ipc::decode("the build", raw.as_bytes()).expect("a build report")
    }

    async fn claim(mac: &OneMac) -> Option<RefreshClaim> {
        Pair::report(mac).await.refresh
    }

    /// Studio presses Refresh on both Macs. Answers the status and body.
    async fn pressed(&self) -> (StatusCode, String) {
        let asked = UpdateMac { mac: Some(self.air.id.clone()), this_mac_too: true, build: Some(BuildSource::Preview), ..UpdateMac::default() };
        post(&self.studio, "/macs/update", ipc::encode(&asked).expect("encodes")).await
    }
}

/// Studio and Air, paired and linked, each knowing the other by name, each serving a checkout with a wrapper.
async fn paired_pair() -> Pair {
    let clock = Arc::new(Ticking::from_nine());
    let (studio, air) = (OneMac::new("Studio", "01SMAC", &clock).await, OneMac::new("Air", "01AMAC", &clock).await);
    super::paired(&studio, &air).await;
    let (studio_id, air_id) = (studio.id.clone(), air.id.clone());
    until("Studio is linked to Air", || async { studio.mac(&air_id).await.and_then(|mac| mac.link) == Some(MacLink::Linked) }).await;
    until("Air is linked to Studio", || async { air.mac(&studio_id).await.and_then(|mac| mac.link) == Some(MacLink::Linked) }).await;
    // A Mac's Gateway and Fleet share one store; here each has its own, so what the Gateway knows is copied across.
    for (mac, name) in [(&studio, "Studio"), (&air, "Air")] {
        let this = store::ThisMac { id: mac.id.clone(), name: name.into(), colour: "teal".into(), minted_at: 1 };
        mac.rig.fleet.store().lock().await.this_mac_or(&this).expect("this Mac");
    }
    known_to(&studio, &air, "Air").await;
    known_to(&air, &studio, "Studio").await;
    wrapper_on(&studio);
    wrapper_on(&air);
    Pair { studio, air }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_refresh_both_macs_pressed_on_studio_shows_on_air_with_its_stage_and_ends_with_the_build() {
    let pair = paired_pair().await;
    let (status, answered) = pair.pressed().await;
    assert_eq!(status, StatusCode::ACCEPTED, "{answered}");

    // 1. Air shows who pressed it, at once, and Studio's own report says it started here.
    until("Air shows Studio's refresh", || async { Pair::claim(&pair.air).await.is_some() }).await;
    let on_air = Pair::claim(&pair.air).await.expect("Studio's claim");
    assert_eq!((on_air.mac.as_str(), on_air.name.as_str(), on_air.stage), (pair.studio.id.as_str(), "Studio", None));
    assert!(!on_air.at.as_str().is_empty(), "when it started");
    let on_studio = Pair::claim(&pair.studio).await.expect("Studio's own claim");
    assert_eq!((on_studio.mac.as_str(), on_studio.name.as_str()), (pair.studio.id.as_str(), "Studio"));
    assert_eq!(Pair::report(&pair.air).await.restarting, None, "Air is not restarting: it is told, and will be asked");

    // 2. The wrapper moves on, and Air is told again.
    std::fs::write(Pair::status_of(&pair.studio), "running\npreview\nbuilding_fleet\n").expect("a stage");
    until("Air follows the stage", || async { Pair::claim(&pair.air).await.is_some_and(|one| one.stage == Some(BuildStage::BuildingFleet)) }).await;
    assert_eq!(Pair::claim(&pair.studio).await.expect("own").stage, Some(BuildStage::BuildingFleet));

    // 3. The build ends: both Macs stop showing it.
    std::fs::remove_file(Pair::status_of(&pair.studio)).expect("the build ended");
    until("Air stops showing it", || async { Pair::claim(&pair.air).await.is_none() }).await;
    assert_eq!(Pair::claim(&pair.studio).await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_refresh_that_fails_stops_showing_on_air_and_keeps_its_failure_on_studio() {
    let pair = paired_pair().await;
    let (status, answered) = pair.pressed().await;
    assert_eq!(status, StatusCode::ACCEPTED, "{answered}");
    until("Air shows Studio's refresh", || async { Pair::claim(&pair.air).await.is_some() }).await;

    let commit = "0123456789abcdef0123456789abcdef01234567";
    let support = crate::runtime::support_dir(&pair.studio.rig.fleet.host().home);
    std::fs::write(support.join("restart-commit"), format!("{commit}\n")).expect("a commit");
    std::fs::write(Pair::status_of(&pair.studio), format!("failed\npreview\n{commit}\nthe merge stopped on a conflict\n")).expect("a failure");
    until("Air stops showing it", || async { Pair::claim(&pair.air).await.is_none() }).await;
    let on_studio = Pair::report(&pair.studio).await;
    assert_eq!(on_studio.refresh, None);
    assert_eq!(on_studio.failed.as_deref(), Some("the merge stopped on a conflict"), "failed as before");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_second_press_on_either_mac_is_refused_while_a_claim_stands_but_the_pressing_macs_own_ask_is_taken() {
    let pair = paired_pair().await;
    let (status, answered) = pair.pressed().await;
    assert_eq!(status, StatusCode::ACCEPTED, "{answered}");
    until("Air shows Studio's refresh", || async { Pair::claim(&pair.air).await.is_some() }).await;

    let busy = |status: StatusCode, body: &str, who: &str| {
        assert_eq!(status, StatusCode::CONFLICT, "{who}: {body}");
        assert!(body.contains("fleet.build_restarting"), "{who}: {body}");
        assert!(body.contains("restarting onto preview"), "{who}: {body}");
    };
    let again = pair.pressed().await;
    busy(again.0, &again.1, "Studio, Refresh both again");
    let preview = ipc::encode(&ipc::ChangeFleetBuild { build: BuildSource::Preview, adopt: false, clear: None }).expect("encodes");
    let on_studio = post(&pair.studio, "/fleet/build/change", preview.clone()).await;
    busy(on_studio.0, &on_studio.1, "Studio, plain Refresh");
    let on_air = post(&pair.air, "/fleet/build/change", preview).await;
    busy(on_air.0, &on_air.1, "Air, plain Refresh");
    let from_air = UpdateMac { mac: Some(pair.studio.id.clone()), this_mac_too: true, build: Some(BuildSource::Preview), ..UpdateMac::default() };
    let on_air = post(&pair.air, "/macs/update", ipc::encode(&from_air).expect("encodes")).await;
    busy(on_air.0, &on_air.1, "Air, Refresh both");

    // The build the claim announces is Studio's own ask of Air: it is the one call the claim lets through.
    let told = UpdateMac {
        build: Some(BuildSource::Preview),
        commit: Some("1234567".into()),
        reference: Some(format!("preview/{}", pair.studio.id)),
        ..UpdateMac::default()
    };
    let (status, answered) = post_as(&pair.air, "mac", "/macs/update", ipc::encode(&told).expect("encodes")).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{answered}");
    tokio::time::sleep(Duration::from_millis(100)).await;
}
