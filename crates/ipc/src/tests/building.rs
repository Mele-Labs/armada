//! What `get_fleet_build` and `change_fleet_build` must keep true: an empty
//! optional field leaves no key, a build outside the two is refused, and a
//! request that omits `adopt` does not adopt.

use crate::{
    decode, encode, BuildPosition, BuildSource, BuildStage, ChangeFleetBuild, FleetBuildReport,
    LeftOutBranch, WaitingActKind, WaitingItem, WaitingSource,
};

#[test]
fn a_build_with_nothing_to_say_carries_only_where_it_runs() {
    let report = FleetBuildReport {
        on: BuildSource::Main,
        commit: None,
        position: None,
        restarting: None,
        stage: None,
        failed: None,
        left_out: Vec::new(),
    };
    let json = encode(&report).expect("a report is plain data");
    assert_eq!(json, r#"{"on":"main"}"#);
    assert_eq!(
        decode::<FleetBuildReport>("build", json.as_bytes()).expect("it round-trips"),
        report
    );
}

#[test]
fn a_full_report_round_trips() {
    let report = FleetBuildReport {
        on: BuildSource::Preview,
        commit: Some("a4fcca362".into()),
        position: Some(BuildPosition { ahead: 4, behind: 0 }),
        restarting: Some(BuildSource::Main),
        stage: None,
        failed: Some("main cannot fast-forward to origin/main".into()),
        left_out: vec![
            LeftOutBranch { branch: "armada/a".into(), why: "conflict: apps/a.ts".into(), moved: false },
            LeftOutBranch { branch: "armada/b".into(), why: "conflict: apps/b.ts".into(), moved: true },
        ],
    };
    let json = encode(&report).expect("a report is plain data");
    assert!(json.contains(r#""on":"preview""#), "{json}");
    assert!(json.contains(r#""position":{"ahead":4,"behind":0}"#), "{json}");
    assert!(json.contains(r#""restarting":"main""#), "{json}");
    // `moved` is left out where false, as Bridge's `moved?` reads it.
    assert!(
        json.contains(
            r#""left_out":[{"branch":"armada/a","why":"conflict: apps/a.ts"},{"branch":"armada/b","why":"conflict: apps/b.ts","moved":true}]"#
        ),
        "{json}"
    );
    assert_eq!(
        decode::<FleetBuildReport>("build", json.as_bytes()).expect("it round-trips"),
        report
    );
}

#[test]
fn a_stage_crosses_as_a_snake_case_word_and_round_trips() {
    for (stage, word) in [
        (BuildStage::Merging, "merging"),
        (BuildStage::FetchingMain, "fetching_main"),
        (BuildStage::BuildingFleet, "building_fleet"),
        (BuildStage::BuildingBridge, "building_bridge"),
        (BuildStage::RestartingFleet, "restarting_fleet"),
        (BuildStage::ReopeningBridge, "reopening_bridge"),
        (BuildStage::Retrying, "retrying"),
    ] {
        let report = FleetBuildReport {
            on: BuildSource::Preview,
            commit: None,
            position: None,
            restarting: Some(BuildSource::Preview),
            stage: Some(stage),
            failed: None,
            left_out: Vec::new(),
        };
        let json = encode(&report).expect("a report is plain data");
        assert!(json.contains(&format!(r#""stage":"{word}""#)), "{json}");
        assert_eq!(decode::<FleetBuildReport>("build", json.as_bytes()).expect("round-trips"), report);
    }
}

#[test]
fn a_report_from_before_stages_reads_with_none() {
    let report = decode::<FleetBuildReport>("build", br#"{"on":"main","restarting":"main"}"#)
        .expect("stage is optional");
    assert_eq!(report.stage, None);
}

#[test]
fn a_stage_that_is_not_one_of_the_seven_is_refused() {
    let refused = decode::<FleetBuildReport>("build", br#"{"on":"main","stage":"compiling"}"#);
    assert!(refused.is_err());
}

#[test]
fn a_request_that_omits_adopt_does_not_adopt() {
    let asked = decode::<ChangeFleetBuild>("build to change to", br#"{"build":"preview"}"#)
        .expect("adopt is optional");
    assert_eq!(
        asked,
        ChangeFleetBuild {
            build: BuildSource::Preview,
            adopt: false
        }
    );
}

#[test]
fn a_build_that_is_neither_main_nor_the_preview_is_refused() {
    let refused = decode::<ChangeFleetBuild>("build to change to", br#"{"build":"feature"}"#);
    assert!(refused.is_err(), "only main and preview exist");
}

#[test]
fn an_offer_to_refresh_the_preview_crosses_as_preview_offer_with_its_command() {
    let item = serde_json::json!({
        "id": "preview:feat/billing-export@0123456789abcdef0123456789abcdef01234567",
        "text": "feat/billing-export moved since the preview left it out: merge conflict with feat/ledger",
        "since": "2026-10-28T09:00:00Z",
        "source": "preview_offer",
        "act": {"kind": "run", "target": "scripts/preview --restart"}
    });
    let read: WaitingItem = serde_json::from_value(item.clone()).unwrap();
    assert_eq!(read.source, WaitingSource::PreviewOffer);
    assert_eq!(read.act.as_ref().map(|act| (act.kind, act.target.as_str())), Some((WaitingActKind::Run, "scripts/preview --restart")));
    assert_eq!(serde_json::to_value(&read).unwrap(), item);
}
