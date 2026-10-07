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
//! | A Command that exits non-zero is recorded failed and the Job stays where it was, step and status | That `block` and `repair` do anything. They are carried in the record and nothing acts on them |
//! | A Trigger on a Command this repository does not declare is recorded skipped, and a destructive one waits on the owner and is not run | That the owner is asked. Nothing asks him yet |

use std::path::{Path, PathBuf};

use std::cell::RefCell;

use config::{Manifest, TriggerCatalogue, TriggerWritten};
use core_model::{
    Job, JobStatus, StepId, StepState, Timestamp, TriggerFiring, TriggerResolution, TriggerRuns,
    TriggerSkipped, TriggerSource, TriggerState, TriggerWhen, Ulid, WorkflowId,
};
use fleet::triggering::{self, Planned};
use testkit::{FakeJudge, FakeWorkProduct};
use verification::{Exit, NeverRan};

// The bench is shared with the other milestones' tests and none of them uses
// all of it.
#[allow(dead_code)]
mod bench;

use bench::board::on_its_branch;
use bench::landing::sends_it_out;
use bench::{a_fix_diff, a_root_cause_note, Bench, Run};

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

// ---------------------------------------------------------------------------
// The owner's case: deploy to QA when the pull request opens
// ---------------------------------------------------------------------------

const DECLARING_DEPLOY: &str =
    "version: 1\nid: armada\ncommands:\n  deploy_qa:\n    run: deploy-qa\n  \
                                wipe_qa:\n    run: wipe-qa\n    destructive: true\n";
const NOT_DECLARING_IT: &str = "version: 1\nid: armada\ncommands:\n  fmt:\n    run: cargo fmt\n";

/// Declared on his machine, so it applies to every workflow and every
/// repository he works in.
fn deploys_when_the_pull_request_opens() -> TriggerWritten {
    machine(
        "deploy.yml",
        "name: deploy\nwhen: pr_opened\ncommand: deploy_qa\n",
    )
}

fn manifest_text(text: &str) -> Manifest {
    Manifest::parse(Path::new("armada.yml"), text).expect("the Manifest is well formed")
}

fn at(second: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-10-07T09:00:{second:02}.000Z"))
}

/// A Job whose two working steps passed, standing at the entry to the step its
/// workflow says delivers: **the moment the branch goes out and a pull request
/// opens.** Moved by its own machines, as `landing.rs` moves one.
async fn a_job_entering_its_delivering_step() -> Run {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        sends_it_out(),
        FakeJudge::that_fails("a Judge that should never be asked"),
    );
    let mut run = bench.created("fix the cursor that reads past the end");
    on_its_branch(&mut run);
    bench.approved_and_dispatched(&mut run);
    for (at, submitted) in [(0, a_root_cause_note()), (1, a_fix_diff())] {
        let step = bench.step(at);
        let ruling = bench.gate(&run, &step, &submitted).await;
        bench.settled(&mut run, &step, &ruling);
    }
    run
}

/// What the fake Command was asked to run, in order.
struct Commands {
    asked: RefCell<Vec<String>>,
    exits: Exit,
}

impl Commands {
    fn exiting(exits: Exit) -> Commands {
        Commands {
            asked: RefCell::new(Vec::new()),
            exits,
        }
    }
}

/// Freeze the Job's Triggers at its approval and fire the ones for `when`, as
/// Fleet does: record each as it opens, run what there is to run, record how it
/// ended. **The loop is Fleet's and is written out here** because Fleet itself
/// needs a store and a process, and what is proved is what each step of it
/// decides.
fn fired(
    job: &Job,
    files: Vec<TriggerWritten>,
    manifest: &Manifest,
    when: TriggerWhen,
    step: &StepId,
    commands: &Commands,
) -> Vec<TriggerFiring> {
    let resolved = TriggerCatalogue::of(files).resolve(manifest);
    let frozen = triggering::freeze(&resolved, job.workflow_id(), job.workflow());
    let planned: Vec<Planned> = triggering::plan(&frozen, when, step, manifest);
    planned
        .iter()
        .map(|one| {
            let opened = one.opened(at(1));
            match one.to_run() {
                Some(command) => {
                    commands.asked.borrow_mut().push(command.to_string());
                    one.ended(opened, &commands.exits, at(2))
                }
                None => opened,
            }
        })
        .collect()
}

#[tokio::test]
async fn a_machine_trigger_runs_its_command_once_when_the_pull_request_opens() {
    let run = a_job_entering_its_delivering_step().await;
    let delivering = run.job.workflow().delivering_step().expect("one delivers");
    let manifest = manifest_text(DECLARING_DEPLOY);
    let commands = Commands::exiting(Exit::Code(0));

    // Nothing the workflow does before the pull request opens runs it.
    for step in run.job.workflow().steps() {
        for when in [TriggerWhen::StepStarts, TriggerWhen::StepPasses] {
            let none = fired(
                &run.job,
                vec![deploys_when_the_pull_request_opens()],
                &manifest,
                when,
                step.id(),
                &commands,
            );
            assert!(none.is_empty(), "{when:?} at {:?}: {none:?}", step.id());
        }
    }
    assert!(commands.asked.borrow().is_empty());

    let firings = fired(
        &run.job,
        vec![deploys_when_the_pull_request_opens()],
        &manifest,
        TriggerWhen::PrOpened,
        delivering.id(),
        &commands,
    );

    assert_eq!(*commands.asked.borrow(), ["deploy-qa"], "exactly once");
    let [only] = firings.as_slice() else {
        panic!("one Trigger, one firing: {firings:?}");
    };
    assert_eq!(only.name, "deploy");
    assert_eq!(only.state, TriggerState::Passed);
    assert_eq!(only.exit_code, Some(0));
    assert_eq!(only.source, TriggerSource::Machine);
    assert_eq!(only.when, TriggerWhen::PrOpened);
    assert_eq!(&only.step, delivering.id());
    assert_eq!(only.started_at, at(1));
    assert_eq!(only.ended_at, Some(at(2)));
}

#[tokio::test]
async fn a_command_that_fails_is_recorded_and_the_job_stays_where_it_was() {
    let run = a_job_entering_its_delivering_step().await;
    let before = (run.job.status(), run.job.current_step_id().cloned());
    let delivering = run.job.workflow().delivering_step().expect("one delivers");
    let commands = Commands::exiting(Exit::Code(2));

    let firings = fired(
        &run.job,
        vec![deploys_when_the_pull_request_opens()],
        &manifest_text(DECLARING_DEPLOY),
        TriggerWhen::PrOpened,
        delivering.id(),
        &commands,
    );

    let [only] = firings.as_slice() else {
        panic!("one Trigger, one firing: {firings:?}");
    };
    assert_eq!(only.state, TriggerState::Failed);
    assert_eq!(only.exit_code, Some(2));
    assert!(!only.on_failure.block && !only.on_failure.repair);
    // Nothing a firing returns reaches the Job, so what is asserted is that the
    // Job a person sees is the Job that was there.
    assert_eq!(run.job.status(), before.0);
    assert_eq!(run.job.status(), JobStatus::Running);
    assert_eq!(run.job.current_step_id().cloned(), before.1);
    assert_eq!(
        run.job.step(delivering.id()).map(|step| step.state()),
        Some(StepState::Running)
    );

    // A command that never started is a failure with no code to report.
    let missing = Commands::exiting(Exit::NeverRan(NeverRan::NoSuchCommand {
        program: "deploy-qa".to_string(),
    }));
    let [ref never] = fired(
        &run.job,
        vec![deploys_when_the_pull_request_opens()],
        &manifest_text(DECLARING_DEPLOY),
        TriggerWhen::PrOpened,
        delivering.id(),
        &missing,
    )[..] else {
        panic!("one firing");
    };
    assert_eq!((never.state, never.exit_code), (TriggerState::Failed, None));
}

#[tokio::test]
async fn a_repository_without_the_command_records_it_skipped_and_runs_nothing() {
    let run = a_job_entering_its_delivering_step().await;
    let delivering = run.job.workflow().delivering_step().expect("one delivers");
    let commands = Commands::exiting(Exit::Code(0));

    let firings = fired(
        &run.job,
        vec![deploys_when_the_pull_request_opens()],
        &manifest_text(NOT_DECLARING_IT),
        TriggerWhen::PrOpened,
        delivering.id(),
        &commands,
    );

    assert!(commands.asked.borrow().is_empty(), "never run");
    let [only] = firings.as_slice() else {
        panic!("one Trigger, one firing: {firings:?}");
    };
    assert_eq!(only.state, TriggerState::Skipped);
    assert_eq!(
        only.skipped,
        Some(TriggerSkipped::NotInThisRepo {
            command: "deploy_qa".to_string()
        })
    );
    assert_eq!(only.exit_code, None);
}

#[tokio::test]
async fn a_destructive_command_waits_on_the_owner_and_a_skill_is_not_run_yet() {
    let run = a_job_entering_its_delivering_step().await;
    let delivering = run.job.workflow().delivering_step().expect("one delivers");
    let commands = Commands::exiting(Exit::Code(0));

    let firings = fired(
        &run.job,
        vec![
            machine(
                "wipe.yml",
                "name: wipe\nwhen: pr_opened\ncommand: wipe_qa\n",
            ),
            machine("lint.yml", "name: lint\nwhen: pr_opened\nskill: tidy-up\n"),
        ],
        &manifest_text(DECLARING_DEPLOY),
        TriggerWhen::PrOpened,
        delivering.id(),
        &commands,
    );

    assert!(commands.asked.borrow().is_empty(), "neither ran");
    let state = |name: &str| firings.iter().find(|one| one.name == name).expect(name);
    assert_eq!(state("wipe").state, TriggerState::AwaitingOwner);
    assert_eq!(state("wipe").ended_at, None);
    assert_eq!(state("lint").state, TriggerState::Skipped);
    assert_eq!(
        state("lint").skipped,
        Some(TriggerSkipped::SkillNotRun {
            skill: "tidy-up".to_string()
        })
    );
}
