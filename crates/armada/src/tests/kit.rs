//! Kit's Workflows, as `Setup::at` reads them from Kit's home. #425.
//!
//! Beside `setup.rs` rather than in it: that file is about a repository's own
//! files, and these are about the one directory that is not the repository's.

use config::WorkflowSource;

use crate::setup::{kit, Setup, KIT_HOME, KIT_WORKFLOWS};
use crate::tests::setup::{a_repository, a_workflow, bug, roster, sources};
use crate::tests::TempDir;

/// **One file replaces a carried definition by id**, from Kit or from the
/// repository, and Kit's replaces the repository's.
#[test]
fn kit_replaces_what_armada_carries_and_what_the_repository_holds() {
    let dir = a_repository();
    let kit = TempDir::new();
    kit.write(&format!("{KIT_WORKFLOWS}/bug.yml"), &a_workflow("bug"));
    kit.write(
        &format!("{KIT_WORKFLOWS}/hotfix.yml"),
        &a_workflow("hotfix"),
    );
    dir.write(".armada/workflows/hotfix.yml", &a_workflow("hotfix"));

    let setup = Setup::at(dir.path(), kit.path(), &roster()).expect("three places merge");
    let held = sources(&setup);
    for expected in [
        ("bug", WorkflowSource::Kit),
        ("feature", WorkflowSource::Armada),
        ("hotfix", WorkflowSource::Kit),
    ] {
        assert!(held.contains(&expected), "{expected:?} in {held:?}");
    }
    assert_eq!(
        bug(&setup).steps().len(),
        1,
        "Kit's one-step bug, not Armada's"
    );
}

/// **A Kit definition that does not fit is left out and named, and Fleet
/// starts anyway** — one that will not parse, and one gating on a Check this
/// repository lacks. The owner's decision. Armada's `bug` answers for the id.
#[test]
fn a_kit_definition_that_does_not_fit_is_left_out_and_named() {
    let dir = a_repository();
    let kit = TempDir::new();
    kit.write(
        &format!("{KIT_WORKFLOWS}/broken.yml"),
        "version: 1\nworkflow_id: broken\n",
    );
    kit.write(
        &format!("{KIT_WORKFLOWS}/bug.yml"),
        "version: 1\nworkflow_id: bug\nname: bug\nsteps:\n  - id: only\n    \
         label: Only\n    evidence: {submitted: {type: diff}}\n    delivers: true\n    \
         advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: build }\n",
    );

    let setup = Setup::at(dir.path(), kit.path(), &roster()).expect("starts anyway");
    assert_eq!(bug(&setup).source(), WorkflowSource::Armada);
    let said: Vec<String> = setup.left_out().iter().map(ToString::to_string).collect();
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(
        said.iter().any(|line| line.contains("broken.yml")),
        "{said:?}"
    );
    assert!(
        said.iter().any(
            |line| line.starts_with("Kit's `bug` was left out, because ")
                && line.contains("build")
                && line.ends_with("; Armada's `bug` is used instead")
        ),
        "a person who customised `bug` is told they are not running it: {said:?}"
    );
}

/// **Two Kit files naming one id are both left out, both named, and Armada's
/// fills in** — one bad Kit never stops Fleet for every repository. The
/// repository's own duplicates stay refused; `setup.rs` holds that.
#[test]
fn two_kit_files_naming_one_id_are_left_out_and_armadas_fills_in() {
    let dir = a_repository();
    let kit = TempDir::new();
    kit.write(&format!("{KIT_WORKFLOWS}/mine.yml"), &a_workflow("bug"));
    kit.write(
        &format!("{KIT_WORKFLOWS}/also-mine.yml"),
        &a_workflow("bug"),
    );

    let setup = Setup::at(dir.path(), kit.path(), &roster()).expect("starts anyway");
    assert_eq!(bug(&setup).source(), WorkflowSource::Armada);
    let said: Vec<String> = setup.left_out().iter().map(ToString::to_string).collect();
    let [line] = said.as_slice() else {
        panic!("one sentence naming both: {said:?}");
    };
    assert!(
        line.contains("mine.yml")
            && line.contains("also-mine.yml")
            && line.ends_with("; Armada's `bug` is used instead"),
        "{line}"
    );
}

/// Kit's home is made where it was not, with somewhere for Workflows to go, and
/// making it again is no act at all.
#[test]
fn kits_home_is_made_with_a_place_for_workflows() {
    let home = TempDir::new();
    let first = kit(home.path()).expect("made");
    assert_eq!(first, home.path().join(KIT_HOME));
    assert!(first.join(KIT_WORKFLOWS).is_dir());
    assert_eq!(kit(home.path()).expect("already there"), first);
}

/// Where Kit keeps a person's own `armada.yml` for `dir`, relative to Kit's home.
fn kit_manifest_at(kit: &TempDir, dir: &TempDir) -> String {
    let file = fleet::kit_manifest(kit.path(), &dir.path().to_string_lossy());
    file.strip_prefix(kit.path()).expect("under Kit").to_string_lossy().to_string()
}

/// **A repository with no `armada.yml` of its own runs from the one Kit keeps for it**, and the
/// Manifest is still found at the repository, not in Kit's folder.
#[test]
fn a_repository_with_no_armada_yml_runs_from_kits() {
    let dir = TempDir::new();
    let kit = TempDir::new();
    kit.write(&kit_manifest_at(&kit, &dir), "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\n");

    let setup = Setup::at(dir.path(), kit.path(), &roster()).expect("Kit's is the Manifest");
    assert_eq!(setup.manifest().path(), dir.path().join("armada.yml"));
    assert_eq!(setup.manifest().check_names(), ["test"]);
    assert!(setup.manifest().from_kit().is_some_and(|from| from.whole));
}

/// **Kit's replaces the repository's Check of the same name**, and a re-read keeps laying it
/// over, so nothing reads as changed that did not change.
#[test]
fn kits_check_replaces_the_repositorys_of_the_same_name() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\n");
    let kit = TempDir::new();
    kit.write(&kit_manifest_at(&kit, &dir), "checks:\n  test:\n    run: make quick\n");

    let setup = Setup::at(dir.path(), kit.path(), &roster()).expect("they merge");
    assert_eq!(setup.manifest().check("test").map(|check| check.run().to_string()).as_deref(), Some("make quick"));
    let from = setup.manifest().from_kit().expect("read from Kit");
    assert!(!from.whole && from.names.iter().all(|one| one.hides));
    let (_, _, reloads) = setup.into_parts();
    let adopted = reloads.reread().expect("it reads again");
    assert!(adopted.at_restart().is_empty(), "{:?}", adopted.at_restart());
}

/// **`armada check` runs the Check the Manifest was served with**: Kit's copy is laid over the
/// root's where Kit's home is named, and the repository's is read alone where none is.
#[test]
fn armada_check_reads_kits_copy_where_kits_home_is_named() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\n");
    let kit = TempDir::new();
    kit.write(&kit_manifest_at(&kit, &dir), "checks:\n  test:\n    run: make quick\n");

    let run = |kit: Option<&std::path::Path>| {
        let manifests = crate::manifests::Manifests::load_over(dir.path(), kit).expect("loads");
        manifests.root().check("test").map(|check| check.run().to_string())
    };
    assert_eq!(run(Some(kit.path())).as_deref(), Some("make quick"));
    assert_eq!(run(None).as_deref(), Some("make test"));
}
