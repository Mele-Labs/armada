//! A panel member's brief asks whether refused work is risky, and its answer
//! must say. A Judge of one is asked and read exactly as before. #206.

use super::judge::{brief, workflow};
use crate::{OutcomeTurn, Refusals, Unreadable, Wording};

const REFUSING: &str = "verdict: not_met\nexpected: the loop stops at n\n\
                        produced: the loop stops at n - 1\nconsequence: the last row is dropped";

#[test]
fn a_judge_of_one_is_not_asked_about_risk() {
    let workflow = workflow();
    let alone = brief(&workflow);
    assert!(!alone.question().contains("risky:"), "{}", alone.question());
    let read = alone.read(REFUSING).expect("a refusal read as before");
    assert_eq!(read.risky, None);
}

#[test]
fn a_panel_member_is_asked_and_its_brief_otherwise_unchanged() {
    let workflow = workflow();
    let alone = brief(&workflow);
    let member = brief(&workflow).on_a_panel(&Wording::shipped());
    assert!(member.question().starts_with(alone.question()));
    assert!(
        member.question().contains("risky: no"),
        "{}",
        member.question()
    );
}

#[test]
fn a_panel_member_that_refuses_and_says_nothing_of_risk_is_unreadable() {
    let workflow = workflow();
    let member = brief(&workflow).on_a_panel(&Wording::shipped());
    assert_eq!(member.read(REFUSING), Err(Unreadable::RefusalWeighsNoRisk));
}

#[test]
fn no_is_not_risky_and_anything_else_is_kept_as_the_reason() {
    let workflow = workflow();
    let member = brief(&workflow).on_a_panel(&Wording::shipped());
    let redo = member
        .read(&format!("{REFUSING}\nrisky: No."))
        .expect("a refusal that only needs redoing");
    assert_eq!(redo.risky, None);
    let risky = member
        .read(&format!(
            "{REFUSING}\nrisky: it deletes the rows other readers rely on"
        ))
        .expect("a refusal that calls the work risky");
    assert_eq!(
        risky.risky.as_deref(),
        Some("it deletes the rows other readers rely on")
    );
}

#[test]
fn a_panel_member_with_no_objection_owes_no_risk_line() {
    let workflow = workflow();
    let member = brief(&workflow).on_a_panel(&Wording::shipped());
    let met = member.read("verdict: met").expect("no objection");
    assert_eq!(met.risky, None);
}

#[test]
fn the_drone_sent_back_is_told_what_was_expected_and_found_once_and_not_the_consequence() {
    let workflow = workflow();
    let member = brief(&workflow).on_a_panel(&Wording::shipped());
    let said = member
        .read(&format!("{REFUSING}\nrisky: no"))
        .expect("a refusal");
    let refusals = Refusals::among(&[said.clone(), said]).expect("refused");
    let told = OutcomeTurn::sent_back(&workflow.steps()[0], &refusals, &Wording::shipped());
    let text = told.text();
    assert!(text.contains("The Judges agreed"), "{text}");
    assert_eq!(text.matches("the loop stops at n - 1").count(), 1, "{text}");
    assert!(!text.contains("the last row is dropped"), "{text}");
}
