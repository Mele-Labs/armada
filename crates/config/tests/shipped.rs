//! Every workflow definition this repository ships parses, and resolves
//! against this repository's own `armada.yml`.
//!
//! An integration test rather than a unit one because the subject is the files
//! in `.armada/workflows/` and the file beside them, not the parser: a
//! definition that stops loading is a Fleet that cannot dispatch, and nothing
//! else in the workspace reads them.
//!
//! The four ways a definition has gone wrong so far were all silent until a Job
//! hit them — a key the parser defers, a gate disagreeing with its judge checks,
//! a `question` where the parser reads `criteria[]`, and a Judge asked something
//! on a step that produces nothing.
//!
//! # The fifth way, which had no test until #200
//!
//! **Parsing is not resolving.** A step may name a Check spelled correctly,
//! shaped correctly and declared nowhere, and [`config::WorkflowDef::parse`]
//! takes it — the cross-file question is [`config::ResolvedWorkflow`]'s and is
//! asked at dispatch. So the one edit this pair invites, adding a Check to a
//! step and forgetting the Manifest, was caught by nothing here and by
//! everything at the worktree.
//!
//! The second test below asks it. It is deliberately the *shipped* Manifest and
//! not a fixture: a fixture declaring `build` and `test` would keep passing
//! while `armada.yml` lost them.

use std::path::{Path, PathBuf};

use config::Roster;

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What this machine can run a Drone as, read from the adapter rather than
/// written out here.
///
/// **A list typed into this file would defeat the test it is part of.** A
/// shipped step naming a model the adapter does not offer would keep parsing
/// against the local copy and fail at spawn, where the Job already has a
/// worktree — which is the whole failure `config::Roster` exists to move
/// earlier. So the roster the parse is checked against is the roster the
/// running daemon resolves, and `adapters` is a dev-dependency for this one
/// call.
fn roster() -> Roster {
    Roster::of(adapters::HeadlessAgent::models())
}

/// Every shipped definition, as `(path, text)`, sorted so a failure names the
/// same file on every machine.
fn shipped() -> Vec<(PathBuf, String)> {
    let dir = root().join(".armada/workflows");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("the shipped definitions are there")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            let name = path.file_name().expect("a file").to_string_lossy();
            !config::REPOSITORY_ONLY.contains(&name.as_ref())
        })
        .collect();
    found.sort();
    found
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("a readable definition");
            (path, text)
        })
        .collect()
}

#[test]
fn every_shipped_workflow_definition_parses() {
    let mut seen = 0;
    for (path, text) in shipped() {
        if let Err(why) = config::WorkflowDef::parse(&path, &text, &roster()) {
            panic!("{} is refused:\n{why}", path.display());
        }
        seen += 1;
    }
    assert!(seen >= 9, "nine workflows ship, and {seen} were read");
}

/// **Which shipped workflows send their work out, read off the files.**
///
/// Five of the nine produce a diff somebody merges and four produce something
/// somebody reads. Until a workflow could say so, all of them pushed a branch and
/// opened a pull request for whatever was in the worktree — and the only thing
/// standing between a design document and a pull request was this repository's
/// `.gitignore` line over `.armada/`.
///
/// Asserted off the files rather than trusted to their headers, because this is
/// the one property of the set that a single edit to one file can break for
/// everybody: a workflow that starts delivering opens pull requests for
/// documents, and one that stops leaves a Job's diff on a branch nobody sees.
#[test]
fn five_shipped_workflows_send_their_work_out_and_four_do_not() {
    let mut delivering: Vec<(String, String)> = Vec::new();
    let mut silent: Vec<String> = Vec::new();
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        let sends: Vec<&config::Step> = def.steps().iter().filter(|s| s.delivers()).collect();
        match sends.as_slice() {
            [] => silent.push(def.id().as_str().to_string()),
            [step] => delivering.push((
                def.id().as_str().to_string(),
                step.id().as_str().to_string(),
            )),
            _ => panic!("{} sends its work out more than once", path.display()),
        }
    }
    delivering.sort();
    silent.sort();
    assert_eq!(
        delivering,
        vec![
            (String::from("bug"), String::from("deliver")),
            (String::from("create-workflow"), String::from("build")),
            (String::from("feature"), String::from("deliver")),
            (String::from("refactor"), String::from("handoff")),
            (String::from("revert"), String::from("handoff")),
        ],
        "the five that produce a diff deliver, each on its last step",
    );
    assert_eq!(
        silent,
        vec!["design_plan", "orchestrate", "prototype", "uat"],
        "and the four that produce something read, or written to a sheet, rather than merged deliver nothing",
    );
}

/// **Every shipped step is in the phase the owner drew it in**: the two that
/// declare one, and every other by what `delivers` says.
#[test]
fn each_shipped_step_reads_in_its_phase() {
    use core_model::StepPhase::{Delivery, Work};
    let declared = [
        ("orchestrate", "summarise"),
        ("uat", "review"),
        ("uat", "writeback"),
    ];
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        for step in def.steps() {
            let named = (def.id().as_str(), step.id().as_str());
            let expected = match declared.contains(&named) {
                true => Some(Delivery),
                false => None,
            };
            assert_eq!(step.phase(), expected, "{named:?}");
            let read = core_model::StepPhase::of(step.phase(), step.delivers());
            let drawn = if declared.contains(&named) || step.delivers() {
                Delivery
            } else {
                Work
            };
            assert_eq!(read, drawn, "{named:?}");
        }
    }
}

/// **Every shipped workflow lands in what it is for**: the three that declare
/// it, and every other by whether a step delivers.
#[test]
fn each_shipped_workflow_lands_in_what_it_is_for() {
    use core_model::Lands::{Document, PullRequest, Report, Spike};
    let mut lands: Vec<(String, core_model::Lands)> = shipped()
        .into_iter()
        .map(|(path, text)| {
            let def = config::WorkflowDef::parse(&path, &text, &roster())
                .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
            (def.id().as_str().to_string(), def.lands())
        })
        .collect();
    lands.sort_by(|a, b| a.0.cmp(&b.0));
    let named: Vec<(&str, core_model::Lands)> = lands.iter().map(|(id, lands)| (id.as_str(), *lands)).collect();
    assert_eq!(
        named,
        vec![
            ("bug", PullRequest),
            ("create-workflow", PullRequest),
            ("design_plan", Document),
            ("feature", PullRequest),
            ("orchestrate", Report),
            ("prototype", Spike),
            ("refactor", PullRequest),
            ("revert", PullRequest),
            ("uat", Document),
        ]
    );
}

/// A phase outside the three is refused rather than drawn in the lane its
/// fallback would pick.
#[test]
fn a_phase_outside_the_three_is_refused() {
    let step = |phase: &str| {
        format!(
            "version: 1\nworkflow_id: phased\nname: phased\n\
             steps:\n  - id: one\n    label: \"One\"\n    phase: {phase}\n    \
             delivers: false\n    advance_gate: auto\n"
        )
    };
    let def = config::WorkflowDef::parse(Path::new("phased.yml"), &step("setup"), &roster())
        .expect("setup is a phase");
    assert_eq!(def.steps()[0].phase(), Some(core_model::StepPhase::Setup));
    let refused = config::WorkflowDef::parse(Path::new("phased.yml"), &step("shipping"), &roster())
        .expect_err("shipping is not a phase");
    assert!(refused.to_string().contains("delivery"), "{refused}");
}

/// **This repository's own `armada.yml` loads**, which nothing asked until now.
///
/// The header above has claimed since `#200` that these definitions resolve
/// against the shipped Manifest, and no test here read that file — so a typo in
/// it was found by starting a daemon, and by nothing before that. `#414` gave it
/// a `drone:` section and made the gap worth closing rather than only worth
/// naming.
///
/// **The value is asserted and not only the parse.** A `quiet_after_seconds`
/// silently dropped by a parser that stopped reading the key would leave this
/// file loading exactly as well as before, and every Drone here back on a
/// threshold shorter than one of its own commands.
#[test]
fn this_repositorys_own_manifest_loads_and_states_its_patience() {
    let path = root().join("armada.yml");
    let manifest = config::Manifest::load(&path)
        .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
    assert_eq!(
        manifest.quiet_after_seconds(),
        Some(300),
        "the repository's own patience is what its `drone:` section writes"
    );
    // Nothing here has a reason to want more or fewer nudges than Fleet's, and
    // the two halves fall back separately — so an absent one is the assertion.
    assert_eq!(manifest.poke_limit(), None);
}

/// **Every shipped step says what it does.** Bridge draws the line at the
/// approval gate, where a person meets a workflow's steps for the first time.
#[test]
fn every_shipped_step_says_what_it_does() {
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        for step in def.steps() {
            assert!(
                step.about().is_some_and(|about| !about.trim().is_empty()),
                "{} step `{}` has no `about`",
                path.display(),
                step.id().as_str(),
            );
        }
    }
}

/// **The three code workflows tell the agent what to do at each step.** The text
/// is what the per-step skills said, held in the step that uses it.
#[test]
fn bug_feature_and_refactor_carry_instructions_on_every_step() {
    for (path, text) in shipped() {
        let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default().to_string();
        if !["bug", "feature", "refactor"].contains(&name.as_str()) {
            continue;
        }
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        for step in def.steps() {
            assert!(
                step.instructions().is_some_and(|words| !words.trim().is_empty()),
                "{} step `{}` has no `instructions`",
                path.display(),
                step.id().as_str(),
            );
        }
    }
}

/// **Bug and feature ask how the work is delivered at their first step**, and nothing else does.
#[test]
fn only_the_first_step_of_bug_and_feature_asks_how_the_work_is_delivered() {
    for (path, text) in shipped() {
        let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default().to_string();
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        for (at, step) in def.steps().iter().enumerate() {
            let expected = ["bug", "feature"].contains(&name.as_str()) && at == 0;
            assert_eq!(step.asks_delivery(), expected, "{name} step `{}`", step.id().as_str());
        }
    }
}

/// **Bug and feature each end in a `deliver` step**, the one that delivers.
#[test]
fn bug_and_feature_each_end_in_a_deliver_step() {
    for (path, text) in shipped() {
        let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default().to_string();
        if !["bug", "feature"].contains(&name.as_str()) {
            continue;
        }
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        let last = def.steps().last().expect("a workflow has steps");
        assert_eq!(last.id().as_str(), "deliver", "{name} ends in `deliver`");
        assert!(last.delivers(), "{name}'s `deliver` step delivers");
    }
}

/// **The steps the owner settled on 9 Nov 2026**, in order, and the gate that ends each one's plan group.
/// A step with no gate is the one that ends the Job by itself: `uat`'s `writeback`.
#[test]
fn each_shipped_workflow_runs_the_steps_the_owner_settled() {
    use core_model::StepGate::{self, DocumentApproved, JobsEnded, Merged, PlanApproved, WalkApproved, YouApproved};
    let expected: [(&str, Vec<(&str, StepGate)>); 9] = [
        ("bug", vec![("find_the_cause", PlanApproved), ("fix", WalkApproved), ("deliver", Merged)]),
        ("create-workflow", vec![("quiz", PlanApproved), ("propose", YouApproved), ("build", Merged)]),
        ("design-plan", vec![("draft", DocumentApproved)]),
        ("feature", vec![("propose", PlanApproved), ("walk", WalkApproved), ("deliver", Merged)]),
        ("orchestrate", vec![("plan", PlanApproved), ("oversee", JobsEnded), ("summarise", DocumentApproved)]),
        ("prototype", vec![("frame", PlanApproved), ("build", WalkApproved), ("write_up", DocumentApproved)]),
        ("refactor", vec![("plan", PlanApproved), ("implement", DocumentApproved), ("handoff", Merged)]),
        ("revert", vec![("plan", PlanApproved), ("handoff", Merged)]),
        ("uat", vec![("review", DocumentApproved), ("writeback", StepGate::None)]),
    ];
    let shipped = shipped();
    assert_eq!(shipped.len(), expected.len(), "every shipped workflow is listed");
    for (name, steps) in expected {
        let (path, text) = shipped
            .iter()
            .find(|(path, _)| path.file_stem().and_then(|stem| stem.to_str()) == Some(name))
            .unwrap_or_else(|| panic!("{name} is shipped"));
        let def = config::WorkflowDef::parse(path, text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        let read: Vec<(&str, StepGate)> = def.steps().iter().map(|step| (step.id().as_str(), step.gate())).collect();
        assert_eq!(read, steps, "{name}");
    }
}

/// **Every step of every shipped workflow declares a gate**, except `uat`'s `writeback`, which ends the
/// Job without one. A step that leaves the key out is a plan group nothing Fleet sees can end.
#[test]
fn every_shipped_step_declares_a_gate_but_uat_writeback() {
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        for step in def.steps() {
            let exempt = def.id().as_str() == "uat" && step.id().as_str() == "writeback";
            assert_eq!(
                step.gate() == core_model::StepGate::None,
                exempt,
                "{} step `{}`",
                path.display(),
                step.id().as_str(),
            );
        }
    }
}

/// **Oversee ends when every Job the parent started has merged or ended** (9 Nov 2026), a fact Fleet
/// reads off the Jobs' own records and not off what the agent says; no other step of the workflow waits on one.
#[test]
fn orchestrate_holds_oversee_until_every_job_it_started_has_ended() {
    let (path, text) = shipped()
        .into_iter()
        .find(|(path, _)| path.file_stem().and_then(|stem| stem.to_str()) == Some("orchestrate"))
        .expect("orchestrate is shipped");
    let def = config::WorkflowDef::parse(&path, &text, &roster()).unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
    let waiting: Vec<&str> =
        def.steps().iter().filter(|step| step.gate() == core_model::StepGate::JobsEnded).map(|step| step.id().as_str()).collect();
    assert_eq!(waiting, vec!["oversee"]);
}

/// **Tests are written in `deliver`**: no step before it asks for one, and it does.
#[test]
fn no_step_before_deliver_is_told_to_write_a_test_and_deliver_is() {
    for name in ["feature", "bug"] {
        let (path, text) = shipped()
            .into_iter()
            .find(|(path, _)| path.file_stem().and_then(|stem| stem.to_str()) == Some(name))
            .unwrap_or_else(|| panic!("{name} is shipped"));
        let def = config::WorkflowDef::parse(&path, &text, &roster()).expect("loads");
        for step in def.steps() {
            let words = step.instructions().unwrap_or_default().to_lowercase();
            let asks_for_tests = words.contains("write the test") || words.contains("write a test") || words.contains("each with the test");
            if step.id().as_str() == "deliver" {
                assert!(asks_for_tests, "{name} step `deliver` is where the tests are written");
            } else {
                assert!(!asks_for_tests, "{name} step `{}` asks for tests before deliver", step.id().as_str());
            }
        }
    }
}

/// **Only `uat` carries no code**: a Job on it is never asked for a branch, slot or pull request, so
/// a workflow that read as carrying none by mistake would stop a code Job's slot lease and notices.
/// It also names no step that delivers, which is what keeps it from opening a pull request.
#[test]
fn only_uat_carries_no_code_and_none_of_its_steps_deliver() {
    let mut carrying_none: Vec<String> = Vec::new();
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        if def.carries_no_code() {
            assert!(def.steps().iter().all(|step| !step.delivers()), "{} delivers", path.display());
            carrying_none.push(def.id().as_str().to_string());
        }
    }
    assert_eq!(carrying_none, vec!["uat"]);
}
