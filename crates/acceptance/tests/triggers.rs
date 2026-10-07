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
//! | A pull request opens as a draft by the most specific default there is: the Job's own choice, the delivering step's `draft_pr`, the repository's `pr_mode`, this machine's, then ready. `draft_pr` is refused on a step that does not deliver | That Fleet opens the pull request as a draft once approved. `crates/fleet/src/tests/choosing_delivery.rs` drives the fake VCS |

use std::path::{Path, PathBuf};

// The bench is shared with every other milestone's test and none uses all of it.
#[allow(dead_code)]
mod bench;

use bench::plan::Planned;
use config::{Manifest, TriggerCatalogue, TriggerWritten};
use core_model::{
    PrMode, StepId, TriggerResolution, TriggerRuns, TriggerSkipped, TriggerSource, TriggerWhen,
    Ulid, WorkflowId,
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
    let planned = Planned::created_with("tidy the reader", workflow_with(&step, "", &manifest));
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
