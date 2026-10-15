//! The build record on the wire, and the facts a mod tells it from. `docs/concepts/session.md`, *The build record*.

use crate::{
    decode, encode, BuildPhase, BuildPiece, BuildPieceKind, BuildStop, BuildStopId, Instant, ReportedTaskStatus, SessionBuild, SessionFact, TimelineKind,
};

/// The mod's four facts, as `plugins/armada/hooks/fleet.ts` spells them.
#[test]
fn the_mods_plan_and_road_facts_are_read_as_it_spells_them() {
    let tasks: SessionFact = decode(
        "a fact",
        br#"{"kind":"tasks","tasks":[{"id":"1","text":"Read","status":"completed"},{"id":"2","text":"Test","status":"in_progress"},{"id":"3","text":"Ship","status":"pending"}]}"#,
    )
    .expect("a list");
    let SessionFact::Tasks { tasks } = tasks else { panic!("a list") };
    assert_eq!(
        tasks.iter().map(|one| one.status).collect::<Vec<_>>(),
        [ReportedTaskStatus::Completed, ReportedTaskStatus::InProgress, ReportedTaskStatus::Pending]
    );
    assert_eq!(decode::<SessionFact>("a fact", br#"{"kind":"plan_left"}"#).expect("left"), SessionFact::PlanLeft);
    assert_eq!(
        decode::<SessionFact>("a fact", br#"{"kind":"committed","branch":"armada/topic"}"#).expect("committed"),
        SessionFact::Committed { branch: Some("armada/topic".to_string()) }
    );
    assert_eq!(
        decode::<SessionFact>("a fact", br#"{"kind":"pushed"}"#).expect("pushed"),
        SessionFact::Pushed { branch: None },
        "a detached head names no branch"
    );
}

#[test]
fn a_build_round_trips_and_leaves_out_what_is_absent() {
    let build = SessionBuild {
        phase: BuildPhase::Road,
        pieces: vec![BuildPiece { id: "1".to_string(), text: "Read".to_string(), piece: BuildPieceKind::Solid, by: None }],
        stops: vec![
            BuildStop { id: BuildStopId::Pushed, at: Some(Instant::carried("2026-10-14T09:00:00.000Z")) },
            BuildStop { id: BuildStopId::AutoMerge, at: None },
        ],
        forecast: vec![BuildStopId::Pushed, BuildStopId::AutoMerge],
        landed_at: None,
        quiet_since: None,
        stalled: false,
        workers: vec!["Explore: Map the claims".to_string()],
    };
    let json = encode(&build).expect("plain data");
    assert!(json.contains(r#""phase":"road""#) && json.contains(r#""piece":"solid""#), "{json}");
    assert!(json.contains(r#"{"id":"auto_merge"}"#), "an unreached stop has no time, not a null: {json}");
    assert!(!json.contains("landed_at") && !json.contains("quiet_since") && !json.contains("null"), "{json}");
    assert!(!json.contains(r#""by""#), "a Session's own task names no worker: {json}");
    assert_eq!(decode::<SessionBuild>("a build", json.as_bytes()).expect("round-trips"), build);
}

#[test]
fn a_subagents_task_names_its_agent_and_an_old_one_reads_as_the_sessions_own() {
    let told = br#"{"kind":"tasks","tasks":[{"id":"2","text":"Map","status":"pending","by":"a1"},{"id":"1","text":"Read","status":"pending"}]}"#;
    let SessionFact::Tasks { tasks } = decode::<SessionFact>("a fact", told).expect("tasks") else { panic!("tasks") };
    assert_eq!(tasks.iter().map(|one| one.by.as_deref()).collect::<Vec<_>>(), [Some("a1"), None]);
    assert!(!encode(&tasks[1]).expect("plain data").contains("by"));
    let piece = br#"{"id":"1","text":"Read","piece":"ghost"}"#;
    assert_eq!(decode::<BuildPiece>("a piece", piece).expect("old row").by, None);
}

#[test]
fn the_road_kinds_are_spelt_as_the_timeline_keeps_them() {
    assert_eq!(TimelineKind::SessionWalked.as_wire(), "session.walked");
    assert_eq!(TimelineKind::from_wire("session.committed"), Some(TimelineKind::SessionCommitted));
    assert_eq!(TimelineKind::from_wire("session.pushed"), Some(TimelineKind::SessionPushed));
}
