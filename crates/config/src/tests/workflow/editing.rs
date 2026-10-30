//! A workflow's steps edited as text, for a Job's own copy: add after a step, move, reword and drop, each
//! answering text the loader still reads, in the order the edit asked for.

use super::{parse, BUG};
use crate::workflow::{edit_steps, EditFault, NewStep, StepEdit};

fn ids(text: &str) -> Vec<String> {
    parse(text).expect("the edited text reads").steps().iter().map(|step| step.id().as_str().to_string()).collect()
}

fn dispatch() -> NewStep {
    NewStep {
        id: None,
        label: "Dispatch".into(),
        about: Some("Starts a Job for each fix".into()),
        instructions: Some("Start one Job per fix and note its id.".into()),
        ends_on: vec!["job: landed".into()],
    }
}

#[test]
fn a_step_is_added_after_the_one_named_and_the_text_still_reads() {
    let edited = edit_steps(BUG, &StepEdit::Add { after: Some("plan".into()), step: dispatch() }).expect("added");
    assert_eq!(edited.step, "dispatch", "the id comes from the label");
    assert_eq!(ids(&edited.text), ["plan", "dispatch", "implement", "handoff"]);
    let def = parse(&edited.text).expect("reads");
    let added = &def.steps()[1];
    assert_eq!(added.label(), "Dispatch");
    assert_eq!(added.about(), Some("Starts a Job for each fix"));
    assert_eq!(added.instructions(), Some("Start one Job per fix and note its id."));
    assert_eq!(added.ends_on().len(), 1, "and what ends it");
    assert!(!added.delivers());
}

#[test]
fn a_step_added_after_nothing_is_the_first() {
    let edited = edit_steps(BUG, &StepEdit::Add { after: None, step: NewStep { id: Some("scout".into()), ..dispatch() } }).expect("added");
    assert_eq!(ids(&edited.text), ["scout", "plan", "implement", "handoff"]);
}

#[test]
fn a_step_is_refused_a_name_already_taken_a_place_that_is_not_there_and_no_words() {
    let taken = edit_steps(BUG, &StepEdit::Add { after: None, step: NewStep { id: Some("plan".into()), ..dispatch() } });
    assert_eq!(taken.unwrap_err(), EditFault::StepExists("plan".into()));
    let nowhere = edit_steps(BUG, &StepEdit::Add { after: Some("lunch".into()), step: dispatch() });
    assert_eq!(nowhere.unwrap_err(), EditFault::NoSuchStep("lunch".into()));
    let blank = edit_steps(BUG, &StepEdit::Add { after: None, step: NewStep { label: "  ".into(), ..dispatch() } });
    assert_eq!(blank.unwrap_err(), EditFault::Blank("label"));
}

#[test]
fn a_step_moves_to_after_another_or_to_the_front() {
    let after = edit_steps(BUG, &StepEdit::Move { step: "plan".into(), after: Some("implement".into()) }).expect("moved");
    assert_eq!(ids(&after.text), ["implement", "plan", "handoff"]);
    let front = edit_steps(BUG, &StepEdit::Move { step: "implement".into(), after: None }).expect("moved");
    assert_eq!(ids(&front.text), ["implement", "plan", "handoff"]);
    let itself = edit_steps(BUG, &StepEdit::Move { step: "plan".into(), after: Some("plan".into()) }).expect("moved");
    assert_eq!(ids(&itself.text), ["plan", "implement", "handoff"], "after itself leaves it where it is");
    let nowhere = edit_steps(BUG, &StepEdit::Move { step: "plan".into(), after: Some("lunch".into()) });
    assert_eq!(nowhere.unwrap_err(), EditFault::NoSuchStep("lunch".into()), "and a place that is not there is said");
}

#[test]
fn a_step_takes_new_words_and_keeps_the_rest_of_itself() {
    let edited = edit_steps(
        BUG,
        &StepEdit::Reword { step: "implement".into(), label: Some("Build".into()), about: None, instructions: Some("Build it small.".into()) },
    )
    .expect("reworded");
    let def = parse(&edited.text).expect("reads");
    assert_eq!(def.steps()[1].label(), "Build");
    assert_eq!(def.steps()[1].instructions(), Some("Build it small."));
    assert_eq!(def.steps()[1].mechanical_checks().len(), 3, "its checks stay");
    let nothing = edit_steps(BUG, &StepEdit::Reword { step: "implement".into(), label: None, about: Some(" ".into()), instructions: None });
    assert_eq!(nothing.unwrap_err(), EditFault::Blank("words"));
}

#[test]
fn a_step_is_dropped_except_the_one_that_delivers() {
    let dropped = edit_steps(BUG, &StepEdit::Drop { step: "implement".into() }).expect("dropped");
    assert_eq!(ids(&dropped.text), ["plan", "handoff"]);
    let delivers = edit_steps(BUG, &StepEdit::Drop { step: "handoff".into() });
    assert_eq!(delivers.unwrap_err(), EditFault::Delivers("handoff".into()));
    let gone = edit_steps(BUG, &StepEdit::Drop { step: "lunch".into() });
    assert_eq!(gone.unwrap_err(), EditFault::NoSuchStep("lunch".into()));
}

#[test]
fn text_that_is_not_a_workflow_is_said_so() {
    assert!(matches!(edit_steps("just words", &StepEdit::Drop { step: "plan".into() }), Err(EditFault::NotAWorkflow(_))));
    assert!(matches!(edit_steps("version: 1\nname: x\n", &StepEdit::Drop { step: "plan".into() }), Err(EditFault::NotAWorkflow(_))));
}
