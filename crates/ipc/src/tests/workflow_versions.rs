//! The versions of a workflow and what a copy was based on, on the wire: each type round-trips through JSON, what is absent is
//! absent, and a Fleet that says nothing of a base reads as having none.

use crate::{
    decode, encode, BasedOn, Instant, Lands, ManifestId, RevertWorkflow, VersionOrigin, WorkflowDefinition, WorkflowId, WorkflowSummary,
    WorkflowVersion, WorkflowVersions,
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

fn version(number: u32, origin: VersionOrigin) -> WorkflowVersion {
    WorkflowVersion {
        version: number,
        text: format!("{{v{number}}}"),
        hash: String::from("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"),
        at: Instant::carried("2026-11-13T09:00:00.000Z"),
        origin,
    }
}

#[test]
fn the_versions_are_newest_first_and_name_where_each_came_from_by_a_word() {
    let spelled = round_trips(&WorkflowVersions { versions: vec![version(2, VersionOrigin::Published), version(1, VersionOrigin::Found)] });
    assert!(spelled.contains(r#""origin":"published""#) && spelled.contains(r#""origin":"found""#), "{spelled}");
    assert!(spelled.find(r#""version":2"#) < spelled.find(r#""version":1"#), "{spelled}");
    assert_eq!(round_trips(&WorkflowVersions::default()), r#"{"versions":[]}"#, "where nothing was published the list is empty, not absent");
}

#[test]
fn a_revert_names_the_place_and_the_version_and_a_kits_names_no_project() {
    let spelled = round_trips(&RevertWorkflow { workflow_id: WorkflowId::carried("feature"), source: String::from("kit"), project: None, version: 3 });
    assert!(!spelled.contains("project"), "{spelled}");
    round_trips(&RevertWorkflow { workflow_id: WorkflowId::carried("feature"), source: String::from("project"), project: Some(String::from("storefront")), version: 1 });
}

fn summary() -> WorkflowSummary {
    WorkflowSummary {
        id: WorkflowId::carried("feature"),
        name: String::from("feature"),
        version: 2,
        steps: Vec::new(),
        manifest_id: ManifestId::carried("01FIXTUREMANIFEST"),
        source: String::from("kit"),
        project: None,
        for_requests: None,
        lands: Lands::PullRequest,
        file: String::from("/kit/workflows/feature.json"),
        overrides: Vec::new(),
        hides: None,
        based_on: None,
        armada_changed: false,
    }
}

#[test]
fn a_summary_says_what_its_copy_was_based_on_and_that_armada_moved_and_otherwise_says_neither() {
    let plain = round_trips(&summary());
    assert!(!plain.contains("based_on") && !plain.contains("armada_changed"), "a copy with no base sends neither key: {plain}");

    let moved = round_trips(&WorkflowSummary { based_on: Some(BasedOn { version: 1, hash: String::from("abc") }), armada_changed: true, ..summary() });
    assert!(moved.contains(r#""based_on":{"version":1,"hash":"abc"}"#) && moved.contains(r#""armada_changed":true"#), "{moved}");

    let held = round_trips(&WorkflowSummary { based_on: Some(BasedOn { version: 1, hash: String::from("abc") }), ..summary() });
    assert!(held.contains("based_on") && !held.contains("armada_changed"), "{held}");

    let older: WorkflowSummary = decode("a summary", plain.as_bytes()).expect("reads without either");
    assert_eq!((older.based_on, older.armada_changed), (None, false));
}

#[test]
fn a_definition_carries_its_base_only_where_it_has_one_and_the_base_is_a_source_of_its_own() {
    let plain = round_trips(&WorkflowDefinition { workflow_id: WorkflowId::carried("feature"), source: String::from("kit"), ..Default::default() });
    assert!(!plain.contains("based_on"), "{plain}");
    let base = round_trips(&WorkflowDefinition {
        workflow_id: WorkflowId::carried("feature"),
        source: String::from("base"),
        definition: String::from("{carried}"),
        based_on: Some(BasedOn { version: 1, hash: String::from("abc") }),
        ..Default::default()
    });
    assert!(base.contains(r#""source":"base""#) && base.contains(r#""based_on":{"version":1,"hash":"abc"}"#), "{base}");
}
