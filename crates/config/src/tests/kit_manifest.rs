//! A person's own `armada.yml` for a repository, kept in Kit, over the repository's.

use std::path::Path;

use crate::manifest::{KitKind, KitName, Manifest};

const REPOSITORY: &str = "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\n  \
                          lint:\n    run: make lint\ncommands:\n  dev:\n    run: make dev\n";

fn file() -> &'static Path {
    Path::new("/work/widgets/armada.yml")
}

fn kit() -> &'static Path {
    Path::new("/home/.armada/manifests/acme--widgets.yml")
}

fn named(kind: KitKind, name: &str, hides: bool) -> KitName {
    KitName { kind, name: name.to_string(), hides }
}

#[test]
fn kits_copy_is_the_manifest_where_the_repository_has_none() {
    let manifest = Manifest::from_kit_alone(file(), kit(), REPOSITORY).expect("Kit's parses");
    assert_eq!(manifest.path(), file(), "the repository is found from the Manifest, not Kit's folder");
    assert_eq!(manifest.check("test").map(|check| check.run().to_string()).as_deref(), Some("make test"));
    let from = manifest.from_kit().expect("read from Kit");
    assert!(from.whole);
    assert_eq!(from.file, kit());
    assert_eq!(
        from.names,
        [
            named(KitKind::Check, "test", false),
            named(KitKind::Check, "lint", false),
            named(KitKind::Command, "dev", false)
        ]
    );
}

#[test]
fn kits_checks_and_commands_replace_the_repositorys_by_name_and_the_rest_stands() {
    let mine = "checks:\n  test:\n    run: cargo nextest run\n  lint:\n    run: make lint\n  \
                e2e:\n    run: make e2e\ncommands:\n  dev:\n    run: pnpm dev\n";
    let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("they merge");
    let run = |name: &str| manifest.check(name).map(|check| check.run().to_string());
    assert_eq!(run("test").as_deref(), Some("cargo nextest run"), "Kit's wins");
    assert_eq!(run("lint").as_deref(), Some("make lint"));
    assert_eq!(run("e2e"), None, "a name the repository does not declare is not read");
    assert_eq!(manifest.id().as_str(), "01FIXTUREMANIFEST", "the rest of the repository's stands");
    assert_eq!(manifest.path(), file());
    let from = manifest.from_kit().expect("read from Kit");
    assert!(!from.whole && from.left_out.is_none());
    assert_eq!(
        from.names,
        [
            named(KitKind::Check, "test", true),
            named(KitKind::Check, "lint", false),
            named(KitKind::Command, "dev", true)
        ],
        "a copy that is the same hides nothing"
    );
    assert_eq!(from.not_read, [named(KitKind::Check, "e2e", false)]);
}

#[test]
fn a_kit_copy_that_will_not_merge_is_left_out_and_the_repositorys_stands() {
    for mine in ["checks: [unclosed", "checks:\n  test:\n    run: make test\n    flavour: x\n"] {
        let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("the repository's stands");
        assert_eq!(manifest.check("test").map(|check| check.run().to_string()).as_deref(), Some("make test"));
        let said = manifest.from_kit().and_then(|from| from.left_out.clone()).expect("named");
        assert!(said.starts_with("/home/.armada/manifests/acme--widgets.yml was left out"), "{said}");
    }
}

#[test]
fn a_repository_that_will_not_load_is_refused_whatever_kit_holds() {
    let refused = Manifest::with_kit(file(), "version: 1\n", kit(), "checks: {}\n").expect_err("no id");
    assert_eq!(refused.path(), file());
}
