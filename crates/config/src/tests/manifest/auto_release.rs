//! `setup.auto_release` and its grace window.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

const HEAD: &str = "version: 1\nid: a\n";

#[test]
fn a_manifest_that_says_nothing_releases_after_fifteen_minutes() {
    let released = parse(HEAD).expect("a bare manifest").auto_release();
    assert!(released.on);
    assert_eq!(released.grace_minutes.get(), 15);
}

#[test]
fn both_keys_are_read_and_neither_needs_requires() {
    let manifest = parse(&format!(
        "{HEAD}setup:\n  auto_release: false\n  auto_release_grace_minutes: 30\n"
    ))
    .expect("the two alone");
    assert!(!manifest.auto_release().on);
    assert_eq!(manifest.auto_release().grace_minutes.get(), 30);
}

#[test]
fn a_zero_window_and_a_word_for_the_switch_are_refused() {
    let refused = refusals(parse(&format!(
        "{HEAD}setup:\n  auto_release: sometimes\n  auto_release_grace_minutes: 0\n"
    )));
    assert!(matches!(
        fault_at(&refused, "setup.auto_release"),
        Fault::WrongType { .. }
    ));
    assert!(matches!(
        fault_at(&refused, "setup.auto_release_grace_minutes"),
        Fault::WrongType { .. }
    ));
}
