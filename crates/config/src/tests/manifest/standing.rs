//! `standing_rules:`, the file a repository names as what every change in it
//! carries.

use crate::error::{BadTarget, Fault};
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

#[test]
fn a_named_file_arrives_as_written() {
    let manifest = parse("version: 1\nid: a\nstanding_rules: docs/every-change.md\n")
        .expect("a path inside the checkout");
    assert_eq!(manifest.standing_rules(), Some("docs/every-change.md"));
}

#[test]
fn a_manifest_that_names_none_has_none() {
    let manifest = parse("version: 1\nid: a\n").expect("no key");
    assert_eq!(manifest.standing_rules(), None);
}

/// One file inside the checkout, by a deliverable's rules: what a brief
/// carries has to be a file the repository holds, and the same file for every
/// Job.
#[test]
fn a_path_that_cannot_name_one_file_in_the_checkout_is_refused_at_its_key() {
    for (written, why) in [
        ("/etc/motd", BadTarget::Absolute),
        ("../elsewhere/rules.md", BadTarget::Escapes),
        ("docs/*.md", BadTarget::Globbed),
        ("docs/", BadTarget::ADirectory),
    ] {
        let refused = refusals(parse(&format!(
            "version: 1\nid: a\nstanding_rules: \"{written}\"\n"
        )));
        assert_eq!(
            fault_at(&refused, "standing_rules"),
            &Fault::NotAnArtifactPath {
                value: written.to_string(),
                why,
            },
            "{written}"
        );
    }
}

#[test]
fn a_value_that_is_not_text_is_refused_at_its_key() {
    let refused = refusals(parse("version: 1\nid: a\nstanding_rules: [a, b]\n"));
    assert!(matches!(
        fault_at(&refused, "standing_rules"),
        Fault::WrongType { .. }
    ));
}
