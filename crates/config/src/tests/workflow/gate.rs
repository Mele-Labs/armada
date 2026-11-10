//! A step's `gate`: the plan gate its group ends on. Absent is `none`, and a word outside the
//! eight is refused naming them.

use core_model::StepGate;

use super::{bug_with, manifest};
use crate::error::Fault;
use crate::resolve::ResolvedWorkflow;
use crate::tests::refusals;
use crate::workflow::step::PLAN_GATE_LEGAL;

const REVIEW: &str = "  - id: review\n    label: Review\n    delivers: false\n    advance_gate: auto\n";

#[test]
fn the_words_the_step_reads_are_the_gates_core_model_has() {
    let words: Vec<&str> = StepGate::ALL.iter().map(|gate| gate.word()).collect();
    assert_eq!(PLAN_GATE_LEGAL, words.as_slice());
    for gate in StepGate::ALL {
        let def = bug_with(&format!("{REVIEW}    gate: {}\n", gate.word())).expect("a gate word loads");
        assert_eq!(def.steps()[3].gate(), gate);
    }
}

#[test]
fn a_step_with_no_gate_has_none_and_the_gate_reaches_the_resolved_step() {
    let def = bug_with(&format!("{REVIEW}")).expect("no gate loads");
    assert_eq!(def.steps()[3].gate(), StepGate::None);
    let def = bug_with(&format!("{REVIEW}    gate: walk_approved\n")).expect("a gate loads");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest()).expect("every name declared");
    assert_eq!(resolved.steps()[3].gate(), StepGate::WalkApproved);
    assert_eq!(resolved.steps()[0].gate(), StepGate::None);
}

#[test]
fn a_word_that_is_no_gate_is_refused_naming_the_seven() {
    let refused = refusals(bug_with(&format!("{REVIEW}    gate: approved\n")));
    let found = refused.iter().find(|r| r.key == "steps[3].gate").expect("refused at the key");
    assert_eq!(
        found.fault,
        Fault::NotInTheSchema { value: "approved".to_string(), legal: PLAN_GATE_LEGAL }
    );
}
