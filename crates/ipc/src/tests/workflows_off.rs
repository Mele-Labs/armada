//! `workflows.off`: the workflows switched off for the machine and for single repositories. Read
//! leniently, so a hand edit gone wrong leaves every workflow on rather than none.

use crate::workflows_off::WorkflowsOff;

#[test]
fn text_that_is_absent_or_not_the_shape_reads_as_nothing_off() {
    for text in ["", "null", "not json", "[\"bug\"]", r#"{"machine":"bug"}"#, r#"{"repositories":{"a":"bug"}}"#] {
        let read = WorkflowsOff::read(text);
        assert_eq!(read, WorkflowsOff::default(), "{text:?}");
        assert!(!read.has("a", "bug"), "{text:?}");
    }
}

#[test]
fn both_lists_are_read_and_either_switches_a_workflow_off_for_its_place() {
    let read = WorkflowsOff::read(r#"{"machine":["bug"],"repositories":{"one":["feature"]}}"#);
    assert_eq!(read.machine, ["bug"]);
    assert_eq!(read.repositories.get("one").map(Vec::as_slice), Some(&["feature".to_string()][..]));
    assert!(read.has("one", "bug") && read.has("two", "bug"), "the machine's list is every repository's");
    assert!(read.has("one", "feature"), "a repository's own");
    assert!(!read.has("two", "feature"), "and no other repository's");
    assert!(!read.has("one", "other"));

    let only_one_list = WorkflowsOff::read(r#"{"machine":["bug"]}"#);
    assert!(only_one_list.has("any", "bug") && only_one_list.repositories.is_empty(), "a list left out is empty");
}

#[test]
fn a_projects_list_reads_after_the_repositorys_and_the_machines_and_each_switch_names_the_layer_that_holds_it_off() {
    use crate::ValueLayer;
    let held = WorkflowsOff::read(r#"{"machine":["bug"],"repositories":{"one":["feature","chore"]}}"#);
    let project = ["chore".to_string(), "bug".to_string(), "docs".to_string()];

    assert_eq!(held.off_from(Some("one"), &project, "docs"), Some(ValueLayer::Project), "the project's own");
    assert_eq!(held.off_from(Some("one"), &project, "feature"), Some(ValueLayer::Repository), "the repository's");
    assert_eq!(held.off_from(Some("one"), &project, "chore"), Some(ValueLayer::Repository), "off in the repository holds over the project's");
    assert_eq!(held.off_from(Some("one"), &project, "bug"), Some(ValueLayer::Machine), "off on the machine holds over both");
    assert_eq!(held.off_from(Some("one"), &project, "spike"), None, "named by none is on");
    assert_eq!(held.off_from(Some("two"), &project, "feature"), None, "another repository's list is not read");
    assert_eq!(held.off_from(None, &project, "chore"), Some(ValueLayer::Project), "no repository: the machine and the project");
    assert_eq!(held.off_from(Some("one"), &[], "docs"), None, "no project: nothing of one");
}
