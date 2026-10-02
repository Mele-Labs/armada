//! `merge_by:`, how Fleet lands a Job's work once somebody has said it may.

use crate::error::Fault;
use crate::manifest::{Manifest, MergeBy};
use crate::tests::{fault_at, named, refusals};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

/// **Absent is `forge`**, so a repository written before the key existed
/// lands exactly as it did.
#[test]
fn a_file_that_says_nothing_merges_through_the_forge() {
    let manifest = parse("version: 1\nid: a\n").expect("the key is optional");
    assert_eq!(manifest.merge_by(), MergeBy::Forge);
}

#[test]
fn both_values_are_read_off_the_file() {
    for (written, read) in [("forge", MergeBy::Forge), ("push", MergeBy::Push)] {
        let manifest = parse(&format!("version: 1\nid: a\nmerge_by: {written}\n"))
            .expect("a value the key has");
        assert_eq!(manifest.merge_by(), read);
        assert_eq!(read.as_written(), written);
    }
}

/// A mistyped value lands nothing by a road nobody chose.
#[test]
fn a_word_the_key_has_no_value_for_is_refused_and_names_both() {
    let refused = refusals(parse("version: 1\nid: a\nmerge_by: rebase\n"));
    let Fault::NotInTheSchema { legal, .. } = fault_at(&refused, "merge_by") else {
        panic!("`rebase` is not a way this lands work");
    };
    assert_eq!(*legal, ["forge", "push"]);
}
