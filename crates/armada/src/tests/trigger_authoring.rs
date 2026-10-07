//! Saving a Trigger, and what a list then says. Real files in a temporary
//! repository and a temporary machine folder, because what is claimed is what
//! ends up on disk and what the loader then reads from it.

use std::path::PathBuf;

use config::{Manifest, TriggerCatalogue, TriggerWritten};
use fleet::repositories::{Locating, TriggerNotSaved};
use ipc::{SaveTrigger, TriggerLevel, TriggerScope};

use crate::locating::Locator;
use crate::tests::setup::roster;
use crate::tests::TempDir;
use crate::trigger_authoring::{remove, save};

const MANIFEST: &str = "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  fmt:\n    run: cargo fmt\n";
const TIDY: &str = "name: tidy\nwhen: step_passes\nstep: implement\ncommand: fmt\n";

fn manifest() -> Manifest {
    Manifest::parse(std::path::Path::new("armada.yml"), MANIFEST).expect("a Manifest")
}

fn asking(scope: TriggerScope, definition: &str, overwrite: bool) -> SaveTrigger {
    SaveTrigger {
        scope,
        definition: definition.to_string(),
        overwrite,
        kept_from: None,
    }
}

#[test]
fn a_command_the_repository_does_not_declare_is_refused_with_the_loaders_reason_and_nothing_is_written(
) {
    let repository = TempDir::new();
    let machine = TempDir::new();
    let refused = save(
        repository.path(),
        machine.path(),
        &manifest(),
        &asking(
            TriggerScope::Repository,
            "name: lint\nwhen: step_starts\ncommand: clippy\n",
            false,
        ),
    )
    .expect_err("clippy is not declared");
    let TriggerNotSaved::Unfit { why } = refused else {
        panic!("a refusal for the definition: {refused:?}");
    };
    assert!(
        why.contains("skipped: `clippy` is not in this repo"),
        "{why}"
    );
    assert!(
        !repository.path().join(".armada").exists(),
        "not even the folder"
    );
}

#[test]
fn a_file_the_loader_would_leave_out_is_refused_in_its_own_words() {
    let repository = TempDir::new();
    let machine = TempDir::new();
    let refused = save(
        repository.path(),
        machine.path(),
        &manifest(),
        &asking(
            TriggerScope::Machine,
            "name: x\nwhen: whenever\ncommand: fmt\n",
            false,
        ),
    )
    .expect_err("whenever is not a moment");
    let TriggerNotSaved::Unfit { why } = refused else {
        panic!("a refusal for the definition: {refused:?}");
    };
    assert!(why.contains("whenever"), "{why}");
    assert!(!machine.path().join("machine").exists());
}

#[test]
fn a_machine_trigger_for_a_command_this_repository_lacks_is_saved_and_says_so() {
    let repository = TempDir::new();
    let machine = TempDir::new();
    let saved = save(
        repository.path(),
        machine.path(),
        &manifest(),
        &asking(
            TriggerScope::Machine,
            "name: lint\nwhen: step_starts\ncommand: clippy\n",
            false,
        ),
    )
    .expect("the same file is right in the next repository");
    assert!(matches!(
        saved.trigger.resolution(),
        core_model::TriggerResolution::Skipped(_)
    ));
}

#[test]
fn replacing_is_said_and_a_machine_save_then_replaces_the_repositorys_in_the_list() {
    let repository = TempDir::new();
    let machine = TempDir::new();
    let manifest = manifest();

    // The repository's copy, as the base branch would hand it over.
    let by_the_repository =
        TriggerWritten::in_repository(PathBuf::from(".armada/triggers/tidy.yml"), TIDY.to_string());
    let theirs =
        "name: tidy\nwhen: step_passes\nstep: implement\ncommand: fmt\non_failure: {block: true}\n";

    let first = save(
        repository.path(),
        machine.path(),
        &manifest,
        &asking(TriggerScope::Machine, theirs, false),
    )
    .expect("a first save");
    assert!(!first.replaced);
    let again = save(
        repository.path(),
        machine.path(),
        &manifest,
        &asking(TriggerScope::Machine, theirs, false),
    )
    .expect_err("it is there");
    assert!(matches!(again, TriggerNotSaved::Exists { .. }));
    assert!(
        save(
            repository.path(),
            machine.path(),
            &manifest,
            &asking(TriggerScope::Machine, theirs, true),
        )
        .expect("overwritten")
        .replaced
    );

    // What a Fleet reads next: the machine's files, then the repository's.
    let locator = Locator::at(machine.path(), machine.path().join("kit"), roster());
    let mut written = vec![by_the_repository];
    written.extend(locator.triggers(repository.path(), None));
    let list = fleet::trigger_list(&TriggerCatalogue::of(written).resolve(&manifest));

    let [one] = list.triggers.as_slice() else {
        panic!("one identity is one Trigger: {:?}", list.triggers);
    };
    assert_eq!(one.level, TriggerLevel::Machine);
    assert!(one.block, "its copy is whole, with no field merge");
    assert_eq!(one.overrides.len(), 1);
    assert_eq!(one.overrides[0].level, TriggerLevel::Repository);
    assert!(list.left_out.is_empty());
}

#[test]
fn removing_deletes_the_file_that_holds_the_identity() {
    let repository = TempDir::new();
    let machine = TempDir::new();
    save(
        repository.path(),
        machine.path(),
        &manifest(),
        &asking(TriggerScope::Machine, TIDY, false),
    )
    .expect("saved");
    let gone = ipc::RemoveTrigger {
        scope: TriggerScope::Machine,
        when: ipc::TriggerMoment::StepPasses,
        step: Some(ipc::StepId::carried("implement")),
        name: "tidy".into(),
    };
    remove(repository.path(), machine.path(), &gone).expect("removed");
    assert!(
        remove(repository.path(), machine.path(), &gone).is_err(),
        "nothing left to remove"
    );
}
