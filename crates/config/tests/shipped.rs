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
    assert!(seen >= 7, "seven workflows ship, and {seen} were read");
}

/// **One shipped step may create Jobs, and this names which.**
///
/// This assertion read `no_shipped_workflow_grants_the_dispatch_tool_yet` until
/// `epic.json` was written, and the sentence under it said what was missing: the
/// grant, the tool and the loop all existed and no definition used any of them.
/// One does now, so the claim inverts rather than retiring — a set of exactly
/// one, spelled out, is what makes a *second* step acquiring the ability to
/// create Jobs a failing test rather than a Drone with an extra tool.
///
/// **Why `epic.plan` alone may.** It is the one step in the repository whose
/// product is other Jobs. Every other shipped step produces a diff, a note or a
/// document that a person or a Judge reads, and a wrong one costs a refusal; a
/// wrong dispatch costs Drones that run and spend. What makes it safe to grant
/// there is that the Jobs it creates wait at `awaiting_approval` and the step
/// is `human_always`: a person reads every one of them at its gate, and
/// `approve_wave` releases them together (spike 022, slice 6). See the file's
/// own header.
///
/// The pair is asserted rather than the flag, because "the epic workflow grants
/// it" and "the epic workflow's dispatching step grants it" are different
/// claims, and the second is the one the design makes.
#[test]
fn epic_s_plan_is_the_only_shipped_step_that_may_create_jobs() {
    let mut granted: Vec<(String, String)> = Vec::new();
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        for step in def.steps() {
            if step.may_dispatch_jobs() {
                let file = path
                    .file_name()
                    .expect("a shipped definition is a file")
                    .to_string_lossy()
                    .to_string();
                granted.push((file, step.id().as_str().to_string()));
            }
        }
    }
    assert_eq!(
        granted,
        vec![("epic.json".to_string(), "plan".to_string())],
        "exactly one shipped step creates Jobs, and it is the epic's plan",
    );
}

/// **Which shipped workflows send their work out, read off the files.**
///
/// Four of the eight produce a diff somebody merges and four produce something
/// somebody reads. Until a workflow could say so, all eight pushed a branch and
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
            (String::from("bug"), String::from("handoff")),
            (String::from("feature"), String::from("handoff")),
            (String::from("refactor"), String::from("handoff")),
            (String::from("revert"), String::from("handoff")),
        ],
        "the four that produce a diff deliver, each on its last step",
    );
    assert_eq!(
        silent,
        vec!["code_review", "design_plan", "epic", "prototype"],
        "and the four that produce something read rather than merged deliver nothing",
    );
}

/// **The step that creates the Jobs is the one a person answers.** Since slice
/// 6 a Job it creates waits at `awaiting_approval`, so the gate is no longer a
/// person approving a spend that already happened: it is a person reading the
/// Jobs themselves before any of them runs, and releasing them in one press.
/// Until slice 6 the grant sat on the step after the gate, because a child
/// entered `queued` and spent at once.
///
/// Asserted off the file rather than trusted to its header, because the two keys
/// are one intent and the parser now refuses them apart.
#[test]
fn the_step_that_creates_the_epic_s_jobs_is_the_one_a_person_answers() {
    let path = root().join(".armada/workflows/epic.json");
    let text = std::fs::read_to_string(&path).expect("a readable definition");
    let def = config::WorkflowDef::parse(&path, &text, &roster())
        .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
    let step = def
        .steps()
        .iter()
        .find(|step| step.may_dispatch_jobs())
        .expect("the epic dispatches somewhere");
    assert_eq!(
        step.advance_gate(),
        config::AdvanceGate::HumanAlways,
        "`{}` creates Jobs, and a person has to answer it",
        step.id().as_str(),
    );
}

/// The key is real on any definition, not only on the one that ships with it.
/// **Parsed rather than asserted against `epic.json`**, which is the file the
/// two tests above read: this one is about the parser taking the key wherever it
/// is written, and reading it off the same file would make three assertions of
/// one file's contents and none of the language.
#[test]
fn a_definition_may_grant_the_dispatch_tool() {
    let text = "version: 1\nworkflow_id: grants\nname: grants\nstructure: linear\n\
                steps:\n  - id: split\n    label: \"Split\"\n    \
                evidence: {submitted: {type: facts_note}}\n    may_dispatch_jobs: true\n    \
                delivers: false\n    advance_gate: human_always\n";
    let def = config::WorkflowDef::parse(Path::new("grants.yml"), text, &roster())
        .expect("a definition may say a step creates Jobs");
    assert!(def.steps()[0].may_dispatch_jobs());
}

/// **A step that creates Jobs and asks nobody is refused.** What it creates
/// waits for a person at its gate, so a step gated any other way would advance
/// past Jobs nothing can release.
#[test]
fn a_dispatch_grant_on_a_step_nobody_answers_is_refused() {
    let text = "version: 1\nworkflow_id: grants\nname: grants\nstructure: linear\n\
                steps:\n  - id: split\n    label: \"Split\"\n    \
                evidence: {submitted: {type: facts_note}}\n    may_dispatch_jobs: true\n    \
                delivers: false\n    advance_gate: auto\n";
    let refused = config::WorkflowDef::parse(Path::new("grants.yml"), text, &roster())
        .expect_err("Jobs nobody can release");
    assert!(refused.to_string().contains("a person"), "{refused}");
}

/// A value that is not a boolean is refused rather than read as `false`. A step
/// written to create Jobs that silently cannot is a Job that goes quiet, which
/// is the hardest failure here to see.
#[test]
fn a_dispatch_grant_that_is_not_a_boolean_is_refused() {
    let text = "version: 1\nworkflow_id: grants\nname: grants\nstructure: linear\n\
                steps:\n  - id: split\n    label: \"Split\"\n    \
                evidence: {submitted: {type: facts_note}}\n    may_dispatch_jobs: dispatches\n    \
                delivers: false\n    advance_gate: auto\n";
    assert!(config::WorkflowDef::parse(Path::new("grants.yml"), text, &roster()).is_err());
}

/// **Every shipped step is in the phase the owner drew it in**: the two that
/// declare one, and every other by what `delivers` says.
#[test]
fn each_shipped_step_reads_in_its_phase() {
    use core_model::StepPhase::{Delivery, Work};
    let declared = [("design_plan", "present"), ("epic", "roll_up")];
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

/// A phase outside the three is refused rather than drawn in the lane its
/// fallback would pick.
#[test]
fn a_phase_outside_the_three_is_refused() {
    let step = |phase: &str| {
        format!(
            "version: 1\nworkflow_id: phased\nname: phased\nstructure: linear\n\
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

/// **The workflow that runs milestones says so, in the words of somebody
/// asking for one.** #424: a request to finish a milestone was declined with
/// `epic` on the list, because nothing the proposer was shown about it was a
/// word the request used.
///
/// Asserted off the file, and against the steps' vocabulary as well as for the
/// requester's: a line that restates `Plan the wave -> Dispatch the wave` is
/// the defect at greater length. `code_review` is asserted beside it since
/// `#1379`, for the same reason and against its own steps' words. A definition
/// that declares nothing is still legal; every one shipped here declares a
/// line since 1 Oct 2026, which `fleet`'s `tests::proposing` holds.
#[test]
fn the_epic_says_it_is_for_a_milestone_in_a_requesters_words() {
    let what_for = says_what_it_is_for("epic.json");
    assert!(what_for.contains("milestone"), "{what_for}");
    for steps_word in ["wave", "roll up", "dispatch"] {
        assert!(
            !what_for.to_lowercase().contains(steps_word),
            "`{steps_word}` is the steps' word, not a requester's: {what_for}"
        );
    }
}

/// **A pull request arrives as a link and nothing else, so `code_review` has
/// to say it is the one for a link to one.** `#1379`.
///
/// A Studio dispatches a Link naming a pull request as its address, and Fleet
/// resolves no text behind one — `adapters::IssueLookup` reads an issue link
/// and no other shape — so the proposer is choosing off a URL and eight step
/// lists. `Read the diff -> Assess -> Deliver the review` is this workflow's
/// own vocabulary and says nothing about whether a request is this kind of
/// work, which is exactly what `#424` measured about `epic` above.
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
    for (word, owner) in [("pull request", "code_review"), ("milestone", "epic")] {
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
