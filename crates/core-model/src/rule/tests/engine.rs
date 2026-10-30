//! What an occurrence sets off, and that it sets it off once.

use super::*;
use crate::lifecycle::Standing;
use crate::rule::{
    ActKind, Autonomy, AutonomyDefaults, Course, Engine, JobCondition, JobEvent, JobFacts, Occurrence, PlannedAct, RanOnce,
};

fn plan(rules: &[Rule], defaults: &AutonomyDefaults, seen: &Occurrence, held: &HeldInstances, ran: &mut RanOnce) -> Vec<PlannedAct> {
    Engine::new(rules, defaults).plan(seen, held, &JobFacts::default(), ran)
}

fn on_keys(planned: &[PlannedAct]) -> Vec<String> {
    planned.iter().map(|one| one.on().map_or(String::from("-"), |on| on.key().to_string())).collect()
}

fn the_move_rule() -> Vec<Rule> {
    vec![rule(when_planning_starts(parts("plan_moves_tickets", When::Job(JobEvent::Started), vec![move_todo_tickets()])))]
}

#[test]
fn a_step_starting_plans_the_ticket_move_and_asks_by_default() {
    let held = a_job_with_two_tickets("To Do", "To Do");
    let planned = plan(&the_move_rule(), &AutonomyDefaults::new(), &Occurrence::step_started(step("plan"), "t1"), &held, &mut RanOnce::new());

    assert_eq!(on_keys(&planned), ["ARM-41", "ARM-42"], "one planned act for each ticket it is done to");
    assert!(planned.iter().all(|one| one.course() == Course::Asks), "a tracker write asks until the rule is set to act");
    assert_eq!(planned[0].act().kind(), ActKind::TrackerWrite);
    assert_eq!(planned[0].rule().as_str(), "plan_moves_tickets");
    assert_eq!((planned[0].position(), planned[1].position()), (0, 0));
}

#[test]
fn the_same_rule_acts_at_once_when_it_or_its_repository_says_so() {
    let held = a_job_with_two_tickets("To Do", "In Progress");
    let seen = Occurrence::step_started(step("plan"), "t1");

    let mut acting = the_move_rule();
    let mut parts = when_planning_starts(parts("plan_moves_tickets", When::Job(JobEvent::Started), vec![move_todo_tickets()]));
    parts.autonomy = Some(Autonomy::Acts);
    acting[0] = rule(parts);
    let planned = plan(&acting, &AutonomyDefaults::new(), &seen, &held, &mut RanOnce::new());
    assert_eq!(on_keys(&planned), ["ARM-41"], "only the ticket still in To do");
    assert_eq!(planned[0].course(), Course::Acts);

    let repository = AutonomyDefaults::new().setting(ActKind::TrackerWrite, Autonomy::Acts);
    let planned = plan(&the_move_rule(), &repository, &seen, &held, &mut RanOnce::new());
    assert_eq!(planned[0].course(), Course::Acts, "a default the repository set for the kind");
}

#[test]
fn off_plans_nothing_and_neither_does_a_missing_rule_file() {
    let held = a_job_with_two_tickets("To Do", "To Do");
    let seen = Occurrence::step_started(step("plan"), "t1");

    let mut off = when_planning_starts(parts("plan_moves_tickets", When::Job(JobEvent::Started), vec![move_todo_tickets()]));
    off.autonomy = Some(Autonomy::Off);
    assert!(plan(&[rule(off)], &AutonomyDefaults::new(), &seen, &held, &mut RanOnce::new()).is_empty());

    let off_by_default = AutonomyDefaults::new().setting(ActKind::TrackerWrite, Autonomy::Off);
    assert!(plan(&the_move_rule(), &off_by_default, &seen, &held, &mut RanOnce::new()).is_empty());

    assert!(plan(&[], &AutonomyDefaults::new(), &seen, &held, &mut RanOnce::new()).is_empty(), "no rule, nothing planned");
}

#[test]
fn a_rule_runs_once_for_an_occurrence_and_a_replay_after_a_restart_plans_nothing_new() {
    let held = a_job_with_two_tickets("To Do", "To Do");
    let rules = the_move_rule();
    let seen = Occurrence::step_started(step("plan"), "t1");
    let mut ran = RanOnce::new();

    assert_eq!(plan(&rules, &AutonomyDefaults::new(), &seen, &held, &mut ran).len(), 2);
    assert!(plan(&rules, &AutonomyDefaults::new(), &seen, &held, &mut ran).is_empty(), "the same occurrence again");

    // Fleet keeps the pairs and reads them back after a restart.
    let mut after_a_restart: RanOnce = ran.iter().map(|(rule, key)| (rule.clone(), key.clone())).collect();
    assert_eq!(after_a_restart, ran);
    assert!(plan(&rules, &AutonomyDefaults::new(), &seen, &held, &mut after_a_restart).is_empty(), "a replay");

    // The step starting again later is another occurrence, and the rule sees it.
    let again = Occurrence::step_started(step("plan"), "t2");
    assert_eq!(plan(&rules, &AutonomyDefaults::new(), &again, &held, &mut after_a_restart).len(), 2);
}

#[test]
fn a_rule_that_did_not_hold_has_still_seen_the_occurrence() {
    let rules = the_move_rule();
    let seen = Occurrence::step_started(step("plan"), "t1");
    let mut ran = RanOnce::new();

    let nothing_to_do = a_job_with_two_tickets("In Progress", "Done");
    assert!(plan(&rules, &AutonomyDefaults::new(), &seen, &nothing_to_do, &mut ran).is_empty());
    let later = a_job_with_two_tickets("To Do", "To Do");
    assert!(
        plan(&rules, &AutonomyDefaults::new(), &seen, &later, &mut ran).is_empty(),
        "a replay does not reach back for a transition the conditions turned away"
    );
}

#[test]
fn an_instance_entering_a_state_sets_off_the_rules_for_it_and_for_nothing_else() {
    let pull_request = Lifecycle::pull_request();
    let held = a_job_with_two_tickets("In Review", "In Review");
    let mut tell = parts("tell_when_approved", When::Enters { lifecycle: lifecycle_id("pr"), state: state("approved") }, vec![Act::Wake]);
    tell.conditions = vec![Condition::Instances {
        among: Among::LinkedFrom(lifecycle_id("ticket")),
        quantity: Quantity::Every,
        in_state: state("in_review"),
    }];
    let rules = [rule(tell)];
    let pr = at(&pull_request, "812");
    let entered = |into_state: &str, when: &str| Occurrence::entered(pr.clone(), moved_into(into_state), when);
    let defaults = AutonomyDefaults::new();

    assert!(plan(&rules, &defaults, &entered("merged", "t1"), &held, &mut RanOnce::new()).is_empty(), "another state");
    let unmapped = Occurrence::entered(pr.clone(), Moved { from: Standing::Unread, to: Standing::Unmapped("odd".into()) }, "t1");
    assert!(plan(&rules, &defaults, &unmapped, &held, &mut RanOnce::new()).is_empty(), "a place no state claims");

    let planned = plan(&rules, &defaults, &entered("approved", "t1"), &held, &mut RanOnce::new());
    assert_eq!(planned.len(), 1);
    assert_eq!((planned[0].course(), planned[0].on()), (Course::Acts, None), "waking the agent acts, and is done to no instance");
}

#[test]
fn a_condition_reads_across_the_links_and_every_needs_somebody() {
    let pull_request = Lifecycle::pull_request();
    let pr = at(&pull_request, "812");
    let entered = Occurrence::entered(pr, moved_into("approved"), "t1");
    let when = When::Enters { lifecycle: lifecycle_id("pr"), state: state("approved") };
    let every_ticket_in_review = Condition::Instances {
        among: Among::LinkedFrom(lifecycle_id("ticket")),
        quantity: Quantity::Every,
        in_state: state("in_review"),
    };
    let tell = |condition: Condition| {
        let mut one = parts("tell", when.clone(), vec![Act::Wake]);
        one.conditions = vec![condition];
        [rule(one)]
    };
    let defaults = AutonomyDefaults::new();
    let count = |rules: &[Rule], held: &HeldInstances| plan(rules, &defaults, &entered, held, &mut RanOnce::new()).len();

    assert_eq!(count(&tell(every_ticket_in_review.clone()), &a_job_with_two_tickets("In Review", "In Review")), 1);
    assert_eq!(count(&tell(every_ticket_in_review), &a_job_with_two_tickets("In Review", "In Progress")), 0, "one is not");

    // A pull request with no tickets is not a pull request whose tickets are all in review.
    let ticketless = {
        let mut held = HeldInstances::new();
        held.take_on(LifecycleInstance::taken_on(at(&pull_request, "812"))).expect("held");
        held
    };
    let all_in_review = Condition::Instances { among: Among::LinkedFrom(lifecycle_id("ticket")), quantity: Quantity::Every, in_state: state("in_review") };
    let none_closed = Condition::Instances { among: Among::LinkedFrom(lifecycle_id("ticket")), quantity: Quantity::No, in_state: state("closed") };
    assert_eq!(count(&tell(all_in_review), &ticketless), 0);
    assert_eq!(count(&tell(none_closed), &ticketless), 1, "none of nothing is closed");
}

#[test]
fn a_condition_on_the_job_reads_what_fleet_shows_of_it() {
    let held = HeldInstances::new();
    let seen = Occurrence::job(JobEvent::PullRequestOpened, "t1");
    let mut after_a_pull_request = parts("after", When::Job(JobEvent::PullRequestOpened), vec![Act::Wake]);
    after_a_pull_request.conditions = vec![
        Condition::Job(JobCondition::Workflow(words("feature"))),
        Condition::Job(JobCondition::AtStep(step("build"))),
        Condition::Job(JobCondition::PullRequestOpened),
    ];
    let rules = [rule(after_a_pull_request)];
    let defaults = AutonomyDefaults::new();
    let engine = Engine::new(&rules, &defaults);
    let facts = |workflow: &str, on: &str| JobFacts {
        workflow: Some(words(workflow)),
        step: Some(step(on)),
        pull_request_opened: true,
    };

    assert_eq!(engine.plan(&seen, &held, &facts("feature", "build"), &mut RanOnce::new()).len(), 1);
    assert!(engine.plan(&seen, &held, &facts("bug", "build"), &mut RanOnce::new()).is_empty());
    assert!(engine.plan(&seen, &held, &facts("feature", "plan"), &mut RanOnce::new()).is_empty());
    assert!(engine.plan(&seen, &held, &JobFacts::default(), &mut RanOnce::new()).is_empty());
}

#[test]
fn what_leaves_a_state_and_comes_back_is_two_occurrences_and_two_keys_never_alike() {
    let ticket = tickets();
    let one = at(&ticket, "ARM-41");
    let seen = |from: Standing, to: Standing, when: &str| Occurrence::entered(one.clone(), Moved { from, to }, when).key();

    let first = seen(Standing::Unread, into("in_review"), "t1");
    assert_eq!(first, seen(Standing::Unread, into("in_review"), "t1"));
    assert_ne!(first, seen(Standing::Unread, into("in_review"), "t2"), "seen again later");
    assert_ne!(first, seen(into("in_progress"), into("in_review"), "t1"), "from somewhere else");

    // Two different words must not write alike however a tracker spells them.
    let a = seen(Standing::Unmapped("a".into()), Standing::Unmapped("b:c".into()), "t");
    let b = seen(Standing::Unmapped("a:1:b".into()), Standing::Unmapped("c".into()), "t");
    assert_ne!(a.to_string(), b.to_string());
}
