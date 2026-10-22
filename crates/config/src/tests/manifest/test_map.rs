//! `test_map.shared`, whether the test map goes through the central store.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

const HEAD: &str = "version: 1\nid: a\n";

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

#[test]
fn a_file_that_says_nothing_keeps_each_machines_map() {
    assert!(!parse(HEAD).expect("a bare manifest").test_map_shared());
    assert!(!parse(&format!("{HEAD}test_map: {{}}\n"))
        .expect("an empty table")
        .test_map_shared());
}

#[test]
fn true_shares_the_map() {
    let shared = parse(&format!("{HEAD}test_map:\n  shared: true\n")).expect("shared");
    assert!(shared.test_map_shared());
}

#[test]
fn false_keeps_each_machines_map() {
    let kept = parse(&format!("{HEAD}test_map:\n  shared: false\n")).expect("kept");
    assert!(!kept.test_map_shared());
}

#[test]
fn a_word_for_the_switch_and_a_key_beside_it_are_refused() {
    let refused = refusals(parse(&format!(
        "{HEAD}test_map:\n  shared: sometimes\n  window: 30\n"
    )));
    assert!(matches!(
        fault_at(&refused, "test_map.shared"),
        Fault::WrongType { .. }
    ));
    assert!(matches!(
        fault_at(&refused, "test_map.window"),
        Fault::Unknown { known } if known == &["shared"]
    ));
}

#[test]
fn it_is_the_roots_to_say_and_a_workspace_that_says_it_is_refused() {
    let root = parse("version: 1\nid: root\n").expect("the root");
    let refused = refusals(Manifest::parse_workspace(
        &named("packages/a/armada.yml"),
        "packages/a",
        &format!("{HEAD}test_map:\n  shared: true\n"),
        &root,
    ));
    assert_eq!(fault_at(&refused, "test_map"), &Fault::RootOnly);
}
