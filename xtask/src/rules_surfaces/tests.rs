//! The rule's negative cases, on manifests written here.

use super::*;

fn faults(text: &str) -> Vec<String> {
    let mut report = Report::new("test");
    check_manifest("packages/surfaces/jobs", text, &mut report);
    report
        .findings
        .iter()
        .map(|f| match f {
            crate::Finding::Fail(w) | crate::Finding::Warn(w) => w.clone(),
        })
        .collect()
}

const WHOLE: &str =
    "version: 1\ndepends_on: [\"packages/screens/**\", \"pnpm-lock.yaml\"]\nchecks:\n  \
                     test:\n    run: pnpm test\n";

#[test]
fn a_whole_manifest_passes() {
    assert!(faults(WHOLE).is_empty());
}

#[test]
fn a_block_list_of_dependencies_passes() {
    let text = "depends_on:\n  - \"packages/screens/**\"\nchecks:\n  test:\n    run: x\n";
    assert!(faults(text).is_empty());
}

#[test]
fn no_checks_fails() {
    let text = "depends_on: [\"a/**\"]\nchecks: {}\n";
    assert_eq!(faults(text).len(), 1);
}

#[test]
fn no_depends_on_fails() {
    let text = "checks:\n  test:\n    run: x\n";
    assert_eq!(faults(text).len(), 1);
}

#[test]
fn an_empty_depends_on_fails() {
    let text = "depends_on: []\nchecks:\n  test:\n    run: x\n";
    assert_eq!(faults(text).len(), 1);
}

#[test]
fn a_commented_check_does_not_count() {
    let text = "depends_on: [\"a/**\"]\nchecks:\n  # test:\n";
    assert_eq!(faults(text).len(), 1);
}

#[test]
fn no_surfaces_is_green() {
    let report = every_surface_is_a_whole_package(Path::new("/nonexistent-armada-root"));
    assert!(!report.failed());
}
