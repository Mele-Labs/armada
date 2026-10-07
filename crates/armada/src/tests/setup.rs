//! What this repository's own setup says, checked against what it has to say.
//!
//! **No daemon is started here.** `Setup::at` is a read and a resolve over two
//! files, which is exactly the part of starting Fleet that can be wrong on
//! disk — everything after it needs a port, a store and a process. Whether the
//! five operations answer is asserted in `fleet`'s own suite, over the router,
//! with no socket.
//!
//! # These are the tests that stop the real files rotting
//!
//! `armada.yml` and every definition in `.armada/workflows/` are read by
//! nothing else in the workspace. Without a test over them, a Check renamed in
//! one and not the other is a daemon that refuses to start, discovered by
//! whoever next tried to start it.

use config::{Fault, LoadError, ResolvedCheck, Roster, WorkflowDef, WorkflowSource};

use crate::setup::{Setup, MANIFEST, WORKFLOWS};
use crate::tests::{repository, TempDir};

/// What this machine can run a Drone as, resolved the way `serve` resolves it.
///
/// **Read through [`crate::model_choices`] rather than written out**, so these
/// tests check the shipped definitions against the roster the daemon would
/// actually use — a list typed here would go on passing after the adapter's
/// changed.
pub(super) fn roster() -> Roster {
    Roster::of(crate::model_choices(None).models)
}

/// A repository with an `armada.yml` that declares no Checks, so a workflow
/// gated on nothing resolves against it without also having to write a Check.
pub(super) fn a_repository() -> TempDir {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTUREMANIFEST\n");
    dir
}

/// A minimal, legal, one-step definition — gated on nothing, so it resolves
/// against a Manifest that declares no Checks.
pub(super) fn a_workflow(id: &str) -> String {
    format!(
        "version: 1\nworkflow_id: {id}\nname: {id}\nsteps:\n  - id: only\n    \
         label: \"Only step\"\n    delivers: true\n    advance_gate: auto\n"
    )
}

/// Bug, this repository's one workflow the tests below name by hand. The
/// other six live beside it and are not this file's business.
pub(super) fn bug(setup: &Setup) -> &config::ResolvedWorkflow {
    setup
        .workflows()
        .get(&core_model::WorkflowId::carried(core_model::Ulid::carried(
            "bug",
        )))
        .expect("bug.json declares workflow_id `bug`")
}

/// **The whole claim of this step, over the real files.** Fleet is pointed at a
/// repository and the repository's setup is enough to build a workflow that can
/// be dispatched.
#[test]
fn this_repositorys_own_setup_loads_and_resolves() {
    let setup = match Setup::at(&repository(), TempDir::new().path(), &roster()) {
        Ok(setup) => setup,
        Err(refused) => panic!("{} and {WORKFLOWS} must load:\n{refused}", MANIFEST),
    };

    // Sorted, because `check_names` walks a `BTreeMap` — so the reading order
    // of the file is not the reading order here.
    assert_eq!(
        setup.manifest().check_names(),
        vec![
            "acceptance".to_string(),
            "build".to_string(),
            "hooks_test".to_string(),
            "preview_test".to_string(),
            "sync_mod_test".to_string(),
            "test".to_string(),
            "typecheck".to_string(),
        ],
        "the Checks this workspace is built and tested with — Rust, `acceptance` \
         on its own so a red one names a broken milestone claim rather than a \
         unit test (#1130), the Bridge's packages nobody owns (#200: every \
         Check used to compile Rust), and the merge line's own two Python suites, which no other \
         Check reads. The rest of the Bridge's Checks are in `packages/` and \
         `apps/desktop`, one `armada.yml` each. There is no `clippy` — `[clippy-as-a-check]` in \
         `docs/OPEN.md` says why"
    );
    assert_eq!(bug(&setup).name(), "bug");
    assert_eq!(
        bug(&setup).source(),
        WorkflowSource::Repository,
        "this repository's own file, over the identical one Armada carries"
    );
    let steps: Vec<&str> = bug(&setup)
        .steps()
        .iter()
        .map(|step| step.id().as_str())
        .collect();
    assert_eq!(
        steps,
        vec!["plan", "implement", "handoff"],
        "three steps, which is M1's reduced form of the designed Bug workflow \
         — `implement` carries the test Check itself rather than handing off \
         to a separate `verify` step"
    );
}

/// **The Checks the workflow names are the Checks the Manifest declares**, and
/// the command each resolved to is the command that will run.
///
/// Resolution having succeeded above already proves the names matched; this
/// asserts what they matched *to*, because a Check renamed in one file and left
/// in the other resolves to a command nobody meant.
#[test]
fn each_named_check_resolved_to_the_command_the_manifest_holds() {
    let setup =
        Setup::at(&repository(), TempDir::new().path(), &roster()).expect("a setup that loads");
    let resolved: Vec<(&str, &str, &str)> = bug(&setup)
        .steps()
        .iter()
        .flat_map(|step| step.checks())
        .filter_map(|check| match check {
            ResolvedCheck::ManifestCheck {
                name,
                run,
                manifest_dir,
                ..
            } => Some((manifest_dir.as_str(), name.as_str(), run.as_str())),
            ResolvedCheck::DiffNonempty
            | ResolvedCheck::ArtifactExists { .. }
            | ResolvedCheck::PlanRecorded { .. } => None,
        })
        .collect();
    let root: Vec<(&str, &str)> = resolved
        .iter()
        .filter(|(dir, name, _)| {
            dir.is_empty()
                && *name != "hooks_test"
                && *name != "preview_test"
                && *name != "sync_mod_test"
        })
        .map(|(_, name, run)| (*name, *run))
        .collect();

    // **The root's own Checks, literally, in declaration order, and it is the
    // order they run in.** `WorkflowDef` has no field for sequencing — see
    // `config`'s own test saying so. The root's list is stable: a new surface
    // adds a manifest, never a root Check.
    //
    // `hooks_test` is checked below instead: its command names the agent
    // harness, and this file is under the gate rule that keeps a vendor's name
    // out of everything but the adapters. A `run` gets no shell, so a chain
    // lives in a `package.json` script and the Check names one command.
    assert_eq!(
        root,
        vec![
            ("build", "cargo build --workspace --locked"),
            (
                "test",
                "cargo nextest run --workspace --exclude acceptance --test-threads ${width}",
            ),
            (
                "acceptance",
                "cargo nextest run -p acceptance --test-threads ${width}"
            ),
            ("typecheck", "pnpm typecheck"),
        ]
    );

    // **Every workspace's Checks come from the workspace's own manifest, not
    // from a list here**, so a new surface cannot break this. After the root's,
    // each manifest's Checks appear once each, in manifest-directory order and
    // in the order its own `armada.yml` writes them, each carrying the command
    // that manifest holds for it and its own directory.
    assert!(
        setup.workspaces_refused().is_empty(),
        "every workspace manifest loads: {:?}",
        setup.workspaces_refused()
    );
    let dirs: Vec<&str> = setup.workspaces().iter().map(|one| one.dir()).collect();
    assert!(
        dirs.windows(2).all(|pair| pair[0] < pair[1]),
        "workspaces in directory order: {dirs:?}"
    );
    let expected: Vec<(&str, &str, &str)> = setup
        .workspaces()
        .iter()
        .flat_map(|manifest| {
            manifest.checks_as_written().iter().map(|name| {
                let declared = manifest.check(name).expect("a written Check is declared");
                (manifest.dir(), name.as_str(), declared.run())
            })
        })
        .collect();
    let workspace: Vec<(&str, &str, &str)> = resolved
        .iter()
        .filter(|(dir, ..)| !dir.is_empty())
        .copied()
        .collect();
    assert_eq!(workspace, expected);
    let hooks = resolved
        .iter()
        .find(|(_, name, _)| *name == "hooks_test")
        .expect("the hook suite is declared");
    assert!(hooks.2.ends_with("hooks/test_guard_merge.py"), "{hooks:?}");
    let preview = resolved
        .iter()
        .find(|(_, name, _)| *name == "preview_test")
        .expect("the preview suite is declared");
    assert!(
        preview.2.ends_with("scripts/test_preview.py"),
        "{preview:?}"
    );
}

/// **`every_manifest_check` expands in the order `armada.yml` writes, and that
/// order is the order the gate reports.**
///
/// The seven names the shipped steps spell out are moving out of the workflow
/// files, and until they did, the workflow file was the only place this
/// repository sequenced its gate — the assertion above is that sequence read
/// top to bottom. Expanded alphabetically the same seven come back as
/// `bridge_build, bridge_test, build, …`: the same set, and the two slowest
/// Checks leading.
///
/// **What the order still decides is the report, and where untimed Checks
/// start.** `fleet::checking` starts Checks fastest first by their recorded
/// durations (#1062), and one with none keeps its written position. Each result
/// lands in a slot sized from the declaration, so a Drone reads the same order
/// however they raced.
///
/// **Asserted against the real Manifest and a definition written here**, rather
/// than by editing a shipped file: the claim is about the expansion, and it has
/// to hold before the seven files switch over as well as after.
#[test]
fn gating_on_every_check_runs_them_in_the_order_armada_yml_writes_them() {
    let setup =
        Setup::at(&repository(), TempDir::new().path(), &roster()).expect("a setup that loads");
    let text = "version: 1\nworkflow_id: sweeping\nname: sweeping\nsteps:\n  \
                - id: implement\n    label: Implement\n    evidence: {submitted: {type: diff}}\n    \
                delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      \
                - { type: every_manifest_check }\n      - { type: diff_nonempty }\n";
    let def = WorkflowDef::parse(std::path::Path::new("sweeping.yml"), text, &roster())
        .expect("a step may gate on every declared Check");
    let resolved = config::ResolvedWorkflow::resolve(&def, setup.manifest())
        .expect("every declared Check resolves against the file that declared it");

    let names: Vec<&str> = resolved.steps()[0]
        .checks()
        .iter()
        .filter_map(ResolvedCheck::name)
        .collect();
    assert_eq!(
        names,
        vec![
            "build",
            "test",
            "acceptance",
            "typecheck",
            "hooks_test",
            "preview_test",
            "sync_mod_test",
        ],
        "the order `armada.yml` declares them in, which is the order they answer \
         in — not `check_names`' alphabetical"
    );
    // The same seven, and no eighth: the expansion is the registry and the
    // registry is what `check_names` lists.
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, setup.manifest().check_names());
    // And the declaration survives beside what it came to, which is what a
    // repository declaring no Checks would be left with.
    assert!(resolved.steps()[0].gates_on_every_check());
}

/// **`test` excludes `acceptance`, and `acceptance` runs it on its own** —
/// #1130. A milestone's claim is checked by its own Check, so a red `test`
/// always names a unit test and a red `acceptance` always names a broken
/// claim, never the two folded into one line.
#[test]
fn the_test_check_excludes_acceptance_and_a_separate_check_runs_it() {
    let setup =
        Setup::at(&repository(), TempDir::new().path(), &roster()).expect("a setup that loads");
    let test = setup.manifest().check("test").expect("a `test` Check");
    assert!(
        test.run().contains("--exclude acceptance"),
        "the test Check must not run the acceptance crate: {}",
        test.run()
    );
    let acceptance = setup
        .manifest()
        .check("acceptance")
        .expect("a separate `acceptance` Check");
    assert!(
        acceptance.run().contains("-p acceptance"),
        "the acceptance Check must run the acceptance crate: {}",
        acceptance.run()
    );
}

/// **The designed Bug workflow is refused, and the reason is that M1 is small
/// rather than that the file is wrong.**
///
/// `crates/core-model/domain/workflow-samples/bug.json` is the authority on
/// what Bug becomes: seven steps, a Judge on every gate, and a `review` step
/// that routes `request_changes` back to `fix`. What refuses it is the
/// next deferral: `test_run` is a check type the schema sanctions and M1 has
/// no per-step test invocation for.
///
/// That distinction is why the two `bug.json` files in this repository are not
/// duplicates and must not be reconciled, and it is asserted rather than left
/// to a comment: a refusal that turned into `NotInTheSchema` would mean
/// somebody had changed the designed definition to make it load here.
#[test]
fn the_designed_bug_workflow_is_refused_for_a_reason_a_later_milestone_removes() {
    let designed = repository()
        .join("crates/core-model/domain/workflow-samples")
        .join("bug.json");
    let refused = WorkflowDef::load(&designed, &roster())
        .expect_err("seven steps, and M1 reads a slice of what they declare");
    let LoadError::Refused { refusals, .. } = &refused else {
        panic!("a document that parsed and was refused, not {refused}");
    };
    let deferred = refusals
        .iter()
        .find(|refusal| refusal.key == "steps[0].mechanical_checks[0].type")
        .unwrap_or_else(|| panic!("`test_run` is refused: {refusals:?}"));
    assert!(
        matches!(
            &deferred.fault,
            Fault::NotYetCarried { value, .. } if value == "test_run"
        ),
        "deferred, not wrong: {:?}",
        deferred.fault
    );
}

/// The two definitions are different files with different scope, and nothing
/// reconciles them by accident.
///
/// M1's reduced form has three steps because M1 has no Judge to answer
/// `auto_if_judge_passes` and no verdict to route on, and `implement` carries
/// the test Check itself rather than handing off to a step of its own. The
/// designed one has seven and loops. A change that made the two the same
/// length would mean one of them had been quietly rewritten into the other.
#[test]
fn the_designed_definition_and_m1s_reduced_form_are_not_the_same_workflow() {
    let setup =
        Setup::at(&repository(), TempDir::new().path(), &roster()).expect("a setup that loads");
    assert_eq!(bug(&setup).steps().len(), 3);

    let designed = repository()
        .join("crates/core-model/domain/workflow-samples")
        .join("bug.json");
    let text = std::fs::read_to_string(&designed).expect("the designed definition is readable");
    // Counted rather than parsed, because the definition does not load at M1 —
    // which is the fact the test above is about.
    let steps = text.matches("\"id\":").count();
    assert_eq!(
        steps, 7,
        "repro, root_cause, fix, regression_verify, review, merge, close"
    );
}

/// **The whole point of this step.** A repository may declare more than one
/// workflow, and every one of them loads and is held by its own id.
#[test]
fn two_or_more_workflow_definitions_load_and_are_held_by_their_own_ids() {
    let dir = a_repository();
    dir.write(".armada/workflows/alpha.yml", &a_workflow("alpha"));
    dir.write(".armada/workflows/beta.yml", &a_workflow("beta"));

    let setup = Setup::at(dir.path(), TempDir::new().path(), &roster())
        .expect("two definitions with distinct ids load");
    let held = sources(&setup);
    for id in ["alpha", "beta"] {
        assert!(
            held.contains(&(id, WorkflowSource::Repository)),
            "{id} in {held:?}"
        );
    }
}

/// **The whole daemon-start path, over a workflow that names no Check.** The
/// test above resolves one definition against the real Manifest; this walks
/// `Setup::at` — read the Manifest, read the definitions beside it, resolve
/// each — which is what a repository whose workflows have been switched over
/// actually goes through.
///
/// The Manifest here declares its Checks in an order the alphabet does not
/// agree with, so a resolution that quietly sorted them would come back
/// `apple, build, zebra` and fail here rather than in a Job.
#[test]
fn a_repository_whose_workflow_names_no_check_starts_and_keeps_its_order() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  zebra:\n    run: run zebra\n  \
         build:\n    run: run build\n  apple:\n    run: run apple\n",
    );
    dir.write(
        ".armada/workflows/sweeping.yml",
        "version: 1\nworkflow_id: sweeping\nname: sweeping\nsteps:\n  \
         - id: only\n    label: \"Only step\"\n    evidence: {submitted: {type: diff}}\n    delivers: true\n    \
         advance_gate: auto\n    mechanical_checks:\n      - { type: every_manifest_check }\n",
    );

    let setup = Setup::at(dir.path(), TempDir::new().path(), &roster())
        .expect("a repository that gates on all of them");
    let workflow = setup
        .workflows()
        .values()
        .find(|workflow| workflow.id().as_str() == "sweeping")
        .expect("the repository's one definition");
    let ran: Vec<(&str, &str)> = workflow.steps()[0]
        .checks()
        .iter()
        .filter_map(|check| Some((check.name()?, check.run()?)))
        .collect();
    assert_eq!(
        ran,
        vec![
            ("zebra", "run zebra"),
            ("build", "run build"),
            ("apple", "run apple"),
        ],
        "the Manifest's order, with each Check's own command lifted in"
    );
}

/// **Two files naming the same `workflow_id` in one place are both left out,
/// and named together**, and the repository still starts: a duplicate is not
/// grounds for refusing it, and nothing picks between the two.
#[test]
fn a_duplicate_workflow_id_across_two_files_leaves_both_out_and_the_repository_starts() {
    let dir = a_repository();
    dir.write(".armada/workflows/first.yml", &a_workflow("shared"));
    dir.write(".armada/workflows/second.yml", &a_workflow("shared"));

    let setup = Setup::at(dir.path(), TempDir::new().path(), &roster())
        .expect("a duplicate does not refuse the repository");
    assert!(!setup.workflows().keys().any(|id| id.as_str() == "shared"));
    let said: Vec<String> = setup.left_out().iter().map(ToString::to_string).collect();
    let [said] = said.as_slice() else {
        panic!("one sentence for the pair: {said:?}");
    };
    assert!(said.contains("first.yml"), "{said}");
    assert!(said.contains("second.yml"), "{said}");
    assert!(said.contains("shared"), "{said}");
}

/// Where each id a Setup holds came from, sorted by id.
pub(super) fn sources(setup: &Setup) -> Vec<(&str, WorkflowSource)> {
    setup
        .workflows()
        .iter()
        .map(|(id, workflow)| (id.as_str(), workflow.source()))
        .collect()
}

/// **A repository nobody set up dispatches on what Armada carries** — with no
/// `.armada/workflows/`, and with an empty one, which until #425 was refused.
#[test]
fn a_repository_with_no_workflows_of_its_own_runs_on_what_armada_carries() {
    let dir = a_repository();
    for made in [false, true] {
        if made {
            std::fs::create_dir_all(dir.path().join(WORKFLOWS)).expect("the empty directory");
        }
        let setup = Setup::at(dir.path(), TempDir::new().path(), &roster())
            .unwrap_or_else(|why| panic!("no workflows of its own, directory made {made}: {why}"));
        let carried = [
            "bug",
            "code_review",
            "design_plan",
            "epic",
            "feature",
            "prototype",
            "refactor",
            "revert",
        ];
        assert_eq!(
            sources(&setup),
            carried.map(|id| (id, WorkflowSource::Armada)),
            "directory made: {made}"
        );
    }
}
