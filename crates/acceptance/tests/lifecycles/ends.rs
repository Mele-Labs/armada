//! A step ends on a lifecycle state, the agent is stopped while it waits, and Fleet wakes it on the
//! transition. **Nothing is spawned and nothing is read from a forge**: the instances and the moves
//! the readers keep are values, and `fleet::lifecycles::ends` hands them to the rules the rail draws.

use core_model::{FleetFact, ForgeFact, HeldInstances, Lifecycle, Observed};
use fleet::lifecycles::ends;
use fleet::lifecycles::holding::{read_instance, Transition};

use crate::bench::lifecycles::{at, taken_on, the_jobs_copy};

/// Plan, Review (ends as `ends` says), Close out.
fn feature(ends: &str) -> config::ResolvedWorkflow {
    let text = format!(
        "version: 1\nworkflow_id: feature\nname: Feature\nsteps:\n  \
        - id: plan\n    label: \"Plan the change\"\n    evidence: {{submitted: {{type: plan}}}}\n    \
        mechanical_checks:\n      - {{ type: plan_recorded, min_tasks: 1 }}\n    delivers: false\n    advance_gate: auto\n  \
        - id: review\n    label: \"Review\"\n    instructions: \"Answer the review.\"\n    ends_on:\n      - \"{ends}\"\n    \
        delivers: false\n    advance_gate: auto\n  \
        - id: close\n    label: \"Close out\"\n    instructions: \"Write the summary.\"\n    delivers: false\n    advance_gate: auto\n"
    );
    let def = config::WorkflowDef::parse(std::path::Path::new("feature.yml"), &text, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("the workflow did not parse: {refused}"));
    let manifest = config::Manifest::parse(std::path::Path::new("armada.yml"), "version: 1\nid: 01FIXTUREMANIFEST\n").expect("a manifest");
    config::ResolvedWorkflow::resolve(&def, &manifest).unwrap_or_else(|refused| panic!("the workflow did not resolve: {refused}"))
}

fn seen(held: &mut HeldInstances, moves: &mut Vec<Transition>, copy: &[Lifecycle], of: usize, key: &str, fact: Observed<'_>, when: &str) {
    moves.extend(read_instance(held, copy, &at(&copy[of], key), &fact, when));
}

const PULL_REQUESTS: usize = 1;
const JOBS: usize = 3;
const REVIEW: usize = 1;

#[test]
fn a_step_ends_on_a_pull_request_being_approved_the_agent_is_stopped_while_it_waits_and_fleet_wakes_it_on_the_transition() {
    let (workflow, copy) = (feature("pr: approved"), the_jobs_copy());
    let mut held = HeldInstances::new();
    held.take_on(taken_on(&copy[PULL_REQUESTS], "812")).expect("held");
    let mut moves = Vec::new();

    let waiting = ends::place(&workflow, &copy, &held, &moves);
    assert_eq!(waiting.step, Some(REVIEW), "the Job holds a pull request, so it is on the step that waits for it");
    let told = waiting.told.expect("the agent is told on entering");
    assert!(told.contains("This step ends when every pr is approved, which Fleet watches for."), "{told}");
    assert!(told.contains("End your turn now: Fleet wakes you with a message when it comes, so do not poll for it."), "{told}");
    assert!(!waiting.waits_on_the_person, "a state is the world's to reach: no wait for him");
    assert_eq!(ends::woken(&workflow, &copy, &held, &moves, REVIEW), None, "nothing to wake it with");

    seen(&mut held, &mut moves, &copy, PULL_REQUESTS, "812", Observed::Forge(ForgeFact::ReviewersOn), "t1");
    assert_eq!(ends::place(&workflow, &copy, &held, &moves).step, Some(REVIEW), "reviewers asked is not approval");

    seen(&mut held, &mut moves, &copy, PULL_REQUESTS, "812", Observed::Forge(ForgeFact::Approved), "t2");
    let moved_on = ends::place(&workflow, &copy, &held, &moves);
    assert_eq!(moved_on.step, Some(2), "the transition moves the Job on");
    assert!(moved_on.told.is_some_and(|words| words.contains("Write the summary.")), "and the agent is told the next step");
    assert_eq!(
        ends::woken(&workflow, &copy, &held, &moves, REVIEW).as_deref(),
        Some("Fleet: PR #812 is Approved. That ends the \"Review\" step of this Job's Feature workflow."),
        "one line says what got there"
    );

    seen(&mut held, &mut moves, &copy, PULL_REQUESTS, "812", Observed::Forge(ForgeFact::InReview), "t3");
    assert_eq!(ends::place(&workflow, &copy, &held, &moves).step, Some(2), "a reviewer taking it back does not drag the Job back");
}

#[test]
fn a_step_that_ends_on_a_job_landing_waits_for_every_job_started_and_any_takes_the_first() {
    let copy = the_jobs_copy();
    let (first, second) = ("98632ca8", "a41f09d3");
    let mut held = HeldInstances::new();
    for key in [first, second] {
        held.take_on(taken_on(&copy[JOBS], key).linked_to(at(&copy[JOBS], "33b2b07b"))).expect("held");
    }
    let landed = || Observed::Fleet(FleetFact::Landed);

    let (every, any) = (feature("job: landed"), feature("any job: landed"));
    let mut moves = Vec::new();
    assert_eq!(ends::place(&every, &copy, &held, &moves).step, Some(REVIEW));
    seen(&mut held, &mut moves, &copy, JOBS, first, landed(), "t1");
    assert_eq!(ends::place(&every, &copy, &held, &moves).step, Some(REVIEW), "one of two has landed");
    assert_eq!(ends::place(&any, &copy, &held, &moves).step, Some(2), "and that is enough for any");
    assert_eq!(ends::woken(&every, &copy, &held, &moves, REVIEW), None);

    seen(&mut held, &mut moves, &copy, JOBS, second, landed(), "t2");
    assert_eq!(ends::place(&every, &copy, &held, &moves).step, Some(2), "both have");
    assert_eq!(
        ends::woken(&every, &copy, &held, &moves, REVIEW).as_deref(),
        Some("Fleet: Job 98632ca8 and Job a41f09d3 are Landed. That ends the \"Review\" step of this Job's Feature workflow.")
    );
}
