//! A workflow's `for_requests` line, as `list_workflows` serves it on
//! [`ipc::WorkflowSummary`]. Since 21.1.
//!
//! The approval screen shows the line beside the workflow the proposer picked,
//! so a person can read what that workflow promises before approving the Job.
//! **Absent where the definition declares none**: an empty sentence would be a
//! promise nobody wrote.

use crate::wire::workflow_summary;

/// One workflow, resolved against the fixture Manifest, saying what requests it
/// is for where `for_requests` is given.
fn summary_of(for_requests: Option<&str>) -> ipc::WorkflowSummary {
    let line = for_requests.map_or(String::new(), |line| format!("for_requests: {line}\n"));
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: refactor\nname: refactor\n{line}steps:\n  \
             - id: only\n    label: \"only\"\n    evidence: {{submitted: {{type: diff}}}}\n    \
             delivers: true\n    advance_gate: auto\n"
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
fn a_workflow_that_says_what_it_is_for_serves_the_line() {
    let summary = summary_of(Some("Restructuring code without changing what it does"));
    assert_eq!(
        summary.for_requests.as_deref(),
        Some("Restructuring code without changing what it does")
    );

    let wire = ipc::encode(&summary).expect("a summary encodes");
    assert!(
        wire.contains(r#""for_requests":"Restructuring code without changing what it does""#),
        "the line crosses the wire under its own key:\n{wire}"
    );
}

#[test]
fn a_workflow_that_says_nothing_serves_the_field_absent() {
    let summary = summary_of(None);
    assert_eq!(summary.for_requests, None);

    let wire = ipc::encode(&summary).expect("a summary encodes");
    assert!(
        !wire.contains("for_requests"),
        "absent, not null and not an empty sentence:\n{wire}"
    );
}
