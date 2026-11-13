//! Moving running Jobs on the wire: each type round-trips through JSON, what is absent is absent, and a publish names the Jobs it left pinned.

use crate::{
    decode, encode, AffectedJob, AffectedJobs, ManifestId, MigrateJobs, MigrateResult, MigratedJobs, SessionId, WorkflowId, WorkflowPublished,
    WorkflowSaved, WorkflowScope,
};

fn round_trips<T>(value: &T) -> String
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let spelled = encode(value).expect("plain data");
    let back: T = decode("a value", spelled.as_bytes()).expect("reads");
    assert_eq!(&back, value, "{spelled}");
    spelled
}

fn job(id: &str) -> AffectedJob {
    AffectedJob {
        session_id: SessionId::carried(id),
        title: None,
        manifest_id: ManifestId::carried("01FIXTUREMANIFEST"),
        step_id: None,
        step_label: None,
        survives: true,
        pinned: false,
        edited: false,
        on_version: None,
    }
}

#[test]
fn an_affected_job_names_its_step_and_version_only_where_it_has_them() {
    let bare = round_trips(&job("s1"));
    for absent in ["title", "step_id", "step_label", "on_version"] {
        assert!(!bare.contains(absent), "{absent} is absent: {bare}");
    }
    assert!(bare.contains(r#""survives":true"#) && bare.contains(r#""pinned":false"#) && bare.contains(r#""edited":false"#), "{bare}");

    let held = round_trips(&AffectedJob {
        title: Some(String::from("Fix the reader")),
        step_id: Some(String::from("plan")),
        step_label: Some(String::from("Plan")),
        survives: false,
        pinned: true,
        on_version: Some(2),
        ..job("s2")
    });
    assert!(held.contains(r#""step_id":"plan""#) && held.contains(r#""on_version":2"#) && held.contains(r#""survives":false"#), "{held}");
    assert_eq!(round_trips(&AffectedJobs::default()), r#"{"jobs":[]}"#, "no Job is an empty list, not an absent one");
    round_trips(&AffectedJobs { jobs: vec![job("s1"), AffectedJob { edited: true, ..job("s2") }] });
}

#[test]
fn a_migrate_names_the_place_and_the_jobs_and_a_kits_names_no_project() {
    let kit = round_trips(&MigrateJobs {
        workflow_id: WorkflowId::carried("feature"),
        source: String::from("kit"),
        project: None,
        session_ids: vec![SessionId::carried("s1"), SessionId::carried("s2")],
    });
    assert!(!kit.contains("project") && kit.contains(r#""session_ids":["s1","s2"]"#), "{kit}");
    round_trips(&MigrateJobs {
        workflow_id: WorkflowId::carried("feature"),
        source: String::from("project"),
        project: Some(String::from("storefront")),
        session_ids: Vec::new(),
    });
}

#[test]
fn a_result_carries_where_the_job_stands_when_it_moved_and_a_code_when_it_did_not() {
    let moved = round_trips(&MigrateResult {
        session_id: SessionId::carried("s1"),
        moved: true,
        step_id: Some(String::from("walk")),
        step_label: Some(String::from("Walk")),
        code: None,
        message: None,
    });
    assert!(moved.contains(r#""moved":true"#) && !moved.contains("code") && !moved.contains("message"), "{moved}");
    let refused = round_trips(&MigrateResult {
        session_id: SessionId::carried("s2"),
        moved: false,
        step_id: None,
        step_label: None,
        code: Some(String::from("fleet.workflow_migrate_edited")),
        message: Some(String::from("it holds edits of its own")),
    });
    assert!(refused.contains(r#""code":"fleet.workflow_migrate_edited""#) && !refused.contains("step_id"), "{refused}");
    assert_eq!(round_trips(&MigratedJobs::default()), r#"{"results":[]}"#);
}

#[test]
fn a_publish_names_the_jobs_it_left_pinned() {
    let published = |pinned: Vec<SessionId>| WorkflowPublished {
        saved: WorkflowSaved {
            workflow_id: WorkflowId::carried("feature"),
            scope: WorkflowScope::Kit,
            file: String::from("/kit/workflows/feature.json"),
            replaced: true,
            runs_from: Some(String::from("kit")),
        },
        version: 3,
        pinned,
    };
    let spelled = round_trips(&published(vec![SessionId::carried("s1")]));
    assert!(spelled.contains(r#""pinned":["s1"]"#), "{spelled}");
    assert!(round_trips(&published(Vec::new())).contains(r#""pinned":[]"#), "a publish that pinned none says so");
}
