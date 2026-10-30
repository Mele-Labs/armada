//! A lifecycle rule piece file: what it reads to, and what it is refused for.

use std::path::Path;

use core_model::rule::{
    Act, Among, Autonomy, Condition, ForgeWrite, JobCondition, JobEvent, OnFailure, Quantity, Retries, Rule, Target,
    Then, TrackerWrite, When, Words,
};
use core_model::{LifecycleId, LifecycleScope, ScopeId, StateId, StepId};

use crate::{parse_rule, RuleError};

fn parsed(text: &str) -> Rule {
    parse_rule(Path::new("rule.yml"), text).expect("a rule that loads")
}

fn said(text: &str) -> String {
    parse_rule(Path::new("rule.yml"), text).expect_err("a rule that does not load").to_string()
}

fn lc(written: &str) -> LifecycleId {
    LifecycleId::new(written).expect("a lifecycle id")
}

fn state(written: &str) -> StateId {
    StateId::new(written).expect("a state id")
}

fn words(written: &str) -> Words {
    Words::new(written).expect("words")
}

const MOVES_TICKETS: &str = "version: 1
rule_id: plan_moves_tickets
name: Move the tickets when planning starts
scope: my-store
when: { step_starts: plan }
if:
  - any: { held: ticket, in: todo }
do:
  - tracker_write: { move: in_progress, on: { held: ticket, in: todo } }
autonomy: acts
on_failure: { retries: 3, then: tell_agent }
";

#[test]
fn a_file_reads_to_the_rule_it_says() {
    let rule = parsed(MOVES_TICKETS);
    assert_eq!(rule.id().as_str(), "plan_moves_tickets");
    assert_eq!(rule.scope(), &LifecycleScope::Within(ScopeId::new("my-store").expect("a scope")));
    assert_eq!(rule.when(), &When::StepStarts(StepId::new("plan")));
    assert_eq!(
        rule.conditions(),
        [Condition::Instances {
            among: Among::Held(lc("ticket")),
            quantity: Quantity::Any,
            in_state: state("todo"),
        }]
    );
    assert_eq!(
        rule.does(),
        [Act::Tracker {
            write: TrackerWrite::Move { to: state("in_progress") },
            target: Target::held(lc("ticket")).standing_in(state("todo")),
        }]
    );
    assert_eq!(rule.autonomy(), Some(Autonomy::Acts));
    assert_eq!(rule.on_failure(), OnFailure { retries: Retries::new(3).expect("under the cap"), then: Then::TellAgent });
}

#[test]
fn only_what_a_rule_needs_has_to_be_written() {
    let rule = parsed("version: 1\nrule_id: nudge\nname: Nudge\nwhen: { job: landed }\ndo:\n  - tell_agent: { wake: true }\n");
    assert_eq!(rule.scope(), &LifecycleScope::Everywhere);
    assert_eq!(rule.when(), &When::Job(JobEvent::Landed));
    assert!(rule.conditions().is_empty());
    assert_eq!((rule.autonomy(), rule.on_failure()), (None, OnFailure::default()));
}

#[test]
fn every_moment_condition_and_kind_of_act_is_a_key_a_file_can_write() {
    let rule = parsed(
        "version: 1
rule_id: everything
name: Everything
when: { enters: { lifecycle: pr, state: approved } }
if:
  - every: { linked_from: ticket, in: in_review }
  - no: { linking_to: job, in: running }
  - any: { entered: true, in: approved }
  - job: { workflow: feature }
  - job: { step: build }
  - job: { pull_request_opened: true }
do:
  - tracker_write: { assign: alice, on: { entered: true } }
  - tracker_write: { close: true, on: { linked_from: ticket } }
  - forge_write: { request_reviewers: [alice, bob], on: { entered: true } }
  - forge_write: { auto_merge: true, on: { entered: true } }
  - draft: { template: 'Ready for {pr}', on: { entered: true } }
  - start_job: { prompt: Look at the deploy, workflow: bug }
  - start_watch: { recipe: deploy-health, values: { sha: abc } }
  - tell_agent: { message: Approved }
  - run_command: { command: lint }
  - run_agent: { skill: review }
  - run_agent: { drone: Summarise the review }
",
    );
    assert_eq!(rule.when(), &When::Enters { lifecycle: lc("pr"), state: state("approved") });
    assert_eq!(rule.conditions().len(), 6);
    assert_eq!(rule.conditions()[3], Condition::Job(JobCondition::Workflow(words("feature"))));
    assert_eq!(rule.conditions()[5], Condition::Job(JobCondition::PullRequestOpened));
    let kinds: Vec<&str> = rule.does().iter().map(|act| act.kind().as_wire()).collect();
    assert_eq!(
        kinds,
        [
            "tracker_write",
            "tracker_write",
            "forge_write",
            "forge_write",
            "draft",
            "start_job",
            "start_watch",
            "tell_agent",
            "run_command",
            "run_agent",
            "run_agent"
        ]
    );
    assert_eq!(
        rule.does()[2],
        Act::Forge {
            write: ForgeWrite::RequestReviewers { from: vec![words("alice"), words("bob")] },
            target: Target::entered(),
        }
    );
}

#[test]
fn a_condition_fleet_cannot_test_is_refused_and_the_file_names_the_ones_it_can() {
    let said = said(
        "version: 1
rule_id: stale
name: Stale
when: { step_starts: plan }
if:
  - ticket_age_over: 3d
  - every: { held: ticket, in: todo, assignee: me }
  - job: { agent_said_so: true }
do:
  - tracker_write: { close: true, on: { held: ticket } }
",
    );
    for expected in [
        "`if[0].ticket_age_over` is not a key Armada reads",
        "`if[0]` wanted one of `every`, `any`, `no` or `job` and holds none of them",
        "`if[1].every.assignee` is not a key Armada reads",
        "`if[2].job.agent_said_so` is not a key Armada reads",
    ] {
        assert!(said.contains(expected), "{expected:?} is not in:\n{said}");
    }
}

#[test]
fn a_file_is_refused_for_every_key_that_is_wrong_and_names_each() {
    let said = said(
        "version: 2
rule_id: 'move ticket'
name: Move
when: { step_starts: plan, job: landed }
do:
  - tracker_write: { move: in_progress }
  - tracker_write: { close: true, move: in_progress, on: { held: ticket } }
  - forge_write: { merge: false, on: { held: pr } }
  - tweet: { text: hi }
  - run_agent: { skill: review, drone: summarise }
autonomy: sometimes
on_failure: { retries: 9, then: shrug }
",
    );
    for expected in [
        "`version` is `2`",
        "`rule_id` wanted an id",
        "`when` wanted one of `step_starts`, `step_ends`, `job` or `enters` and holds more than one",
        "`do[0].tracker_write.on` is required",
        "`do[1].tracker_write` wanted one of `assign`, `move` or `close` and holds more than one",
        "`do[2].forge_write.merge` wanted true, or leave the key out and holds false",
        "`do[3].tweet` is not a key Armada reads",
        "`do[4].run_agent` wanted one of `skill` or `drone` and holds more than one",
        "`autonomy` is `sometimes`",
        "`on_failure.retries` wanted a number of retries, five or fewer",
        "`on_failure.then` is `shrug`",
    ] {
        assert!(said.contains(expected), "{expected:?} is not in:\n{said}");
    }
}

#[test]
fn a_file_missing_its_keys_says_which() {
    let said = said("version: 1\nname: Thing\n");
    for expected in ["`rule_id` is required", "`when` is required", "`do` is required"] {
        assert!(said.contains(expected), "{expected:?} is not in:\n{said}");
    }
    assert!(self::said("states: [").contains("is not YAML"));
    assert!(self::said("version: 1\nrule_id: a\nname: A\nwhen: { job: ended }\ndo: []\n").contains("`do` is empty"));
}

#[test]
fn keys_that_are_right_but_make_a_rule_that_is_not_come_back_as_the_rules_own_refusal() {
    // The instance that entered is only there for a rule about an instance entering a state.
    let said = said(
        "version: 1\nrule_id: odd\nname: Odd\nwhen: { step_ends: build }\ndo:\n  - tracker_write: { close: true, on: { entered: true } }\n",
    );
    assert!(matches!(
        parse_rule(
            Path::new("rule.yml"),
            "version: 1\nrule_id: odd\nname: Odd\nwhen: { step_ends: build }\ndo:\n  - tracker_write: { close: true, on: { entered: true } }\n"
        ),
        Err(RuleError::Malformed { .. })
    ));
    assert!(said.starts_with("rule.yml speaks of the instance that entered a state"), "{said}");
}
