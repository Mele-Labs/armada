//! `setup.worktrees`: how many warm worktrees a repository leases out.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

const HEAD: &str = "version: 1\nid: a\n";

#[test]
fn a_manifest_that_says_nothing_leases_eight() {
    let manifest = parse(HEAD).expect("a bare manifest");
    assert_eq!(manifest.worktrees().get(), 8);
}

#[test]
fn a_manifest_value_overrides_the_default() {
    let manifest = parse(&format!("{HEAD}setup:\n  worktrees: 3\n")).expect("worktrees alone");
    assert_eq!(manifest.worktrees().get(), 3);
    assert!(manifest.prepared_by().is_empty());
}

#[test]
fn zero_worktrees_is_refused() {
    let refused = refusals(parse(&format!("{HEAD}setup:\n  worktrees: 0\n")));
    assert!(matches!(
        fault_at(&refused, "setup.worktrees"),
        Fault::WrongType { .. }
    ));
}

#[test]
fn a_repository_with_no_manifest_is_hosted_under_one_that_leases_eight_and_gates_nothing() {
    let manifest = Manifest::implicit(std::path::Path::new("/work/shop"), "shop-1a2b3c4d", None);
    assert_eq!(manifest.id().as_str(), "shop-1a2b3c4d");
    assert_eq!(manifest.worktrees().get(), 8);
    assert_eq!(manifest.base(), None);
    assert!(manifest.check_names().is_empty());
    assert!(manifest.command_names().is_empty());
    assert_eq!(manifest.path(), std::path::Path::new("/work/shop/armada.yml"));
    let on_master = Manifest::implicit(std::path::Path::new("/work/shop"), "shop-1a2b3c4d", Some("master"));
    assert_eq!(on_master.base(), Some("master"));
}
