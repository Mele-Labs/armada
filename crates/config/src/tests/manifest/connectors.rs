//! `connectors`, whether a hosted Job reaches the person's account connectors.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

const HEAD: &str = "version: 1\nid: a\n";

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

#[test]
fn a_file_that_says_nothing_keeps_the_connectors() {
    assert!(parse(HEAD).expect("a bare manifest").connectors());
    assert!(parse(&format!("{HEAD}connectors: true\n"))
        .expect("on")
        .connectors());
}

#[test]
fn false_turns_the_connectors_off() {
    let off = parse(&format!("{HEAD}connectors: false\n")).expect("off");
    assert!(!off.connectors());
}

#[test]
fn a_word_for_the_connectors_switch_is_refused() {
    let refused = refusals(parse(&format!("{HEAD}connectors: sometimes\n")));
    assert!(matches!(
        fault_at(&refused, "connectors"),
        Fault::WrongType { .. }
    ));
}

#[test]
fn the_connectors_are_the_roots_to_say_and_a_workspace_that_says_so_is_refused() {
    let root = parse("version: 1\nid: root\n").expect("the root");
    let refused = refusals(Manifest::parse_workspace(
        &named("packages/a/armada.yml"),
        "packages/a",
        &format!("{HEAD}connectors: false\n"),
        &root,
    ));
    assert_eq!(fault_at(&refused, "connectors"), &Fault::RootOnly);
}
