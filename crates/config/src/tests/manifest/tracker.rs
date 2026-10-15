//! `tracker:`, which tracker holds a repository's issues. The provider is the
//! one key this crate knows; every other key is carried for the adapter.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

#[test]
fn a_manifest_that_names_none_has_none() {
    let manifest = parse("version: 1\nid: a\n").expect("no key");
    assert_eq!(manifest.tracker(), None);
}

#[test]
fn a_block_arrives_with_its_provider_apart_from_every_other_key() {
    let manifest = parse(
        "version: 1\nid: a\ntracker:\n  provider: jira\n  site: acme\n  project: ABC\n  jql: labels = web\n",
    )
    .expect("a block");
    let block = manifest.tracker().expect("a block");
    assert_eq!(block.provider(), Some("jira"));
    let keys: Vec<(&str, &str)> = block
        .keys()
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    assert_eq!(
        keys,
        [("jql", "labels = web"), ("project", "ABC"), ("site", "acme")]
    );
}

#[test]
fn a_block_with_no_provider_is_still_a_block() {
    let manifest = parse("version: 1\nid: a\ntracker:\n  project: ABC\n").expect("a block");
    let block = manifest.tracker().expect("a block");
    assert_eq!(block.provider(), None);
    assert_eq!(block.keys().len(), 1);
}

#[test]
fn a_provider_this_crate_has_no_word_for_is_not_refused_here() {
    let manifest = parse("version: 1\nid: a\ntracker:\n  provider: somewhere-else\n")
        .expect("the composition root answers for providers");
    assert_eq!(
        manifest.tracker().and_then(|block| block.provider()),
        Some("somewhere-else")
    );
}

#[test]
fn a_key_that_is_not_text_is_refused_at_its_own_path_and_the_rest_still_read() {
    let refused = refusals(parse(
        "version: 1\nid: a\ntracker:\n  provider: jira\n  project: [A, B]\n  site: \"\"\n",
    ));
    assert!(matches!(
        fault_at(&refused, "tracker.project"),
        Fault::WrongType { .. }
    ));
    assert_eq!(fault_at(&refused, "tracker.site"), &Fault::Empty);
}

#[test]
fn a_block_that_is_not_a_map_is_refused_at_the_section() {
    let refused = refusals(parse("version: 1\nid: a\ntracker: jira\n"));
    assert!(matches!(
        fault_at(&refused, "tracker"),
        Fault::WrongType { .. }
    ));
}
