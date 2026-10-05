//! Saving a workflow, and what a running Fleet makes of the folders afterwards.
//!
//! Real files in a temporary repository and a temporary Kit, because what is
//! claimed is what ends up on disk and what a loader then reads from it.

use std::time::Duration;

use config::{Manifest, WorkflowSource};
use fleet::repositories::WorkflowNotSaved;
use ipc::{SaveWorkflow, WorkflowScope};
use tokio::sync::mpsc;

use crate::authoring::save;
use crate::setup::{workflows_again, Setup, KIT_WORKFLOWS, WORKFLOWS};
use crate::tests::setup::{a_repository, roster};
use crate::tests::TempDir;
use crate::watching::watch_folders_every;

/// A one-step definition with a label that says whose it is, gated on `check`
/// where one is named.
fn defining(id: &str, label: &str, check: Option<&str>) -> String {
    let gate = check.map_or(String::new(), |check| {
        format!(
            "    evidence: {{submitted: {{type: diff}}}}\n    mechanical_checks:\n      - {{ type: \
             manifest_check, check: {check} }}\n"
        )
    });
    format!(
        "version: 1\nworkflow_id: {id}\nname: {id}\nsteps:\n  - id: only\n    \
         label: \"{label}\"\n    delivers: true\n    advance_gate: auto\n{gate}"
    )
}

fn asking(scope: WorkflowScope, definition: String, overwrite: bool) -> SaveWorkflow {
    SaveWorkflow {
        scope,
        definition,
        overwrite,
    }
}

fn manifest(repository: &TempDir) -> Manifest {
    Manifest::load(&repository.path().join("armada.yml")).expect("the fixture Manifest loads")
}

fn label(
    held: &std::collections::BTreeMap<core_model::WorkflowId, config::ResolvedWorkflow>,
    id: &str,
) -> (String, WorkflowSource) {
    let one = held
        .values()
        .find(|workflow| workflow.id().as_str() == id)
        .unwrap_or_else(|| panic!("`{id}` is held"));
    (one.steps()[0].label().to_string(), one.source())
}

fn files_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// **A definition that does not fit writes nothing** — not the file, not the
/// folder it would have gone in, and not a temporary one.
#[test]
fn a_definition_that_does_not_fit_writes_nothing() {
    let repository = a_repository();
    let kit = TempDir::new();
    let manifest = manifest(&repository);
    for (what, text) in [
        (
            "it names a Check the Manifest does not declare",
            defining("x", "X", Some("missing")),
        ),
        (
            "it is not a definition",
            "workflow_id: [unclosed".to_string(),
        ),
        (
            "it names a key nothing reads",
            format!("{}banana: 1\n", defining("x", "X", None)),
        ),
    ] {
        for scope in [WorkflowScope::Repository, WorkflowScope::Kit] {
            let refused = save(
                repository.path(),
                kit.path(),
                &manifest,
                &roster(),
                &asking(scope, text.clone(), false),
            )
            .expect_err(what);
            assert!(
                matches!(refused, WorkflowNotSaved::Unfit { .. }),
                "{what}: {refused:?}"
            );
        }
    }
    assert!(
        !repository.path().join(".armada").exists(),
        "the repository's folder was made for a definition that was refused"
    );
    assert!(files_in(&kit.path().join(KIT_WORKFLOWS)).is_empty());
}

#[test]
fn the_reason_is_the_loaders_own_and_names_what_is_missing() {
    let repository = a_repository();
    let kit = TempDir::new();
    let refused = save(
        repository.path(),
        kit.path(),
        &manifest(&repository),
        &roster(),
        &asking(
            WorkflowScope::Repository,
            defining("x", "X", Some("missing")),
            false,
        ),
    )
    .expect_err("a Check nobody declared");
    let WorkflowNotSaved::Unfit { why } = refused else {
        panic!("{refused:?}");
    };
    assert!(why.contains("missing"), "{why}");
}

#[test]
fn an_id_that_cannot_be_a_file_name_is_refused_and_writes_nothing() {
    let repository = a_repository();
    let kit = TempDir::new();
    let refused = save(
        repository.path(),
        kit.path(),
        &manifest(&repository),
        &roster(),
        &asking(
            WorkflowScope::Repository,
            defining("../escape", "X", None),
            false,
        ),
    )
    .expect_err("an id that is a path");
    assert!(
        matches!(refused, WorkflowNotSaved::NotAName { .. }),
        "{refused:?}"
    );
    assert!(!repository.path().join(".armada").exists());
    assert!(!repository
        .path()
        .parent()
        .expect("a parent")
        .join("escape.json")
        .exists());
}

/// A saved definition loads: **start reads it back as the repository's own**,
/// and a Kit one as Kit's.
#[test]
fn a_saved_definition_is_the_file_start_reads_back() {
    let repository = a_repository();
    let kit = TempDir::new();
    let manifest = manifest(&repository);
    let in_repository = save(
        repository.path(),
        kit.path(),
        &manifest,
        &roster(),
        &asking(
            WorkflowScope::Repository,
            defining("mine", "Mine", None),
            false,
        ),
    )
    .expect("saved");
    let in_kit = save(
        repository.path(),
        kit.path(),
        &manifest,
        &roster(),
        &asking(
            WorkflowScope::Kit,
            defining("everyones", "Everyone's", None),
            false,
        ),
    )
    .expect("saved");
    assert!(!in_repository.replaced && !in_kit.replaced);
    assert!(
        in_repository
            .file
            .ends_with(&format!("{WORKFLOWS}/mine.json")),
        "{}",
        in_repository.file
    );
    assert!(
        in_kit
            .file
            .ends_with(&format!("{KIT_WORKFLOWS}/everyones.json")),
        "{}",
        in_kit.file
    );

    let setup = Setup::at(repository.path(), kit.path(), &roster()).expect("it starts");
    assert_eq!(
        label(setup.workflows(), "mine"),
        ("Mine".to_string(), WorkflowSource::Repository)
    );
    assert_eq!(
        label(setup.workflows(), "everyones"),
        ("Everyone's".to_string(), WorkflowSource::Kit)
    );
    assert!(
        files_in(&repository.path().join(WORKFLOWS))
            .iter()
            .all(|name| name.ends_with(".json")),
        "no temporary file is left beside it"
    );
}

/// **The repository's definition beats Kit's by id**, whichever was saved first.
#[test]
fn the_repositorys_definition_beats_kits_by_id() {
    for repository_first in [true, false] {
        let repository = a_repository();
        let kit = TempDir::new();
        let manifest = manifest(&repository);
        let saves = [
            (WorkflowScope::Repository, "The repository's"),
            (WorkflowScope::Kit, "Kit's"),
        ];
        for (scope, said) in if repository_first {
            saves
        } else {
            [saves[1], saves[0]]
        } {
            save(
                repository.path(),
                kit.path(),
                &manifest,
                &roster(),
                &asking(scope, defining("shared", said, None), false),
            )
            .expect("saved");
        }
        let (held, _, _) = workflows_again(repository.path(), kit.path(), &roster(), &manifest);
        assert_eq!(
            label(&held, "shared"),
            ("The repository's".to_string(), WorkflowSource::Repository),
            "repository saved first: {repository_first}"
        );
    }
}

/// **Replacing is said, and replaces the file that held the id** — whatever
/// that file is called, so a second file never makes the repository refuse.
#[test]
fn replacing_a_definition_needs_overwrite_and_replaces_the_file_that_held_the_id() {
    let repository = a_repository();
    let kit = TempDir::new();
    let manifest = manifest(&repository);
    repository.write(
        &format!("{WORKFLOWS}/hand-written.yml"),
        &defining("bug", "Before", None),
    );
    let there = repository.path().join(WORKFLOWS);

    let refused = save(
        repository.path(),
        kit.path(),
        &manifest,
        &roster(),
        &asking(
            WorkflowScope::Repository,
            defining("bug", "After", None),
            false,
        ),
    )
    .expect_err("without overwrite");
    let WorkflowNotSaved::Exists { id, file } = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(id, "bug");
    assert!(file.ends_with("hand-written.yml"), "{file}");
    assert!(std::fs::read_to_string(there.join("hand-written.yml"))
        .expect("read")
        .contains("Before"));

    let saved = save(
        repository.path(),
        kit.path(),
        &manifest,
        &roster(),
        &asking(
            WorkflowScope::Repository,
            defining("bug", "After", None),
            true,
        ),
    )
    .expect("with overwrite");
    assert!(saved.replaced);
    assert!(std::fs::read_to_string(there.join("hand-written.yml"))
        .expect("read")
        .contains("After"));
    assert_eq!(
        files_in(&there),
        ["hand-written.yml"],
        "no second file for the id"
    );
    Setup::at(repository.path(), kit.path(), &roster()).expect("and it still starts");
}

#[test]
fn a_file_named_for_the_id_that_defines_something_else_is_not_replaced() {
    let repository = a_repository();
    let kit = TempDir::new();
    repository.write(
        &format!("{WORKFLOWS}/bug.json"),
        &defining("other", "Other", None),
    );

    let refused = save(
        repository.path(),
        kit.path(),
        &manifest(&repository),
        &roster(),
        &asking(
            WorkflowScope::Repository,
            defining("bug", "Bug", None),
            true,
        ),
    )
    .expect_err("even with overwrite");

    assert!(
        matches!(refused, WorkflowNotSaved::Unfit { .. }),
        "{refused:?}"
    );
    assert!(
        std::fs::read_to_string(repository.path().join(WORKFLOWS).join("bug.json"))
            .expect("read")
            .contains("other")
    );
}

/// **One bad file does not take the others down**, on a Fleet that is running
/// and at start alike.
#[test]
fn a_bad_file_is_left_out_with_its_reason_and_the_others_stand() {
    let repository = a_repository();
    let kit = TempDir::new();
    repository.write(
        &format!("{WORKFLOWS}/fine.json"),
        &defining("fine", "Fine", None),
    );
    repository.write(
        &format!("{WORKFLOWS}/broken.json"),
        "workflow_id: [unclosed",
    );
    repository.write(
        &format!("{WORKFLOWS}/needs.json"),
        &defining("needs", "Needs", Some("missing")),
    );
    kit.write(
        &format!("{KIT_WORKFLOWS}/everyones.json"),
        &defining("everyones", "Everyone's", None),
    );

    let (held, left_out, _) = workflows_again(
        repository.path(),
        kit.path(),
        &roster(),
        &manifest(&repository),
    );

    assert_eq!(label(&held, "fine").1, WorkflowSource::Repository);
    assert_eq!(label(&held, "everyones").1, WorkflowSource::Kit);
    assert!(
        held.keys().any(|id| id.as_str() == "bug"),
        "what Armada carries stands too"
    );
    assert!(!held.keys().any(|id| id.as_str() == "needs"));
    let said: Vec<String> = left_out.iter().map(ToString::to_string).collect();
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said.iter().any(|s| s.contains("broken.json")), "{said:?}");
    assert!(
        said.iter()
            .any(|s| s.starts_with("the repository's `needs` was left out")),
        "{said:?}"
    );

    let setup = Setup::at(repository.path(), kit.path(), &roster())
        .expect("start leaves a repository's own bad file out rather than refusing");
    assert_eq!(setup.left_out().len(), 2);
    assert_eq!(
        label(setup.workflows(), "fine").1,
        WorkflowSource::Repository
    );
}

/// Waits for the watch to say something, or fails naming the wait.
async fn noticed(told: &mut mpsc::UnboundedReceiver<()>) {
    tokio::time::timeout(Duration::from_secs(10), told.recv())
        .await
        .expect("the change was noticed")
        .expect("the watch is still running");
}

/// **A folder that does not exist yet is watched**, a first definition is seen,
/// and so is an edit, a replace-by-rename and a deletion.
#[tokio::test(flavor = "multi_thread")]
async fn the_workflow_folders_are_watched_whether_or_not_they_exist_yet() {
    let repository = a_repository();
    let kit = TempDir::new();
    let folder = repository.path().join(WORKFLOWS);
    let (told, mut changes) = mpsc::unbounded_channel();
    let _watching = watch_folders_every(
        vec![folder.clone(), kit.path().join(KIT_WORKFLOWS)],
        Duration::from_millis(20),
        Duration::from_millis(120),
        move || {
            let _ = told.send(());
        },
    )
    .expect("a watch on folders that are not there");

    repository.write(
        &format!("{WORKFLOWS}/first.json"),
        &defining("first", "One", None),
    );
    noticed(&mut changes).await;

    repository.write(
        &format!("{WORKFLOWS}/first.json"),
        &defining("first", "Two", None),
    );
    noticed(&mut changes).await;

    repository.write(
        &format!("{WORKFLOWS}/first.saving"),
        &defining("first", "Three", None),
    );
    std::fs::rename(folder.join("first.saving"), folder.join("first.json")).expect("renamed over");
    noticed(&mut changes).await;

    kit.write(
        &format!("{KIT_WORKFLOWS}/everyones.json"),
        &defining("everyones", "E", None),
    );
    noticed(&mut changes).await;

    std::fs::remove_file(folder.join("first.json")).expect("removed");
    noticed(&mut changes).await;
}
