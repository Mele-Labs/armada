//! What a rule is, what it refuses to be, and what its autonomy comes to.

use super::*;
use crate::lifecycle::NotAnId;
use crate::rule::{ActKind, Autonomy, AutonomyDefaults, ForgeWrite, JobEvent, MalformedRule, Retries, UnknownName};

#[test]
fn a_rule_that_does_nothing_is_not_a_rule() {
    let bare = parts("nothing", When::StepStarts(step("plan")), Vec::new());
    assert_eq!(Rule::declared(bare), Err(MalformedRule::NoActs));
}

#[test]
fn the_instance_that_entered_is_only_there_for_a_rule_about_an_instance_entering() {
    let to_the_entered = |when: When| {
        parts(
            "moves",
            when,
            vec![Act::Tracker { write: TrackerWrite::Close, target: Target::entered() }],
        )
    };
    let on_a_step = Rule::declared(to_the_entered(When::StepStarts(step("plan"))));
    assert_eq!(on_a_step, Err(MalformedRule::NoInstanceToSpeakOf));
    let on_an_event = Rule::declared(to_the_entered(When::Job(JobEvent::Landed)));
    assert_eq!(on_an_event, Err(MalformedRule::NoInstanceToSpeakOf));
    let entering = When::Enters { lifecycle: lifecycle_id("pr"), state: state("merged") };
    assert!(Rule::declared(to_the_entered(entering)).is_ok());

    // A condition on what is linked has the same trouble, and what the Job holds does not.
    let mut linked = parts("linked", When::StepEnds(step("build")), vec![Act::Wake]);
    linked.conditions = vec![Condition::Instances {
        among: Among::LinkedFrom(lifecycle_id("ticket")),
        quantity: Quantity::Every,
        in_state: state("in_review"),
    }];
    assert_eq!(Rule::declared(linked.clone()), Err(MalformedRule::NoInstanceToSpeakOf));
    linked.conditions = vec![Condition::Instances {
        among: Among::Held(lifecycle_id("ticket")),
        quantity: Quantity::Every,
        in_state: state("in_review"),
    }];
    assert!(Rule::declared(linked).is_ok());
}

#[test]
fn a_review_request_names_somebody() {
    let ask = Act::Forge { write: ForgeWrite::RequestReviewers { from: Vec::new() }, target: Target::held(lifecycle_id("pr")) };
    assert_eq!(Rule::declared(parts("ask", When::Job(JobEvent::PullRequestOpened), vec![ask])), Err(MalformedRule::NobodyToAsk));
}

#[test]
fn a_rule_naming_a_state_its_lifecycle_lacks_is_found_out_before_it_never_fires() {
    let (ticket, pull_request) = (tickets(), Lifecycle::pull_request());
    let mut typo = parts(
        "typo",
        When::Enters { lifecycle: lifecycle_id("pr"), state: state("aproved") },
        vec![Act::Tracker {
            write: TrackerWrite::Move { to: state("in_reveiw") },
            target: Target { among: Among::LinkedFrom(lifecycle_id("ticket")), in_state: None },
        }],
    );
    typo.conditions =
        vec![Condition::Instances { among: Among::Held(lifecycle_id("deploy")), quantity: Quantity::No, in_state: state("failed") }];
    let found = rule(typo).unknown_in(&[&ticket, &pull_request]);
    assert_eq!(
        found,
        [
            UnknownName::State { lifecycle: lifecycle_id("pr"), state: state("aproved") },
            UnknownName::Lifecycle(lifecycle_id("deploy")),
            UnknownName::State { lifecycle: lifecycle_id("ticket"), state: state("in_reveiw") },
        ]
    );
    assert_eq!(found[0].to_string(), "names the state `aproved`, which the `pr` lifecycle does not have");

    let fine = rule(when_planning_starts(parts("fine", When::Job(JobEvent::Started), vec![move_todo_tickets()])));
    assert!(fine.unknown_in(&[&ticket]).is_empty());
}

#[test]
fn what_an_act_is_by_default_is_what_the_design_table_says() {
    use Autonomy::{Acts, Asks};
    let table = [
        (ActKind::TrackerWrite, Asks),
        (ActKind::ForgeWrite, Asks),
        (ActKind::Draft, Asks),
        (ActKind::StartJob, Asks),
        (ActKind::StartWatch, Acts),
        (ActKind::TellAgent, Acts),
        (ActKind::RunCommand, Acts),
        (ActKind::RunAgent, Acts),
    ];
    assert_eq!(table.map(|(kind, _)| kind), *ActKind::ALL, "a kind added to the table is a row here");
    for (kind, default) in table {
        assert_eq!(kind.default_autonomy(), default, "{}", kind.as_wire());
        assert_eq!(ActKind::from_wire(kind.as_wire()), Some(kind));
    }
    assert_eq!(Act::Wake.kind(), ActKind::TellAgent);
    assert_eq!(Act::RunSkill { skill: words("review") }.kind(), ActKind::RunAgent);
}

#[test]
fn the_rules_word_beats_the_repository_which_beats_the_workspace_which_beats_the_kind() {
    let workspace = AutonomyDefaults::new().setting(ActKind::TrackerWrite, Autonomy::Acts).setting(ActKind::Draft, Autonomy::Off);
    let repository = AutonomyDefaults::new().setting(ActKind::TrackerWrite, Autonomy::Asks);
    let laid = repository.laid_over(&workspace);

    assert_eq!(AutonomyDefaults::new().resolved(ActKind::TrackerWrite, None), Autonomy::Asks, "the kind's own");
    assert_eq!(workspace.resolved(ActKind::TrackerWrite, None), Autonomy::Acts);
    assert_eq!(laid.resolved(ActKind::TrackerWrite, None), Autonomy::Asks, "the repository's, over the Workspace's");
    assert_eq!(laid.resolved(ActKind::Draft, None), Autonomy::Off, "a kind the repository left alone is the Workspace's");
    assert_eq!(laid.resolved(ActKind::StartWatch, None), Autonomy::Acts, "and one nobody spoke of is the kind's");
    assert_eq!(laid.resolved(ActKind::TrackerWrite, Some(Autonomy::Acts)), Autonomy::Acts, "the rule's own word");
}

#[test]
fn a_planned_act_has_no_off_and_a_rule_cannot_ask_for_endless_retries() {
    assert_eq!(Autonomy::Off.course(), None);
    assert!(Autonomy::Acts.course().is_some() && Autonomy::Asks.course().is_some());
    assert_eq!(Retries::new(5).map(Retries::times), Ok(5));
    assert_eq!(Retries::new(6), Err(crate::rule::OverTheCap { asked: 6 }));
    assert!(Retries::new(u32::MAX).is_err());
    assert_eq!(OnFailure::default().retries.times(), 2);
}

#[test]
fn an_id_and_words_are_checked_where_they_are_made() {
    assert!(RuleId::new("move_ticket").is_ok());
    let refused: NotAnId = RuleId::new("move ticket").expect_err("two words");
    assert_eq!(refused.written(), "move ticket");
    assert!(Words::new("   ").is_err());
    assert_eq!(words("  review please ").as_str(), "review please");
}
