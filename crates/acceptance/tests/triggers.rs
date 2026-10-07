//! Triggers' claim: **something a person wants run at a moment in a Job is a
//! file they can put in the repository or on their machine, the machine's copy
//! wins, and a file that does not fit is named while the rest stand.**
//!
//! Asserted over text: the loader takes a file's text and the place it came
//! from, and Fleet's firing of one is a later change.
//!
//! | Proved | Not proved |
//! |---|---|
//! | A machine Trigger replaces the repository's of the same identity, whole | That Fleet fires one at its moment, and that the repository's are read from `main` and never the Job's branch. Fleet reads the files |
//! | A Trigger naming a Command the repository does not declare is marked skipped, and the others stand | That a Trigger freezes onto the Job at approval |
//! | A file that will not parse is left out with its reason, and the others stand | That a Trigger on a destructive Command asks first. The flag is carried and nothing enforces it |

use std::path::{Path, PathBuf};

use config::{Manifest, TriggerCatalogue, TriggerWritten};
use core_model::{
    StepId, TriggerResolution, TriggerRuns, TriggerSkipped, TriggerSource, TriggerWhen, Ulid,
    WorkflowId,
};

const MANIFEST: &str = "version: 1\nid: armada\ncommands:\n  fmt:\n    run: cargo fmt\n  \
                        wipe:\n    run: rm -rf target\n    destructive: true\n";

fn manifest() -> Manifest {
    Manifest::parse(Path::new("armada.yml"), MANIFEST).expect("the Manifest is well formed")
}

fn repository(file: &str, text: &str) -> TriggerWritten {
    TriggerWritten::in_repository(
        PathBuf::from(".armada/triggers").join(file),
        text.to_string(),
    )
}

fn machine(file: &str, text: &str) -> TriggerWritten {
    TriggerWritten::on_machine(
        PathBuf::from("/home/user/.armada/machine/triggers").join(file),
        text.to_string(),
    )
}

#[test]
fn a_machine_trigger_replaces_the_repositorys_of_the_same_identity() {
    let by_the_repository = "name: tidy\nwhen: step_passes\nstep: implement\ncommand: fmt\n";
    let by_this_machine =
        "name: tidy\nwhen: step_passes\nstep: implement\ncommand: wipe\non_failure: {block: true}\n";

    let resolved = TriggerCatalogue::of([
        repository("tidy.yml", by_the_repository),
        machine("tidy.yml", by_this_machine),
    ])
    .resolve(&manifest());

    assert!(resolved.left_out().is_empty());
    let [only] = resolved.triggers() else {
        panic!(
            "one identity is one Trigger, found {:?}",
            resolved.triggers()
        );
    };
    assert_eq!(only.source(), TriggerSource::Machine);
    assert_eq!(
        only.trigger().runs(),
        &TriggerRuns::Command("wipe".to_string())
    );
    // Whole, with no field merge: the repository's copy said nothing about
    // blocking, and this machine's says it does, so there is nothing to merge
    // the other way. The destructive flag comes from the Command it names.
    assert!(only.trigger().on_failure().block);
    assert_eq!(
        only.resolution(),
        &TriggerResolution::Command {
            name: "wipe".to_string(),
            asks_first: true
        }
    );
}

#[test]
fn an_undeclared_command_is_skipped_and_a_malformed_file_is_left_out() {
    let resolved = TriggerCatalogue::of([
        repository("tidy.yml", "name: tidy\nwhen: step_passes\ncommand: fmt\n"),
        repository(
            "lint.yml",
            "name: lint\nwhen: step_starts\ncommand: clippy\n",
        ),
        repository("broken.yml", "name: broken\nwhen: whenever\ncommand: fmt\n"),
    ])
    .resolve(&manifest());

    let bug = WorkflowId::carried(Ulid::carried("bug"));
    let implement = StepId::new("implement");

    let standing: Vec<_> = resolved
        .applying(&bug, TriggerWhen::StepPasses, &implement)
        .collect();
    assert_eq!(standing.len(), 1);
    assert_eq!(
        standing[0].resolution(),
        &TriggerResolution::Command {
            name: "fmt".to_string(),
            asks_first: false
        }
    );

    let skipped: Vec<_> = resolved
        .applying(&bug, TriggerWhen::StepStarts, &implement)
        .collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(
        skipped[0].resolution(),
        &TriggerResolution::Skipped(TriggerSkipped::NotInThisRepo {
            command: "clippy".to_string()
        })
    );

    let [left_out] = resolved.left_out() else {
        panic!("one file was bad, found {:?}", resolved.left_out());
    };
    assert!(left_out.path().ends_with("broken.yml"));
    assert_eq!(left_out.source(), TriggerSource::Repository);
    let why = left_out.to_string();
    assert!(why.contains("`when`") && why.contains("whenever"), "{why}");
}
