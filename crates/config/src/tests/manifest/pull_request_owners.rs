//! `policy.pull_request_owners`, the switch for the central store.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

const HEAD: &str = "version: 1\nid: a\n";

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

#[test]
fn a_file_that_says_nothing_keeps_the_store_off() {
    assert!(!parse(HEAD).expect("a bare manifest").pull_request_owners());
    assert!(!parse(&format!("{HEAD}policy: {{}}\n"))
        .expect("an empty table")
        .pull_request_owners());
}

#[test]
fn true_turns_it_on_and_false_leaves_it_off() {
    let on = parse(&format!("{HEAD}policy:\n  pull_request_owners: true\n")).expect("on");
    assert!(on.pull_request_owners());
    let off = parse(&format!("{HEAD}policy:\n  pull_request_owners: false\n")).expect("off");
    assert!(!off.pull_request_owners());
}

#[test]
fn a_word_for_the_switch_and_a_key_beside_it_are_refused() {
    let refused = refusals(parse(&format!(
        "{HEAD}policy:\n  pull_request_owners: sometimes\n  audit: true\n"
    )));
    assert!(matches!(
        fault_at(&refused, "policy.pull_request_owners"),
        Fault::WrongType { .. }
    ));
    assert!(matches!(
        fault_at(&refused, "policy.audit"),
        Fault::Unknown { known } if known == &["pull_request_owners"]
    ));
}

#[test]
fn it_is_the_roots_to_say_and_a_workspace_that_says_it_is_refused() {
    let root = parse("version: 1\nid: root\n").expect("the root");
    let refused = refusals(Manifest::parse_workspace(
        &named("packages/a/armada.yml"),
        "packages/a",
        &format!("{HEAD}policy:\n  pull_request_owners: true\n"),
        &root,
    ));
    assert_eq!(
        fault_at(&refused, "policy.pull_request_owners"),
        &Fault::RootOnly
    );
}
