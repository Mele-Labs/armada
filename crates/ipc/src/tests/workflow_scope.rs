//! Where a workflow is kept, on the wire: the two places that were bare words still are, and a project's is
//! an object naming the project, so a Bridge that wrote the words before is read as it was.

use crate::{decode, encode, SaveWorkflow, WorkflowCopy, WorkflowScope, WorkflowSummary};

fn written(scope: &WorkflowScope) -> String {
    encode(scope).expect("a scope encodes")
}

fn read(text: &str) -> WorkflowScope {
    decode("a scope", text.as_bytes()).expect("a scope reads")
}

/// Each of the three spells as it did or as it is now, and reads back as what was written.
#[test]
fn a_scope_is_a_word_or_a_project_object_and_reads_back_either_way() {
    for (scope, spelled) in [
        (WorkflowScope::Repository, r#""repository""#),
        (WorkflowScope::Kit, r#""kit""#),
        (WorkflowScope::Project { id: String::from("storefront") }, r#"{"project":{"id":"storefront"}}"#),
    ] {
        assert_eq!(written(&scope), spelled);
        assert_eq!(read(spelled), scope);
    }
}

/// The words are the same words `source` carries, and only a project's names a project.
#[test]
fn a_scope_names_its_place_and_only_a_projects_names_a_project() {
    let project = WorkflowScope::Project { id: String::from("storefront") };
    assert_eq!([WorkflowScope::Repository.as_wire(), WorkflowScope::Kit.as_wire(), project.as_wire()], ["repository", "kit", "project"]);
    assert_eq!([WorkflowScope::Repository.project(), WorkflowScope::Kit.project(), project.project()], [None, None, Some("storefront")]);
}

/// A save for a project carries the object inside the body, and a body from before it reads as it did.
#[test]
fn a_save_for_a_project_and_a_save_from_before_projects_both_read() {
    let save: SaveWorkflow =
        decode("a save", br#"{"scope":{"project":{"id":"storefront"}},"definition":"{}","overwrite":true}"#).expect("it reads");
    assert_eq!(save.scope, WorkflowScope::Project { id: String::from("storefront") });
    let old: SaveWorkflow = decode("a save", br#"{"scope":"kit","definition":"{}"}"#).expect("it reads");
    assert_eq!(old.scope, WorkflowScope::Kit);
}

/// A scope that is neither a word Fleet has nor a project object is refused and not defaulted.
#[test]
fn a_scope_that_is_none_of_the_three_is_refused() {
    for bad in [r#""projects""#, r#"{"project":{}}"#, r#"{"project":"storefront"}"#, r#"{"team":{"id":"x"}}"#] {
        assert!(decode::<WorkflowScope>("a scope", bad.as_bytes()).is_err(), "{bad}");
    }
}

/// The project a copy or a workflow is for is absent, not null, where there is none, and a row from before
/// it reads without it.
#[test]
fn a_project_is_absent_where_there_is_none_and_a_row_from_before_reads() {
    let copy = WorkflowCopy { source: String::from("kit"), ..Default::default() };
    assert!(!encode(&copy).expect("encodes").contains("project"));
    let of_a_project = WorkflowCopy { source: String::from("project"), project: Some(String::from("storefront")), ..Default::default() };
    assert!(encode(&of_a_project).expect("encodes").contains(r#""project":"storefront""#));
    let old: WorkflowCopy = decode("a copy", br#"{"workflow_id":"bug","source":"kit"}"#).expect("a copy from before reads");
    assert_eq!(old.project, None);
    let summary: WorkflowSummary = decode(
        "a workflow",
        br#"{"id":"bug","name":"Bug","version":1,"steps":[],"manifest_id":"m","lands":"pull_request","source":"project","project":"storefront"}"#,
    )
    .expect("a workflow names its project");
    assert_eq!(summary.project.as_deref(), Some("storefront"));
}
