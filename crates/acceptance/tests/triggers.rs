//! Triggers' claim: **a Trigger runs at a moment in a Job from a file in the
//! repository or on this machine, the machine's wins, and a file that does not
//! fit is named while the rest stand.**
//!
//! Fleet freezes the set at approval and records each firing; here the Command is
//! a fake that answers the exit code a case plants.
//!
//! | Proved | Not proved |
//! |---|---|
//! | A machine Trigger replaces the repository's of the same identity, whole | That the repository's are read from `main` and never the Job's branch. `armada::Locator` reads them, over a repository |
//! | A Trigger naming a Command the repository does not declare is marked skipped, and the others stand | That a Command really runs in the Job's worktree under the Check budget. `fleet::tests::triggering` runs `true` and `false` there |
//! | A file that will not parse is left out with its reason, and the others stand | That the three moments are where Fleet calls this from. `fleet::tests::triggering` drives a Job through each |
//! | A machine `pr_opened` Trigger is frozen onto the delivering step of any workflow, runs its Command once and is recorded passed | That the frozen set is written at the approval and survives a restart. `store`'s own tests and `fleet::tests::triggering` |
//! | A Command that exits non-zero is recorded failed, or `repairing` with `repair` on, and the Job stays where it was. A repair is told what failed, is bound at two tries, holds a passing fix for the owner, and each choice is a different delivery | A Drone on a branch, a push, a merge and the alert: `fleet::tests::trigger_repair` drives those with a fake Drone and `FakeVcs` |
//! | A Trigger that blocks and fails is `held` and holds the Job, through a repair and until a rerun passes, the owner skips it or a repair ends passed. A failure that does not block holds nothing, and neither does the last step's `step_passes` | Where each moment holds, the rerun, the skip and the refusals: `fleet::tests::trigger_hold` drives a Job through each, and the walk `aTriggerHoldsTheJob` is Bridge's |
//! | A step added to one Job that fails with `repair` on is `repairing` and not ended, as a Trigger is, and holds the Job through it when it blocks. The branch a repair wrote on is done with once its fix is on the Job's branch or the repair failed, and never when it is a pull request's head | The repair itself, the owner's choice and the branch's deletion for an added step: `fleet::tests::addition_repair` drives them, and `adapters` deletes against a real repository |
//! | A Trigger on a Command this repository does not declare is recorded skipped, and a destructive one waits on the owner and is not run. With `block` it holds the Job until he answers, without it holds nothing, his Run ends as any firing does and his Skip records him | The act, the bell and the restart: `fleet::tests::trigger_asks` drives them, and the walk `aDestructiveTriggerAsks` is Bridge's. A step added to one Job on a destructive Command asks the same way: `fleet::tests::addition_asks`, and the walk `anAddedStepAsks` |
//! | A Skill or Drone Trigger (a `brief:` prompt) or Skill or Drone step opens `running`, never skipped, runs no Command, ignores `repair` and holds the Job where it blocks. Its branch is given back when it changed nothing | The side Drone, the slot, `fix_ready`, `passed`, `failed`, Rerun and the restart: `fleet::tests::side_run` drives them with a fake Drone, and the walk `aSkillStepRuns` is Bridge's |
//! | A pull request opens as a draft by the most specific default there is: the Job's own choice, the delivering step's `draft_pr`, the repository's `pr_mode`, this machine's, then ready. `draft_pr` is refused on a step that does not deliver | That Fleet opens the pull request as a draft once approved. `crates/fleet/src/tests/choosing_delivery.rs` drives the fake VCS |

use std::path::{Path, PathBuf};


use config::{Manifest, TriggerCatalogue, TriggerWritten};
use core_model::{
    PrMode, StepId,
    TriggerResolution, TriggerRuns, TriggerSkipped, TriggerSource, TriggerWhen, Ulid,
    WorkflowId,
};

// The bench is shared with the other milestones' tests and none of them uses
// all of it.
#[allow(dead_code)]
mod bench;


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














// ------------------------------------------------------ the draft default

fn workflow_with(
    delivering: &str,
    supporting: &str,
    manifest: &Manifest,
) -> core_model::FrozenWorkflow {
    let text = format!(
        "version: 1\nworkflow_id: drafted\nname: drafted\nsteps:\n  \
         - id: implement\n    label: Implement\n    evidence: {{submitted: {{type: diff}}}}\n    \
         delivers: false\n    advance_gate: auto\n{supporting}  \
         - id: land\n    label: Land\n    evidence: {{submitted: {{type: diff}}}}\n    \
         delivers: true\n    advance_gate: auto\n{delivering}"
    );
    let def = config::WorkflowDef::parse(
        Path::new("drafted.yml"),
        &text,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the workflow did not load: {refused}"));
    config::ResolvedWorkflow::resolve(&def, manifest)
        .expect("the workflow resolves")
        .frozen()
        .clone()
}

fn manifest_saying(pr_mode: Option<&str>) -> Manifest {
    let line = pr_mode
        .map(|mode| format!("pr_mode: {mode}\n"))
        .unwrap_or_default();
    Manifest::parse(
        Path::new("armada.yml"),
        &format!("version: 1\nid: armada\n{line}"),
    )
    .expect("the Manifest is well formed")
}



#[test]
fn draft_pr_is_the_delivering_steps_to_carry_and_is_refused_anywhere_else() {
    let manifest = manifest_saying(None);
    let wrongly = "version: 1\nworkflow_id: drafted\nname: drafted\nsteps:\n  \
                   - id: implement\n    label: Implement\n    evidence: {submitted: {type: diff}}\n    \
                   delivers: false\n    advance_gate: auto\n    draft_pr: true\n";
    let refused = config::WorkflowDef::parse(
        Path::new("drafted.yml"),
        wrongly,
        &config::Roster::offering_nothing(),
    )
    .expect_err("a step that delivers nothing has no pull request to draft");
    assert!(
        refused.to_string().contains("steps[0].draft_pr"),
        "the refusal names the key: {refused}"
    );
    // On the delivering step it loads, and what it says is carried.
    let carried = workflow_with("    draft_pr: true\n", "", &manifest);
    assert_eq!(
        carried.delivering_step().and_then(|step| step.draft_pr()),
        Some(PrMode::Draft)
    );
}







