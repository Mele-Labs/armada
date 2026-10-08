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
//! | A Trigger on a Command this repository does not declare is recorded skipped, and a destructive one waits on the owner and is not run. With `block` it holds the Job until he answers, without it holds nothing, his Run ends as any firing does and his Skip records him | The act, the bell and the restart: `fleet::tests::trigger_asks` drives them, and the walk `aDestructiveTriggerAsks` is Bridge's |
//! | A Skill or Drone Trigger (a `brief:` prompt) or Skill or Drone step opens `running`, never skipped, runs no Command, ignores `repair` and holds the Job where it blocks. Its branch is given back when it changed nothing | The side Drone, the slot, `fix_ready`, `passed`, `failed`, Rerun and the restart: `fleet::tests::side_run` drives them with a fake Drone, and the walk `aSkillStepRuns` is Bridge's |
//! | A pull request opens as a draft by the most specific default there is: the Job's own choice, the delivering step's `draft_pr`, the repository's `pr_mode`, this machine's, then ready. `draft_pr` is refused on a step that does not deliver | That Fleet opens the pull request as a draft once approved. `crates/fleet/src/tests/choosing_delivery.rs` drives the fake VCS |

use std::path::{Path, PathBuf};

use std::cell::RefCell;

use config::{Manifest, TriggerCatalogue, TriggerWritten};
use core_model::{
    FixChoice, Job, JobStatus, PrMode, StepId, StepState, Timestamp, TriggerFiring,
    TriggerResolution, TriggerRuns, TriggerSkipped, TriggerSource, TriggerState, TriggerWhen, Ulid,
    WorkflowId,
};
use fleet::trigger_repair::{self, AfterRerun, Delivery};
use fleet::triggering::{self, Planned};
use testkit::{FakeJudge, FakeWorkProduct};
use verification::{Exit, NeverRan};

// The bench is shared with the other milestones' tests and none of them uses
// all of it.
#[allow(dead_code)]
mod bench;

use bench::board::on_its_branch;
use bench::landing::sends_it_out;
use bench::plan::Planned as PlannedJob;
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
async fn a_destructive_command_waits_on_the_owner_and_a_skill_is_a_side_drones_to_run() {
    let run = a_job_entering_its_delivering_step().await;
    let delivering = run.job.workflow().delivering_step().expect("one delivers");
    let commands = Commands::exiting(Exit::Code(0));

    let skill = "name: lint\nwhen: pr_opened\nskill: tidy-up\n\
                 on_failure:\n  block: true\n  repair: true\n";
    let files = vec![
        machine(
            "wipe.yml",
            "name: wipe\nwhen: pr_opened\ncommand: wipe_qa\n",
        ),
        machine("lint.yml", skill),
    ];
    let resolved = TriggerCatalogue::of(files.clone()).resolve(&manifest_text(DECLARING_DEPLOY));
    let frozen = triggering::freeze(&resolved, run.job.workflow_id(), run.job.workflow());
    let firings = fired(
        &run.job,
        files,
        &manifest_text(DECLARING_DEPLOY),
        TriggerWhen::PrOpened,
        delivering.id(),
        &commands,
    );

    assert!(commands.asked.borrow().is_empty(), "no Command ran");
    let state = |name: &str| firings.iter().find(|one| one.name == name).expect(name);
    assert_eq!(state("wipe").state, TriggerState::AwaitingOwner);
    assert_eq!(state("wipe").ended_at, None);
    // A skill is opened running and is a side Drone's; it is never skipped.
    assert_eq!(state("lint").state, TriggerState::Running);
    assert_eq!(state("lint").skipped, None);
    // Block stands; repair is ignored, since a Drone already fixes its own failures.
    let lint = frozen
        .iter()
        .find(|one| one.name == "lint")
        .expect("frozen");
    assert!(lint.on_failure.block && !lint.on_failure.repair);
    assert!(!state("lint").on_failure.repair);

    // A Skill or Drone step in flight reads `running` with a Drone on it, and
    // holds the Job where it blocks. Its branch is done with whether it
    // changed nothing (`passed`, no choice) or failed.
    let record = core_model::RepairRecord {
        tries: 1,
        ..core_model::RepairRecord::default()
    };
    assert!(record.side_run_in_flight(TriggerState::Running));
    assert!(!core_model::RepairRecord::default().side_run_in_flight(TriggerState::Running));
    assert!(trigger_repair::branch_is_done_with(
        TriggerState::Passed,
        None
    ));
    assert!(!trigger_repair::branch_is_done_with(
        TriggerState::FixReady,
        None
    ));
    let first = run
        .job
        .workflow()
        .steps()
        .first()
        .expect("a step")
        .id()
        .clone();
    let step = core_model::AddedStep {
        id: "a1".to_string(),
        kind: core_model::AddedKind::Drone {
            brief: "Add a changelog line.".to_string(),
        },
        when: TriggerWhen::StepPasses,
        step: first,
        on_failure: core_model::OnTriggerFailure {
            block: true,
            repair: false,
        },
        placed: core_model::Placed::WhileRunning,
        added_at: at(0),
        fired: Some(core_model::Fired::running(at(1))),
        kept: None,
        repair: record,
    };
    assert!(step.holds_the_job(run.job.workflow()));
}

#[tokio::test]
async fn a_saved_drone_trigger_carries_its_prompt_and_is_a_side_drones_to_run() {
    let run = a_job_entering_its_delivering_step().await;
    let delivering = run.job.workflow().delivering_step().expect("one delivers");
    let commands = Commands::exiting(Exit::Code(0));

    let drone = "name: notes\nwhen: pr_opened\nbrief: Add a changelog line.\n\
                 on_failure:\n  block: true\n  repair: true\n";
    let files = vec![machine("notes.yml", drone)];
    let resolved = TriggerCatalogue::of(files.clone()).resolve(&manifest_text(DECLARING_DEPLOY));
    let frozen = triggering::freeze(&resolved, run.job.workflow_id(), run.job.workflow());
    let notes = frozen
        .iter()
        .find(|one| one.name == "notes")
        .expect("frozen");
    assert_eq!(
        notes.resolution,
        core_model::TriggerResolution::Drone {
            brief: "Add a changelog line.".to_string()
        }
    );
    assert!(notes.on_failure.block && !notes.on_failure.repair);

    let firings = fired(
        &run.job,
        files,
        &manifest_text(DECLARING_DEPLOY),
        TriggerWhen::PrOpened,
        delivering.id(),
        &commands,
    );
    assert!(commands.asked.borrow().is_empty(), "no Command ran");
    assert_eq!(firings[0].state, TriggerState::Running, "a Drone is sent");
    assert_eq!(firings[0].skipped, None);
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

/// What a person's approval comes to, read where Fleet reads it.
fn approved_as(
    job_says: Option<&str>,
    step_says: Option<&str>,
    repository_says: Option<&str>,
    machine_drafts: bool,
) -> PrMode {
    let manifest = manifest_saying(repository_says);
    let step = step_says
        .map(|value| format!("    draft_pr: {value}\n"))
        .unwrap_or_default();
    let planned = PlannedJob::created_with("tidy the reader", workflow_with(&step, "", &manifest));
    let body = match job_says {
        Some(mode) => format!(
            r#"{{"landing": {{"branching": "job", "pr_mode": "{mode}", "complete_when": "delivered"}}}}"#
        ),
        None => "{}".to_string(),
    };
    let body: ipc::ApproveDispatch =
        ipc::decode("an approval body", body.as_bytes()).expect("Bridge's body decodes");
    let machine = machine_drafts.then_some(PrMode::Draft);
    let decided =
        fleet::approving::decided_under(&planned.job, &body, None, manifest.pr_mode(), machine)
            .expect("a proposal a person may approve");
    // What the approval served Bridge to start on is what it then froze when
    // the person left it alone.
    if job_says.is_none() {
        assert_eq!(
            fleet::approving::pr_mode_default(planned.job.workflow(), manifest.pr_mode(), machine),
            decided.landing.pr_mode
        );
    }
    decided.landing.pr_mode
}

#[test]
fn a_pull_request_opens_as_the_most_specific_default_says_and_the_jobs_own_choice_beats_them_all() {
    use PrMode::{Draft, Ready};
    // Nothing anywhere is ready, as every Job was before there was a default.
    assert_eq!(approved_as(None, None, None, false), Ready);
    // Each tier, alone, then over the one below it.
    assert_eq!(approved_as(None, None, None, true), Draft, "the machine's");
    assert_eq!(
        approved_as(None, None, Some("draft"), false),
        Draft,
        "the repository's"
    );
    assert_eq!(
        approved_as(None, None, Some("ready"), true),
        Ready,
        "the repository over the machine"
    );
    assert_eq!(approved_as(None, None, Some("draft"), true), Draft);
    assert_eq!(
        approved_as(None, Some("true"), None, false),
        Draft,
        "the step's"
    );
    assert_eq!(
        approved_as(None, Some("false"), Some("draft"), true),
        Ready,
        "the step over the repository"
    );
    assert_eq!(
        approved_as(None, Some("true"), Some("ready"), false),
        Draft,
        "the step over the repository"
    );
    // The Job's own choice, over every tier saying the other.
    assert_eq!(
        approved_as(Some("ready"), Some("true"), Some("draft"), true),
        Ready
    );
    assert_eq!(
        approved_as(Some("draft"), Some("false"), Some("ready"), false),
        Draft
    );
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

const REPAIRING_DEPLOY: &str =
    "name: deploy\nwhen: pr_opened\ncommand: deploy_qa\non_failure:\n  repair: true\n";

/// `deploy_qa` fails after the pull request opens, and the Trigger asked for a
/// repair. Every step after is the repair's own, and none of them is the Job's.
#[tokio::test]
async fn a_failed_repairing_trigger_is_repaired_twice_at_most_and_the_fix_waits_for_the_owner() {
    let run = a_job_entering_its_delivering_step().await;
    let before = (run.job.status(), run.job.current_step_id().cloned());
    let delivering = run.job.workflow().delivering_step().expect("one delivers");
    let commands = Commands::exiting(Exit::Code(1));

    let firings = fired(
        &run.job,
        vec![machine("deploy.yml", REPAIRING_DEPLOY)],
        &manifest_text(DECLARING_DEPLOY),
        TriggerWhen::PrOpened,
        delivering.id(),
        &commands,
    );
    let [failed] = firings.as_slice() else {
        panic!("one Trigger, one firing: {firings:?}");
    };
    // Failed, but the repair is what settles it, so it is neither ended nor failed yet.
    assert_eq!(failed.state, TriggerState::Repairing);
    assert_eq!((failed.exit_code, failed.ended_at.clone()), (Some(1), None));

    // The Drone is told what failed and what to do, and the first line is the
    // one a harness keys on.
    let told = trigger_repair::brief("deploy", "deploy-qa", Some(1), "booting", "no such host");
    assert!(told.starts_with("REPAIR THE TRIGGER `deploy`"));
    for said in [
        "deploy-qa",
        "code 1",
        "booting",
        "no such host",
        "Make the command pass",
    ] {
        assert!(told.contains(said), "{said}: {told}");
    }

    // Two tries, and the second only where the first failed.
    assert_eq!(AfterRerun::of(1, false), AfterRerun::TryAgain);
    assert_eq!(AfterRerun::of(2, false), AfterRerun::GiveUp);
    assert_eq!(AfterRerun::of(1, true).state(), TriggerState::FixReady);
    assert_eq!(AfterRerun::of(2, true).state(), TriggerState::FixReady);
    assert_eq!(AfterRerun::of(2, false).state(), TriggerState::Failed);
    assert_eq!(core_model::REPAIR_TRIES, 2);

    // The owner chooses, and the two choices are two deliveries. A new pull
    // request is the end of it; the Job's own branch is run again first.
    assert_eq!(Delivery::of(FixChoice::NewPr), Delivery::AsAPullRequest);
    assert_eq!(
        Delivery::of(FixChoice::NewPr).state_once_made(),
        TriggerState::Passed
    );
    assert_eq!(
        Delivery::of(FixChoice::ThisBranch),
        Delivery::OntoTheJobsBranch
    );
    assert_eq!(
        Delivery::of(FixChoice::ThisBranch).state_once_made(),
        TriggerState::Rerunning
    );
    assert!(trigger_repair::alert("deploy").contains("deploy"));

    // Non-blocking: nothing above reached the Job.
    assert_eq!(
        (run.job.status(), run.job.current_step_id().cloned()),
        before
    );
    assert_eq!(run.job.status(), JobStatus::Running);
}

/// A Trigger on a destructive Command asks the owner: with `block` on it holds
/// the Job until he answers, without it waits and holds nothing. His Run ends as
/// any firing does, and his Skip records who.
#[tokio::test]
async fn a_destructive_command_that_blocks_holds_the_job_until_he_runs_or_skips_it() {
    let run = a_job_entering_its_delivering_step().await;
    let delivering = run.job.workflow().delivering_step().expect("one delivers");
    let commands = Commands::exiting(Exit::Code(0));
    let asking = |on_failure: &str| {
        fired(
            &run.job,
            vec![machine(
                "wipe.yml",
                &format!("name: wipe\nwhen: pr_opened\ncommand: wipe_qa\n{on_failure}"),
            )],
            &manifest_text(DECLARING_DEPLOY),
            TriggerWhen::PrOpened,
            delivering.id(),
            &commands,
        )
        .remove(0)
    };

    let blocking = asking("on_failure:\n  block: true\n");
    assert_eq!(blocking.state, TriggerState::AwaitingOwner);
    assert!(blocking.holds_the_job());
    let plain = asking("");
    assert_eq!(plain.state, TriggerState::AwaitingOwner);
    assert!(
        !plain.holds_the_job(),
        "without block it waits and holds nothing"
    );
    assert!(commands.asked.borrow().is_empty(), "nothing ran unasked");

    // His Run is a firing like any other: it passes, or it fails as the Trigger says.
    assert_eq!(
        blocking.clone().ended(Some(0), at(9)).state,
        TriggerState::Passed
    );
    assert_eq!(
        blocking.clone().ended(Some(1), at(9)).state,
        TriggerState::Held
    );
    assert_eq!(
        plain.clone().ended(Some(1), at(9)).state,
        TriggerState::Failed
    );
    let repairing = asking("on_failure:\n  block: true\n  repair: true\n");
    assert_eq!(
        repairing.ended(Some(1), at(9)).state,
        TriggerState::Repairing
    );

    // His Skip is a record of who, and it holds nothing.
    let skipped = blocking.skipped_by_the_owner(at(9));
    assert_eq!(skipped.skipped, Some(TriggerSkipped::ByOwner));
    assert!(!skipped.holds_the_job());
}

const BLOCKING_DEPLOY: &str =
    "name: deploy\nwhen: pr_opened\ncommand: deploy_qa\non_failure:\n  block: true\n";

/// `deploy_qa` fails after the pull request opens and the Trigger blocks. The
/// firing is `held` and holds the Job; the Job's own record is where Fleet
/// puts the hold, and the claim here is the rule that decides it.
#[tokio::test]
async fn a_blocking_trigger_that_fails_holds_the_job_until_it_is_let_go() {
    let run = a_job_entering_its_delivering_step().await;
    let delivering = run.job.workflow().delivering_step().expect("one delivers");
    let manifest = manifest_text(DECLARING_DEPLOY);
    let fail = Commands::exiting(Exit::Code(1));
    let one = |file: &str| {
        fired(
            &run.job,
            vec![machine("deploy.yml", file)],
            &manifest,
            TriggerWhen::PrOpened,
            delivering.id(),
            &fail,
        )
        .remove(0)
    };

    let held = one(BLOCKING_DEPLOY);
    assert_eq!(held.state, TriggerState::Held);
    assert!(
        held.ended_at.is_some(),
        "the failure ended; the hold did not"
    );
    assert!(held.holds_the_job());
    // The Job a person sees is where it was: the hold stands in front of the gate.
    assert_eq!(run.job.status(), JobStatus::Running);

    // A failure that does not block holds nothing.
    let plain = one("name: deploy\nwhen: pr_opened\ncommand: deploy_qa\n");
    assert_eq!(plain.state, TriggerState::Failed);
    assert!(!plain.holds_the_job());

    // With `repair` on the hold waits through it, and a repair that did not fix it
    // leaves the hold and not a plain failure.
    let repairing = one(
        "name: deploy\nwhen: pr_opened\ncommand: deploy_qa\non_failure:\n  block: true\n  repair: true\n",
    );
    assert_eq!(repairing.state, TriggerState::Repairing);
    assert!(repairing.holds_the_job());
    assert_eq!(
        repairing.settled_as(TriggerState::Failed),
        TriggerState::Held
    );
    assert_eq!(plain.settled_as(TriggerState::Failed), TriggerState::Failed);
    assert_eq!(
        repairing.settled_as(TriggerState::Passed),
        TriggerState::Passed
    );

    // The owner's skip is a record of who, and it holds nothing.
    let skipped = held.skipped_by_the_owner(at(9));
    assert_eq!(skipped.state, TriggerState::Skipped);
    assert_eq!(skipped.skipped, Some(TriggerSkipped::ByOwner));
    assert!(!skipped.holds_the_job());

    // Nothing stands in front of the end of a Job's last step.
    let last = run.job.workflow().steps().last().expect("a step");
    let first = run.job.workflow().steps().first().expect("a step");
    assert!(core_model::can_hold(
        run.job.workflow(),
        TriggerWhen::StepStarts,
        first.id()
    ));
    assert!(core_model::can_hold(
        run.job.workflow(),
        TriggerWhen::StepPasses,
        first.id()
    ));
    assert!(core_model::can_hold(
        run.job.workflow(),
        TriggerWhen::PrOpened,
        delivering.id()
    ));
    assert!(!core_model::can_hold(
        run.job.workflow(),
        TriggerWhen::StepPasses,
        last.id()
    ));
}

/// An added step is repaired as a Trigger is: the same state, the same hold, the
/// same rule for what becomes of the branch.
#[tokio::test]
async fn an_added_step_that_fails_with_repair_on_goes_through_the_triggers_repair() {
    use core_model::{AddedKind, AddedStep, Fired, OnTriggerFailure, Placed, RepairRecord};

    let run = a_job_entering_its_delivering_step().await;
    let workflow = run.job.workflow();
    let first = workflow.steps().first().expect("a step").id().clone();
    let last = workflow.steps().last().expect("a step").id().clone();
    let added = |block: bool, repair: bool, when: TriggerWhen, step: &StepId| AddedStep {
        id: "a1".to_string(),
        kind: AddedKind::Script {
            command: "deploy_qa".to_string(),
        },
        when,
        step: step.clone(),
        on_failure: OnTriggerFailure { block, repair },
        placed: Placed::WhileRunning,
        added_at: at(0),
        fired: None,
        kept: None,
        repair: RepairRecord::default(),
    };
    let failing = |one: AddedStep| AddedStep {
        fired: Some(Fired::running(at(1)).ended(
            Some(1),
            one.on_failure.block,
            one.on_failure.repair,
            at(2),
        )),
        ..one
    };

    // With `repair` on the failure is `repairing` and has no end, as a Trigger's.
    let repairing = failing(added(false, true, TriggerWhen::StepPasses, &first));
    let fired = repairing.fired.as_ref().expect("fired");
    assert_eq!(
        (fired.state, fired.ended_at.clone()),
        (TriggerState::Repairing, None)
    );
    assert!(!repairing.holds_the_job(workflow), "it does not block");

    // Without it, a failure that blocks is held at once, and one that does not is failed.
    let held = failing(added(true, false, TriggerWhen::StepPasses, &first));
    assert_eq!(
        held.fired.as_ref().map(|f| f.state),
        Some(TriggerState::Held)
    );
    assert!(held.holds_the_job(workflow));
    let plain = failing(added(false, false, TriggerWhen::StepPasses, &first));
    assert_eq!(
        plain.fired.as_ref().map(|f| f.state),
        Some(TriggerState::Failed)
    );

    // A step that blocks holds the Job through every state of the repair.
    let mut blocking = failing(added(true, true, TriggerWhen::StepPasses, &first));
    for state in [
        TriggerState::Repairing,
        TriggerState::Rerunning,
        TriggerState::FixReady,
        TriggerState::Held,
    ] {
        blocking.fired = blocking.fired.map(|fired| Fired { state, ..fired });
        assert!(blocking.holds_the_job(workflow), "{state:?}");
    }
    blocking.fired = blocking.fired.map(|fired| Fired {
        state: TriggerState::Passed,
        ..fired
    });
    assert!(
        !blocking.holds_the_job(workflow),
        "a repair that passed lets it go"
    );

    // Nothing stands in front of the end of the last step, here as for a Trigger.
    let at_the_end = failing(added(true, true, TriggerWhen::StepPasses, &last));
    assert!(!at_the_end.holds_the_job(workflow));

    // The branch a repair wrote on is done with once its fix is on the Job's
    // branch or the repair failed, and a new pull request's is that pull
    // request's head.
    let done = trigger_repair::branch_is_done_with;
    assert!(done(TriggerState::Passed, Some(FixChoice::ThisBranch)));
    assert!(done(TriggerState::Failed, None));
    assert!(done(TriggerState::Held, None));
    assert!(done(TriggerState::Failed, Some(FixChoice::ThisBranch)));
    assert!(!done(TriggerState::Passed, Some(FixChoice::NewPr)));
    assert!(!done(TriggerState::FixReady, None));
    assert!(!done(TriggerState::Repairing, None));
}
