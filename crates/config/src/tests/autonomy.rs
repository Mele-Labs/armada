//! Autonomy defaults are a piece of their own kind: read off a file, scoped to a Workspace or a
//! repository, and the repository's laid over its Workspace's.

use std::path::{Path, PathBuf};

use core_model::rule::{ActKind, Autonomy, AutonomyId};
use core_model::{ManifestId, Ulid};

use crate::piece::Written;
use crate::{parse_autonomy, Autonomies, Workspaces};

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn my_store() -> Workspaces {
    let file = "version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n  - store-web\n  - store-api\n";
    Workspaces::of([Written::in_repository(PathBuf::from("store-web/.armada/workspaces/my-store.yml"), file.into())])
}

fn file(id: &str, scope: Option<&str>, defaults: &str) -> String {
    let scope = scope.map(|one| format!("scope: {one}\n")).unwrap_or_default();
    format!("version: 1\nautonomy_id: {id}\n{scope}defaults:\n{defaults}")
}

fn in_kit(name: &str, text: String) -> Written {
    Written::in_kit(PathBuf::from("kit/autonomy").join(name), text)
}

fn in_repository(root: &str, name: &str, text: String) -> Written {
    Written::in_repository(PathBuf::from(root).join(".armada/autonomy").join(name), text)
}

fn applying(to: &str, written: Vec<Written>) -> Autonomies {
    Autonomies::applying_to(&repository(to), &my_store(), written)
}

#[test]
fn a_file_names_the_kinds_it_sets_and_nothing_else() {
    let read = parse_autonomy(Path::new("a.yml"), &file("mine", Some("my-store"), "  tracker_write: acts\n  forge_write: off\n")).expect("reads");
    assert_eq!(read.id(), &AutonomyId::new("mine").expect("an id"));
    assert_eq!(read.defaults().get(ActKind::TrackerWrite), Some(Autonomy::Acts));
    assert_eq!(read.defaults().get(ActKind::ForgeWrite), Some(Autonomy::Off));
    assert_eq!(read.defaults().get(ActKind::Draft), None, "a kind it does not name is left to the next layer");
}

#[test]
fn a_kind_nobody_has_and_a_word_nobody_means_are_each_refused_and_so_is_a_file_that_sets_nothing() {
    for (defaults, said) in [
        ("  tracker_writes: acts\n", "tracker_writes"),
        ("  tracker_write: sometimes\n", "sometimes"),
        ("  tracker_write: acts\n  forge_write: maybe\n", "maybe"),
    ] {
        let refused = parse_autonomy(Path::new("a.yml"), &file("mine", None, defaults)).expect_err("refused").to_string();
        assert!(refused.contains(said), "{refused}");
    }
    let empty = parse_autonomy(Path::new("a.yml"), "version: 1\nautonomy_id: mine\ndefaults: {}\n").expect_err("refused").to_string();
    assert!(empty.contains("defaults"), "{empty}");
    let unknown = parse_autonomy(Path::new("a.yml"), &format!("{}extra: 1\n", file("mine", None, "  draft: asks\n"))).expect_err("refused").to_string();
    assert!(unknown.contains("extra"), "{unknown}");
}

#[test]
fn the_repository_s_defaults_are_laid_over_its_workspace_s_and_both_over_every_repository_s() {
    let applying = applying(
        "store-web",
        vec![
            in_kit("everywhere.yml", file("everywhere", None, "  tracker_write: asks\n  forge_write: asks\n  start_job: off\n")),
            in_kit("workspace.yml", file("workspace", Some("my-store"), "  tracker_write: acts\n  forge_write: acts\n")),
            in_repository("store-web", "repository.yml", file("repository", Some("store-web"), "  forge_write: off\n")),
            in_repository("store-api", "another.yml", file("another", Some("store-api"), "  start_job: acts\n")),
        ],
    );
    let laid = applying.defaults();
    assert_eq!(laid.get(ActKind::ForgeWrite), Some(Autonomy::Off), "the repository's word wins");
    assert_eq!(laid.get(ActKind::TrackerWrite), Some(Autonomy::Acts), "the Workspace's wins over every repository's");
    assert_eq!(laid.get(ActKind::StartJob), Some(Autonomy::Off), "another repository's file is not this one's");
    assert_eq!(laid.get(ActKind::Draft), None);
}

#[test]
fn the_same_id_is_chosen_between_as_every_piece_is_and_a_file_that_will_not_read_is_set_aside() {
    let applying = applying(
        "store-web",
        vec![
            in_repository("store-web", "theirs.yml", file("defaults", None, "  tracker_write: acts\n")),
            in_kit("mine.yml", file("defaults", None, "  tracker_write: off\n")),
            in_kit("broken.yml", "version: 1\nautonomy_id: Broken Id\ndefaults:\n  draft: asks\n".into()),
        ],
    );
    assert_eq!(applying.defaults().get(ActKind::TrackerWrite), Some(Autonomy::Off), "yours over the repository's");
    assert_eq!(applying.files().len(), 2, "the loser is still listed");
    assert_eq!(applying.left_out().len(), 1);
    assert!(applying.left_out()[0].said().contains("autonomy_id"), "{}", applying.left_out()[0].said());
}

#[test]
fn with_no_file_nothing_is_set() {
    let none = applying("store-web", Vec::new());
    assert!(none.files().is_empty() && none.defaults().iter().next().is_none());
}
