//! What a Job holds of a lifecycle, read against a step's end: which instances have got there, which
//! have not, and how each is named in the sentence that waits on it and the one that wakes the agent.
//! Pure: the instances and their moves are passed in, as the rail passes them.

use core_model::{Lifecycle, LifecycleEnd, Waits};
use store::{KeptInstance, KeptMove};

use super::lifecycle_ends::{end, reviewed};
use super::session_rail::{fitted, follows, plan_step, step};
use crate::rail::{self, Facts, Fit, RailStep};
use crate::rail_lifecycle::{self, Held, StepBits};

fn instance(lifecycle: &str, key: &str) -> KeptInstance {
    KeptInstance {
        session_id: "job".into(),
        lifecycle: lifecycle.into(),
        key: key.into(),
        links: Vec::new(),
        seen_source: String::new(),
        seen: String::new(),
        watch_id: String::new(),
        taken_on_at: "2026-10-29T09:00:00.000Z".into(),
    }
}

fn moved(lifecycle: &str, key: &str, to: &str) -> KeptMove {
    KeptMove {
        id: 0,
        session_id: "job".into(),
        lifecycle: lifecycle.into(),
        key: key.into(),
        from: "unread".into(),
        to: to.into(),
        seen_source: String::new(),
        seen: String::new(),
        at: "2026-10-29T09:05:00.000Z".into(),
    }
}

fn built_in(id: &str) -> Lifecycle {
    Lifecycle::built_in().into_iter().find(|one| one.id().as_str() == id).expect("a built-in")
}

fn written(end: &str) -> LifecycleEnd {
    LifecycleEnd::from_wire(end).expect("a lifecycle end")
}

fn reached_of(end: &str, held: &Held) -> (Vec<String>, Vec<String>, bool) {
    let end = written(end);
    let lifecycle = built_in(end.lifecycle().as_str());
    let reading = rail_lifecycle::read(&end, &lifecycle, held);
    let keys = |ones: &[&KeptInstance]| ones.iter().map(|one| one.key.clone()).collect::<Vec<_>>();
    (keys(&reading.reached), keys(&reading.pending), reading.met(end.waits()))
}

fn two_pull_requests(first: &str, second: &str) -> Held {
    Held {
        instances: vec![instance("pr", "812"), instance("pr", "813"), instance("ticket", "ARM-1")],
        moves: vec![moved("pr", "812", first), moved("pr", "813", second), moved("ticket", "ARM-1", "in:closed")],
    }
}

#[test]
fn every_waits_for_each_instance_the_job_holds_and_any_for_the_first() {
    let one_of_two = two_pull_requests("in:approved", "in:draft");
    assert_eq!(reached_of("pr: approved", &one_of_two), (vec!["812".into()], vec!["813".into()], false), "one still to go");
    assert_eq!(reached_of("any pr: approved", &one_of_two), (vec!["812".into()], vec!["813".into()], true));
    let both = two_pull_requests("in:approved", "in:approved");
    assert!(reached_of("pr: approved", &both).2);
    let neither = two_pull_requests("in:draft", "in:ready");
    assert!(!reached_of("pr: approved", &neither).2 && !reached_of("any pr: approved", &neither).2);
}

#[test]
fn only_the_instances_of_the_lifecycle_the_end_names_are_read() {
    let held = two_pull_requests("in:draft", "in:draft");
    assert_eq!(reached_of("ticket: closed", &held), (vec!["ARM-1".into()], Vec::new(), true), "the ticket, and no pull request");
}

#[test]
fn a_job_holding_none_of_the_lifecycle_has_met_nothing_even_where_every_is_vacuous() {
    let none = Held::default();
    assert_eq!(reached_of("pr: approved", &none), (Vec::new(), Vec::new(), false));
    assert_eq!(reached_of("any pr: approved", &none), (Vec::new(), Vec::new(), false));
}

#[test]
fn an_instance_that_got_there_stays_there_and_one_that_is_over_has_reached_every_state_of_its_lifecycle() {
    let taken_back = Held {
        instances: vec![instance("pr", "812")],
        moves: vec![moved("pr", "812", "in:approved"), moved("pr", "812", "in:in_review")],
    };
    assert!(reached_of("pr: approved", &taken_back).2, "a reviewer taking an approval back does not drag the Job back");
    let merged = Held { instances: vec![instance("pr", "812")], moves: vec![moved("pr", "812", "in:merged")] };
    assert!(reached_of("pr: approved", &merged).2, "merged without a move through approved: nothing more will happen to it");
    let unread = Held { instances: vec![instance("pr", "812")], moves: vec![moved("pr", "812", "unmapped:in the queue")] };
    assert!(!reached_of("pr: approved", &unread).2, "a standing the lifecycle has no state for reaches nothing");
}

#[test]
fn the_steps_that_hold_and_the_steps_whose_end_has_come_are_read_off_the_instances() {
    let steps = reviewed();
    let lifecycles = Lifecycle::built_in();
    let (held, met) = rail_lifecycle::bits(&steps, &lifecycles, &Held::default());
    assert_eq!((held, met), (StepBits::default(), StepBits::default()));
    let waiting = Held { instances: vec![instance("pr", "812")], moves: vec![moved("pr", "812", "in:draft")] };
    let (held, met) = rail_lifecycle::bits(&steps, &lifecycles, &waiting);
    assert_eq!((held.has(2), met.has(2)), (true, false));
    assert!(!held.has(0) && !held.has(3), "only the step that lists the end");
    let got = Held { instances: vec![instance("pr", "812")], moves: vec![moved("pr", "812", "in:approved")] };
    let (held, met) = rail_lifecycle::bits(&steps, &lifecycles, &got);
    assert_eq!((held.has(2), met.has(2)), (true, true));
}

#[test]
fn a_step_that_lists_a_lifecycle_the_job_has_no_definition_of_holds_and_meets_nothing() {
    let unknown = [RailStep { ends_on: vec![end("pull: approved")], ..step("Review") }];
    let held = Held { instances: vec![instance("pull", "812")], moves: vec![moved("pull", "812", "in:approved")] };
    let (held, met) = rail_lifecycle::bits(&unknown, &Lifecycle::built_in(), &held);
    assert_eq!((held, met), (StepBits::default(), StepBits::default()), "the refusal at read is the place this is said");
}

#[test]
fn an_instance_is_named_the_way_the_person_knows_it() {
    let name = |lifecycle: &str, key: &str, title: Option<&str>| rail_lifecycle::named(&instance(lifecycle, key), title);
    assert_eq!(name("pr", "812", None), "PR #812");
    assert_eq!(name("ticket", "ARM-41", None), "ARM-41");
    assert_eq!(name("job", "01J8ZABCDEF", Some("Fix the reader")), "Job \"Fix the reader\"");
    assert_eq!(name("job", "01J8ZABCDEF", Some("  ")), "Job 01J8ZABC", "a blank title is none");
    assert_eq!(name("deploy", "prod", None), "deploy prod", "a lifecycle of the repository's own");
}

fn called(one: &KeptInstance) -> String {
    rail_lifecycle::named(one, None)
}

#[test]
fn the_wait_reads_waiting_on_the_instance_by_name_and_the_state_by_its_label() {
    let pull = built_in("pr");
    let held = two_pull_requests("in:draft", "in:draft");
    let said = |end: &str, held: &Held| {
        let end = written(end);
        let reading = rail_lifecycle::read(&end, &pull, held);
        rail_lifecycle::waiting_on(&end, &pull, &reading, &called)
    };
    let one = Held { instances: vec![instance("pr", "812")], moves: Vec::new() };
    assert_eq!(said("pr: approved", &one), "Waiting on PR #812: Approved");
    assert_eq!(said("pr: reviewers_on", &one), "Waiting on PR #812: Reviewers on");
    assert_eq!(said("pr: approved", &held), "Waiting on PR #812, PR #813: Approved");
    assert_eq!(said("any pr: approved", &held), "Waiting on any of PR #812, PR #813: Approved");
    let half = two_pull_requests("in:approved", "in:draft");
    assert_eq!(said("pr: approved", &half), "Waiting on PR #813: Approved", "the one that got there is not waited on");
    assert_eq!(said("pr: approved", &Held::default()), "Waiting on a pull request: Approved", "none held yet: the lifecycle by name");
}

#[test]
fn the_line_that_wakes_the_agent_names_what_got_there_and_the_step_it_ended() {
    let pull = built_in("pr");
    let said = |end: &str, held: &Held| {
        let end = written(end);
        let reading = rail_lifecycle::read(&end, &pull, held);
        rail_lifecycle::woke("Review", "feature", &end, &pull, &reading, &called)
    };
    let one = Held { instances: vec![instance("pr", "812")], moves: vec![moved("pr", "812", "in:approved")] };
    assert_eq!(said("pr: approved", &one), "Fleet: PR #812 is Approved. That ends the \"Review\" step of this Job's feature workflow.");
    let both = two_pull_requests("in:approved", "in:approved");
    assert_eq!(
        said("pr: approved", &both),
        "Fleet: PR #812 and PR #813 are Approved. That ends the \"Review\" step of this Job's feature workflow."
    );
    assert_eq!(written("any pr: approved").waits(), Waits::Any);
}

/// The shipped orchestrate workflow's shape: Assess, Plan, Dispatch (follows the plan, its tasks marked
/// done as each Job starts), Oversee, and Summarise (entered on a commit).
fn orchestrating() -> Vec<RailStep> {
    let oversee = RailStep { ends_on: vec![end("job: landed")], ..step("Oversee") };
    vec![step("Assess"), plan_step(), follows("Dispatch"), oversee, fitted("Summarise", Fit::Committed)]
}

fn two_jobs(first: &str, second: &str) -> Held {
    Held {
        instances: vec![instance("job", "98632ca8"), instance("job", "a41f09d3")],
        moves: vec![moved("job", "98632ca8", first), moved("job", "a41f09d3", second)],
    }
}

fn parent_on(steps: &[RailStep], held: &Held, tasks_open: bool, committed: bool) -> Option<usize> {
    let mut facts = Facts { plan_recorded: true, plan_approved: true, tasks_open, committed, ..Facts::default() };
    (facts.lifecycle_held, facts.lifecycle_met) = rail_lifecycle::bits(steps, &Lifecycle::built_in(), held);
    rail::current(steps, &facts)
}

#[test]
fn a_parent_holding_two_jobs_sits_on_oversee_until_both_have_landed_or_ended() {
    let steps = orchestrating();
    let (running, started) = (two_jobs("in:running", "in:running"), Held { instances: vec![instance("job", "98632ca8")], moves: Vec::new() });
    assert_eq!(parent_on(&steps, &started, true, false), Some(2), "one Job started, the others still to start: Dispatch");
    assert_eq!(parent_on(&steps, &running, true, false), Some(2), "both are running and a task is open: still Dispatch");
    assert_eq!(parent_on(&steps, &running, false, false), Some(3), "every task done: Oversee");
    assert_eq!(parent_on(&steps, &two_jobs("in:landed", "in:running"), false, false), Some(3), "one of two has landed");
    assert_eq!(parent_on(&steps, &two_jobs("in:ended", "in:running"), false, false), Some(3), "or ended");
    assert_eq!(parent_on(&steps, &two_jobs("in:landed", "in:ended"), false, false), Some(4), "landed and ended: Summarise, before any commit");
    assert_eq!(parent_on(&steps, &two_jobs("in:landed", "in:landed"), false, true), Some(4));
}

#[test]
fn a_job_started_late_is_waited_on_and_a_job_ended_after_it_landed_is_still_over() {
    let steps = orchestrating();
    let late = Held {
        instances: vec![instance("job", "98632ca8"), instance("job", "a41f09d3"), instance("job", "c70e1a55")],
        moves: vec![moved("job", "98632ca8", "in:landed"), moved("job", "a41f09d3", "in:landed"), moved("job", "c70e1a55", "in:running")],
    };
    assert_eq!(parent_on(&steps, &late, false, false), Some(3), "the third was started after the others landed");
    let mut done = late;
    done.moves.push(moved("job", "c70e1a55", "in:ended"));
    assert_eq!(parent_on(&steps, &done, false, false), Some(4));
    done.moves.push(moved("job", "98632ca8", "in:ended"));
    assert_eq!(parent_on(&steps, &done, false, false), Some(4), "landed, then ended: it has still landed");
}
