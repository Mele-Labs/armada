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
    assert!(seen >= 8, "eight workflows ship, and {seen} were read");
}

/// **Which shipped workflows send their work out, read off the files.**
///
/// Four of the eight produce a diff somebody merges and four produce something
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
fn four_shipped_workflows_send_their_work_out_and_four_do_not() {
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
            (String::from("feature"), String::from("deliver")),
            (String::from("refactor"), String::from("handoff")),
            (String::from("revert"), String::from("handoff")),
        ],
        "the four that produce a diff deliver, each on its last step",
    );
    assert_eq!(
        silent,
        vec!["code_review", "design_plan", "orchestrate", "prototype"],
        "and the four that produce something read rather than merged deliver nothing",
    );
}

/// **Every shipped step is in the phase the owner drew it in**: the two that
/// declare one, and every other by what `delivers` says.
#[test]
fn each_shipped_step_reads_in_its_phase() {
    use core_model::StepPhase::{Delivery, Work};
    let declared = [("design_plan", "present"), ("orchestrate", "summarise")];
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
            ("code_review", Report),
            ("design_plan", Document),
            ("feature", PullRequest),
            ("orchestrate", Report),
            ("prototype", Spike),
            ("refactor", PullRequest),
            ("revert", PullRequest),
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

/// **A pull request arrives as a link and nothing else, so `code_review` has
/// to say it is the one for a link to one.** `#1379`.
///
/// A Studio dispatches a Link naming a pull request as its address, and Fleet
/// resolves no text behind one — `adapters::IssueLookup` reads an issue link
/// and no other shape — so the proposer is choosing off a URL and eight step
/// lists. `Read the diff -> Assess -> Deliver the review` is this workflow's
/// own vocabulary and says nothing about whether a request is this kind of
/// work, which is exactly what `#424` measured.
#[test]
fn code_review_says_it_is_for_a_pull_request_in_a_requesters_words() {
    let what_for = says_what_it_is_for("code-review.json");
    assert!(what_for.contains("pull request"), "{what_for}");
    for steps_word in ["diff", "assess", "deliver"] {
        assert!(
            !what_for.to_lowercase().contains(steps_word),
            "`{steps_word}` is the steps' word, not a requester's: {what_for}"
        );
    }
}

/// The `for_requests` line one shipped definition declares, off the file.
fn says_what_it_is_for(file: &str) -> String {
    let path = root().join(".armada/workflows").join(file);
    let text = std::fs::read_to_string(&path).expect("a readable definition");
    let def = config::WorkflowDef::parse(&path, &text, &roster())
        .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
    def.for_requests()
        .unwrap_or_else(|| panic!("{file} says what requests it is for"))
        .to_string()
}

/// **What the proposer is handed for each of the three addresses a Studio
/// dispatches**, off the shipped catalogue rather than a fixture — `#1379`.
///
/// It measures everything up to the model call and nothing past it: which
/// workflow is chosen is a model's answer, and this file spawns none. What it
/// holds is the half that was wrong before — that the catalogue offers a line
/// a person asking about a pull request or a milestone would have used, and
/// that no other line offers either word to be matched on by accident. Every
/// shipped definition has declared a line since 1 Oct 2026, so the guard is on
/// the words and no longer on which two say anything.
#[test]
fn a_forge_link_is_offered_a_line_written_in_the_words_somebody_asking_would_use() {
    let declared: Vec<(String, Option<String>)> = shipped()
        .into_iter()
        .map(|(path, text)| {
            let def = config::WorkflowDef::parse(&path, &text, &roster())
                .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
            (
                def.id().as_str().to_string(),
                def.for_requests().map(str::to_string),
            )
        })
        .collect();
    for (word, owner) in [("pull request", "code_review")] {
        let saying: Vec<&str> = declared
            .iter()
            .filter(|(_, line)| {
                line.as_deref()
                    .is_some_and(|line| line.to_lowercase().contains(word))
            })
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(
            saying,
            [owner],
            "only the workflow a bare forge link has to reach says `{word}`"
        );
    }
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

/// **The steps the owner chose on 27 Oct 2026**, in order, which of them walk, and what ends each before
/// the plan: Propose ends on an approval, never a reply (28 Oct 2026). The decision record "A feature Job
/// walks before it tests".
#[test]
fn feature_and_bug_run_the_steps_the_owner_chose() {
    use core_model::StepEnd::{DraftApproved, IdeaPicked, SketchApproved, WalkApproved};
    let steps_of = |name: &str| -> Vec<(String, bool, Vec<core_model::StepEnd>)> {
        let (path, text) = shipped()
            .into_iter()
            .find(|(path, _)| path.file_stem().and_then(|stem| stem.to_str()) == Some(name))
            .unwrap_or_else(|| panic!("{name} is shipped"));
        config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()))
            .steps()
            .iter()
            .map(|step| (step.id().as_str().to_string(), step.walks(), step.ends_on().to_vec()))
            .collect()
    };
    let expect = |rows: &[(&str, bool, &[core_model::StepEnd])]| -> Vec<(String, bool, Vec<core_model::StepEnd>)> {
        rows.iter().map(|(id, walks, ends)| (id.to_string(), *walks, ends.to_vec())).collect()
    };
    assert_eq!(
        steps_of("feature"),
        expect(&[
            ("propose", false, &[WalkApproved, SketchApproved, DraftApproved, IdeaPicked]),
            ("discuss", false, &[]),
            ("approve_plan", false, &[]),
            ("walk", true, &[]),
            ("refine", false, &[]),
            ("deliver", false, &[]),
        ])
    );
    assert_eq!(
        steps_of("bug"),
        expect(&[
            ("find_the_cause", false, &[]),
            ("fix", false, &[]),
            ("short_walk", true, &[]),
            ("tests", false, &[]),
            ("deliver", false, &[]),
        ])
    );
}

/// **Oversee holds until every Job the parent started has landed or ended** (29 Oct 2026), read off the
/// Jobs' lifecycle instances and not off what the agent says; no other step of the workflow waits on one.
#[test]
fn orchestrate_holds_oversee_until_every_job_it_started_has_landed() {
    let (path, text) = shipped()
        .into_iter()
        .find(|(path, _)| path.file_stem().and_then(|stem| stem.to_str()) == Some("orchestrate"))
        .expect("orchestrate is shipped");
    let def = config::WorkflowDef::parse(&path, &text, &roster()).unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
    let ends: Vec<(&str, Vec<String>)> =
        def.steps().iter().map(|step| (step.id().as_str(), step.ends_on().iter().map(|end| end.as_wire().into_owned()).collect())).collect();
    let expected = ["assess", "plan", "dispatch", "oversee", "summarise"];
    assert_eq!(ends.iter().map(|(id, _)| *id).collect::<Vec<_>>(), expected);
    for (id, ends) in &ends {
        let want: &[&str] = if *id == "oversee" { &["job: landed"] } else { &[] };
        assert_eq!(ends, want, "{id}");
    }
}

/// **Tests come after the walk**: no step before it, nor the walk itself, asks for a test, and the step
/// after it does.
#[test]
fn no_step_before_the_walk_is_told_to_write_a_test_and_the_one_after_is() {
    for (name, after) in [("feature", "refine"), ("bug", "tests")] {
        let (path, text) = shipped()
            .into_iter()
            .find(|(path, _)| path.file_stem().and_then(|stem| stem.to_str()) == Some(name))
            .unwrap_or_else(|| panic!("{name} is shipped"));
        let def = config::WorkflowDef::parse(&path, &text, &roster()).expect("loads");
        let walk_at = def.steps().iter().position(|step| step.walks()).expect("a walk step");
        for (at, step) in def.steps().iter().enumerate() {
            let words = step.instructions().unwrap_or_default().to_lowercase();
            let asks_for_tests = words.contains("write the test") || words.contains("write a test") || words.contains("each with the test");
            let sentence_forbidding = words.contains("write no test") || words.contains("no tests");
            if at <= walk_at && !sentence_forbidding {
                assert!(!asks_for_tests, "{name} step `{}` asks for tests before the walk is approved", step.id().as_str());
            }
            if step.id().as_str() == after {
                assert!(asks_for_tests, "{name} step `{after}` is where the tests are written");
            }
        }
    }
}
