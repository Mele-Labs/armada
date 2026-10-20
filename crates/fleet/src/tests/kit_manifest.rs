//! A person's own `armada.yml` for a repository, as `list_manifests` reports it on
//! [`ipc::ManifestSummary`]: which names came from Kit and which repository ones they hide.

use std::path::Path;

use crate::wire::manifest_summary;

const REPOSITORY: &str = "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\n  \
                          lint:\n    run: make lint\n";

#[test]
fn the_summary_names_what_came_from_kit_and_what_it_hides() {
    let manifest = config::Manifest::with_kit(
        Path::new("/work/widgets/armada.yml"),
        REPOSITORY,
        Path::new("/kit/manifests/acme--widgets.yml"),
        "checks:\n  test:\n    run: make quick\n  lint:\n    run: make lint\n  e2e:\n    run: make e2e\n",
    )
    .expect("they merge");
    let kit = manifest_summary(&manifest, "/records").kit.expect("read from Kit");
    assert_eq!(kit.file, "/kit/manifests/acme--widgets.yml");
    assert!(!kit.whole);
    let names: Vec<(&str, &str, bool)> =
        kit.names.iter().map(|one| (one.kind.as_str(), one.name.as_str(), one.hides)).collect();
    assert_eq!(names, [("check", "test", true), ("check", "lint", false)]);
    assert_eq!(kit.not_read.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["e2e"]);
}

#[test]
fn a_repository_kit_holds_nothing_for_carries_no_kit_field() {
    let manifest = config::Manifest::parse(Path::new("/work/widgets/armada.yml"), REPOSITORY).expect("parses");
    let summary = manifest_summary(&manifest, "/records");
    assert_eq!(summary.kit, None);
    let wire = ipc::encode(&summary).expect("encodes");
    assert!(!wire.contains("\"kit\""), "absent rather than null");
}
