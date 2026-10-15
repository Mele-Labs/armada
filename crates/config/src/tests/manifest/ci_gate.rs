//! `ci_gate:`, the check a pull request's `ci` is read from.

use crate::error::Fault;
use crate::manifest::{Manifest, DEFAULT_CI_GATE};
use crate::tests::{fault_at, named, refusals};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

#[test]
fn a_manifest_that_names_none_reads_the_check_called_ci() {
    let manifest = parse("version: 1\nid: a\n").expect("no key");
    assert_eq!(manifest.ci_gate(), DEFAULT_CI_GATE);
    assert_eq!(manifest.ci_gate(), "ci");
}

#[test]
fn a_named_check_arrives_as_written() {
    let manifest = parse("version: 1\nid: a\nci_gate: build / test\n").expect("a name");
    assert_eq!(manifest.ci_gate(), "build / test");
}

#[test]
fn an_empty_name_and_a_name_that_is_not_text_are_refused_at_the_key() {
    let empty = refusals(parse("version: 1\nid: a\nci_gate: \"\"\n"));
    assert_eq!(fault_at(&empty, "ci_gate"), &Fault::Empty);
    let listed = refusals(parse("version: 1\nid: a\nci_gate: [ci]\n"));
    assert!(matches!(
        fault_at(&listed, "ci_gate"),
        Fault::WrongType { .. }
    ));
}
