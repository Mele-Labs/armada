//! A Job created at `proposing`, and each of its three roads out. #1714.

use super::*;

fn answer() -> Answered {
    NewJob {
        title: title("say which line the parser drops"),
        ..draft()
    }
    .into_answer()
}

fn later() -> Timestamp {
    at("2026-08-26T09:05:00.000Z")
}

#[test]
fn a_dispatched_request_is_a_job_with_no_workflow_no_steps_and_no_plan() {
    let job = proposing();
    assert_eq!(job.status(), JobStatus::Proposing);
    assert_eq!(
        job.title().as_str(),
        "the parser is off by one on the last line"
    );
    assert_eq!(job.frozen_workflow(), None);
    assert!(job.workflow().steps().is_empty());
    assert_eq!(job.workflow_id().as_str(), "");
    assert!(job.steps().is_empty());
    assert!(job.acceptance_criteria().is_empty());
    assert!(job.proposal_id().is_some());
    assert_eq!(job.origin(), Origin::Manual);
}

#[test]
fn an_answer_freezes_the_workflow_and_crosses_to_the_gate() {
    let moved = proposing()
        .answered(answer(), Actor::Fleet, later())
        .expect("proposing -> awaiting_approval");
    assert_eq!(moved.job.status(), JobStatus::AwaitingApproval);
    assert_eq!(moved.event.from(), JobStatus::Proposing);
    assert_eq!(moved.event.to(), JobStatus::AwaitingApproval);
    assert_eq!(moved.job.frozen_workflow(), Some(&workflow()));
    assert_eq!(moved.job.steps().len(), 2);
    assert!(moved
        .job
        .steps()
        .iter()
        .all(|step| step.state() == StepState::NotStarted));
    // The proposer's title, over the request, and the Job's identity kept.
    assert_eq!(
        moved.job.title().as_str(),
        "say which line the parser drops"
    );
    assert_eq!(moved.job.id(), proposing().id());
    assert_eq!(moved.job.proposal_id(), proposing().proposal_id());
    assert_eq!(moved.job.number(), proposing().number());
    // And it is the Job any other dispatch makes from here: approvable.
    assert_eq!(
        drive(&moved.job, &[Target::Queued]).status(),
        JobStatus::Queued
    );
}

#[test]
fn a_decline_and_a_fault_each_escalate_with_their_own_trigger() {
    for trigger in [
        EscalationTrigger::NoWorkflowFits,
        EscalationTrigger::ProposerFailed,
    ] {
        let moved = proposing()
            .transition(Target::Escalated(trigger), Actor::Fleet, later())
            .unwrap_or_else(|e| panic!("{}: {e}", trigger.as_wire()));
        assert_eq!(moved.job.status(), JobStatus::Escalated);
        assert_eq!(moved.event.reason(), &TransitionReason::Escalation(trigger));
        assert_eq!(trigger.level(), TriggerLevel::Job);
        assert_eq!(StepLevelTrigger::of(trigger), None);
    }
}

#[test]
fn a_person_stopping_the_call_kills_the_job() {
    let moved = proposing()
        .transition(Target::Killed, Actor::Human, later())
        .expect("proposing -> killed");
    assert_eq!(moved.job.status(), JobStatus::Killed);
}

#[test]
fn only_an_answer_crosses_to_the_gate() {
    assert_eq!(
        proposing().transition(Target::AwaitingApproval, Actor::Fleet, later()),
        Err(IllegalTransition::OnlyAnAnswerCrosses)
    );
    assert_eq!(
        created().answered(answer(), Actor::Fleet, later()),
        Err(IllegalTransition::NothingToAnswer {
            from: JobStatus::AwaitingApproval
        })
    );
}

#[test]
fn proposing_escalates_on_its_two_triggers_and_no_other() {
    assert_eq!(
        proposing().transition(
            Target::Escalated(EscalationTrigger::Stalled),
            Actor::Fleet,
            later()
        ),
        Err(IllegalTransition::UndeclaredTrigger {
            from: JobStatus::Proposing,
            to: JobStatus::Escalated,
            given: EscalationTrigger::Stalled,
        })
    );
}

#[test]
fn a_settled_workflow_outlives_a_call_that_dies_and_is_never_frozen() {
    let settled = WorkflowId::carried(id("01J0000000000000000000WF00"));
    let job = proposing().workflow_settled(settled.clone());
    assert_eq!(job.workflow_id(), &settled);
    assert_eq!(job.frozen_workflow(), None);
    let escalated = job
        .transition(
            Target::Escalated(EscalationTrigger::ProposerFailed),
            Actor::Fleet,
            later(),
        )
        .expect("proposing -> escalated")
        .job;
    assert_eq!(escalated.workflow_id(), &settled);
    assert_eq!(escalated.title(), proposing().title());
    // A frozen workflow is not moved by a late settle.
    assert_eq!(created().workflow_settled(settled).workflow(), &workflow());
}

#[test]
fn a_splits_extras_wait_at_the_gate_naming_the_head() {
    let head = proposing();
    let extra = Job::create_split(draft(), head.id().clone(), TopLevelOrigin::Manual, later());
    assert_eq!(extra.status(), JobStatus::AwaitingApproval);
    assert_eq!(extra.origin(), Origin::Manual);
    assert_eq!(
        extra.dispatched_by(),
        Some(&DispatchOrigin {
            job_id: head.id().clone(),
            step_id: None,
            pass: None,
        })
    );
}

/// **A person sends a proposal back, and the answer brings it to the gate
/// again**: `awaiting_approval -> proposing -> awaiting_approval`, a person's act
/// only, from the gate only, and what was there is what an answer puts back.
#[test]
fn a_proposal_sent_back_is_read_again_and_an_answer_returns_it_to_the_gate() {
    let at_the_gate = proposing()
        .answered(answer(), Actor::Fleet, later())
        .expect("answered")
        .job;
    let back = at_the_gate
        .sent_back_to_the_proposer(Actor::Human, later())
        .expect("awaiting_approval -> proposing");
    assert_eq!(back.job.status(), JobStatus::Proposing);
    assert_eq!(back.event.from(), JobStatus::AwaitingApproval);
    assert_eq!(back.event.to(), JobStatus::Proposing);

    assert!(matches!(
        at_the_gate.sent_back_to_the_proposer(Actor::Fleet, later()),
        Err(IllegalTransition::NotAPersonsAct { .. })
    ));
    assert!(matches!(
        proposing().sent_back_to_the_proposer(Actor::Human, later()),
        Err(IllegalTransition::NothingToReconsider { .. })
    ));

    // A revision that could not be made puts the proposal back as it was.
    let restored = back
        .job
        .answered(back.job.as_answered(), Actor::Fleet, later())
        .expect("proposing -> awaiting_approval")
        .job;
    assert_eq!(restored.status(), JobStatus::AwaitingApproval);
    assert_eq!(restored.title(), at_the_gate.title());
    assert_eq!(restored.workflow(), at_the_gate.workflow());
    assert_eq!(restored.steps().len(), at_the_gate.steps().len());
}
