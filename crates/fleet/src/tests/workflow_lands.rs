//! What a workflow lands in, as `list_workflows` serves it on
//! [`ipc::WorkflowSummary`]. Bridge's Job view draws its Landing by it.
//!
//! **Fleet serves the word resolved**, so Bridge never reads `delivers` for it.

use crate::wire::workflow_summary;

/// One workflow, resolved against the fixture Manifest, with its one step
/// delivering or not and `lands` declared where given.
fn summary_of(delivers: bool, lands: Option<&str>) -> ipc::WorkflowSummary {
    let line = lands.map_or(String::new(), |word| format!("lands: {word}\n"));
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: landing\nname: landing\n{line}steps:\n  \
             - id: only\n    label: \"only\"\n    evidence: {{submitted: {{type: diff}}}}\n    \
             delivers: {delivers}\n    advance_gate: auto\n"
        ),
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let manifest = config::Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    let workflow = config::ResolvedWorkflow::resolve(&def, &manifest)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"));
    workflow_summary(&workflow, manifest.id(), &[])
}

#[test]
fn a_delivering_workflow_lands_a_pull_request_on_the_wire() {
    let summary = summary_of(true, None);
    assert_eq!(summary.lands, ipc::Lands::PullRequest);
    let wire = ipc::encode(&summary).expect("a summary encodes");
    assert!(wire.contains(r#""lands":"pull_request""#), "{wire}");
}

#[test]
fn a_workflow_that_delivers_nothing_lands_a_document_on_the_wire() {
    let summary = summary_of(false, None);
    assert_eq!(summary.lands, ipc::Lands::Document);
    let wire = ipc::encode(&summary).expect("a summary encodes");
    assert!(wire.contains(r#""lands":"document""#), "{wire}");
}

#[test]
fn a_declared_landing_crosses_the_wire_as_declared() {
    let summary = summary_of(true, Some("report"));
    assert_eq!(summary.lands, ipc::Lands::Report);
    let wire = ipc::encode(&summary).expect("a summary encodes");
    assert!(wire.contains(r#""lands":"report""#), "{wire}");
}
