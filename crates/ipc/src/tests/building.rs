//! What `get_fleet_build` and `change_fleet_build` must keep true: an empty
//! optional field leaves no key, a build outside the two is refused, and a
//! request that omits `adopt` does not adopt.

use crate::{
    decode, encode, BuildPosition, BuildSource, BuildStage, ChangeFleetBuild, FleetBuildReport,
    FleetBuildKept, FleetBuildReverting, FleetBuildSnapshotting, FleetBuildSwapping, Instant, KeptBuildPrevious,
    KeptBuildRunning, KeptSnapshot, LeftOutBranch, PickFleetBuildJob, PickedJob, SnapshotState, WaitingActKind,
    WaitingItem, WaitingSource,
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
        running: None,
        previous: None,
        carried: Vec::new(),
        reverting: false,
        picked: Vec::new(),
        snapshot: None,
        trying: Vec::new(),
        swapping: false,
        switcher: None,
        origin: Vec::new(),
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
        running: None,
        previous: None,
        carried: Vec::new(),
        reverting: false,
        picked: Vec::new(),
        snapshot: None,
        trying: Vec::new(),
        swapping: false,
        switcher: None,
        origin: Vec::new(),
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
fn the_kept_builds_cross_as_they_are_named_and_what_is_empty_leaves_no_key() {
    let report = FleetBuildReport {
        on: BuildSource::Main,
        commit: None,
        position: None,
        restarting: None,
        stage: None,
        failed: None,
        left_out: Vec::new(),
        running: Some(KeptBuildRunning { built_at: Instant::carried("2026-11-05T17:05:00Z"), commit: "e4f5a6b".into() }),
        previous: Some(KeptBuildPrevious {
            built_at: Instant::carried("2026-11-05T16:20:00Z"),
            commit: "a1b2c3d".into(),
            bridge_reopens: true,
            takes_out: vec!["Fleet core and mods".into()],
            brings_back: Vec::new(),
        }),
        carried: vec!["Kept builds".into()],
        reverting: true,
        picked: Vec::new(),
        snapshot: None,
        trying: Vec::new(),
        swapping: false,
        switcher: None,
        origin: Vec::new(),
    };
    let json = encode(&report).expect("a report is plain data");
    assert_eq!(
        json,
        concat!(
            r#"{"on":"main","running":{"built_at":"2026-11-05T17:05:00Z","commit":"e4f5a6b"},"#,
            r#""previous":{"built_at":"2026-11-05T16:20:00Z","commit":"a1b2c3d","bridge_reopens":true,"takes_out":["Fleet core and mods"]},"#,
            r#""carried":["Kept builds"],"reverting":true}"#
        )
    );
    assert_eq!(decode::<FleetBuildReport>("build", json.as_bytes()).expect("it round-trips"), report);
}

#[test]
fn a_previous_build_that_changes_nothing_carries_only_what_it_is() {
    let previous = KeptBuildPrevious {
        built_at: Instant::carried("2026-11-05T16:20:00Z"),
        commit: "a1b2c3d".into(),
        bridge_reopens: false,
        takes_out: Vec::new(),
        brings_back: Vec::new(),
    };
    assert_eq!(encode(&previous).unwrap(), r#"{"built_at":"2026-11-05T16:20:00Z","commit":"a1b2c3d"}"#);
}

#[test]
fn a_report_from_before_kept_builds_reads_with_none() {
    let report = decode::<FleetBuildReport>("build", br#"{"on":"main"}"#).expect("the new fields are optional");
    assert_eq!((report.running, report.previous, report.carried, report.reverting), (None, None, Vec::new(), false));
}

#[test]
fn the_answer_to_a_revert_names_the_commit_reverted_to() {
    assert_eq!(encode(&FleetBuildReverting { commit: "a1b2c3d".into() }).unwrap(), r#"{"commit":"a1b2c3d"}"#);
}

#[test]
fn the_snapshot_crosses_as_it_is_named_and_what_is_empty_leaves_no_key() {
    let report = FleetBuildReport {
        on: BuildSource::Main,
        commit: None,
        position: None,
        restarting: None,
        stage: None,
        failed: None,
        left_out: Vec::new(),
        running: None,
        previous: None,
        carried: Vec::new(),
        reverting: false,
        picked: vec![
            PickedJob { branch: "armada/a".into(), title: Some("Kept builds".into()) },
            PickedJob { branch: "armada/b".into(), title: None },
        ],
        snapshot: Some(KeptSnapshot {
            state: SnapshotState::Ready,
            stage: None,
            built_at: Some(Instant::carried("2026-11-06T01:10:00Z")),
            commit: Some("a1b2c3d".into()),
            jobs: vec!["Kept builds".into()],
            bridge_reopens: true,
            failed: None,
        }),
        trying: vec!["Fleet core and mods".into()],
        swapping: true,
        switcher: None,
        origin: Vec::new(),
    };
    let json = encode(&report).expect("a report is plain data");
    assert_eq!(
        json,
        concat!(
            r#"{"on":"main","picked":[{"branch":"armada/a","title":"Kept builds"},{"branch":"armada/b"}],"#,
            r#""snapshot":{"state":"ready","built_at":"2026-11-06T01:10:00Z","commit":"a1b2c3d","jobs":["Kept builds"],"bridge_reopens":true},"#,
            r#""trying":["Fleet core and mods"],"swapping":true}"#
        )
    );
    assert_eq!(decode::<FleetBuildReport>("build", json.as_bytes()).expect("it round-trips"), report);
}

#[test]
fn a_snapshot_state_crosses_as_a_snake_case_word_and_a_building_one_carries_its_stage() {
    for (state, word) in [(SnapshotState::Building, "building"), (SnapshotState::Ready, "ready"), (SnapshotState::Failed, "failed")] {
        assert_eq!(encode(&state).unwrap(), format!("\"{word}\""));
    }
    let building = KeptSnapshot {
        state: SnapshotState::Building,
        stage: Some(BuildStage::BuildingBridge),
        built_at: None,
        commit: None,
        jobs: Vec::new(),
        bridge_reopens: false,
        failed: None,
    };
    assert_eq!(encode(&building).unwrap(), r#"{"state":"building","stage":"building_bridge"}"#);
    let failed = KeptSnapshot { state: SnapshotState::Failed, stage: None, failed: Some("a conflict".into()), ..building };
    assert_eq!(encode(&failed).unwrap(), r#"{"state":"failed","failed":"a conflict"}"#);
}

#[test]
fn a_report_from_before_the_snapshot_reads_with_none() {
    let report = decode::<FleetBuildReport>("build", br#"{"on":"main"}"#).expect("the new fields are optional");
    assert_eq!((report.picked, report.snapshot, report.trying, report.swapping), (Vec::new(), None, Vec::new(), false));
}

#[test]
fn the_three_commands_cross_as_their_bodies_and_answers() {
    assert_eq!(
        encode(&PickFleetBuildJob { branch: Some("armada/a".into()), session_id: None, picked: true }).unwrap(),
        r#"{"branch":"armada/a","picked":true}"#
    );
    assert_eq!(
        encode(&PickFleetBuildJob { branch: None, session_id: Some("s-1".into()), picked: true }).unwrap(),
        r#"{"session_id":"s-1","picked":true}"#
    );
    assert!(decode::<PickFleetBuildJob>("pick", br#"{"branch":"armada/a"}"#).is_err(), "the flag is not defaulted");
    assert_eq!(encode(&FleetBuildSnapshotting { picked: vec!["armada/a".into()] }).unwrap(), r#"{"picked":["armada/a"]}"#);
    assert_eq!(encode(&FleetBuildSwapping { commit: "a1b2c3d".into() }).unwrap(), r#"{"commit":"a1b2c3d"}"#);
    assert_eq!(encode(&FleetBuildKept { told: vec!["Kept builds".into()] }).unwrap(), r#"{"told":["Kept builds"]}"#);
    assert_eq!(encode(&FleetBuildKept { told: Vec::new() }).unwrap(), r#"{"told":[]}"#);
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
            running: None,
            previous: None,
            carried: Vec::new(),
            reverting: false,
            picked: Vec::new(),
            snapshot: None,
            trying: Vec::new(),
            swapping: false,
            switcher: None,
            origin: Vec::new(),
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

#[test]
fn an_act_a_rule_asks_about_crosses_as_rule_act_with_its_two_options() {
    let item = serde_json::json!({
        "id": "rule_act:12",
        "text": "Move ARM-41 to In progress",
        "since": "2026-10-29T09:00:00Z",
        "source": "rule_act",
        "act": {"kind": "answer", "target": "rule_act:12"},
        "options": [{"label": "Do it"}, {"label": "Skip"}]
    });
    let read: WaitingItem = serde_json::from_value(item.clone()).unwrap();
    assert_eq!(read.source, WaitingSource::RuleAct);
    assert_eq!(read.options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Do it", "Skip"]);
    assert_eq!(serde_json::to_value(&read).unwrap(), item);
}
