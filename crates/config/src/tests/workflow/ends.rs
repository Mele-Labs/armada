//! `ends_on` naming a lifecycle state: how it may be written, what is refused where it is read, and
//! what is asked once the lifecycles that apply are known.

use core_model::{Lifecycle, LifecycleEnd, StepEnd};

use super::bug_with;
use crate::error::Fault;
use crate::tests::refusals;
use crate::workflow::{names_no_lifecycle, EndFault};

const REVIEW: &str = "  - id: review\n    label: Review\n    delivers: false\n    advance_gate: auto\n";

fn end(written: &str) -> StepEnd {
    StepEnd::Lifecycle(LifecycleEnd::from_wire(written).expect("a lifecycle end"))
}

#[test]
fn a_step_ends_on_a_lifecycle_state_beside_the_words_and_each_way_of_writing_it_reads_alike() {
    let def = bug_with(&format!(
        "{REVIEW}    ends_on:\n      - replied\n      - pr: approved\n      - any ticket: closed\n      - \"job: landed\"\n"
    ))
    .expect("a lifecycle state is an end");
    assert_eq!(
        def.steps()[3].ends_on(),
        [StepEnd::Replied, end("pr: approved"), end("any ticket: closed"), end("job: landed")],
        "an unquoted `- pr: approved` is a map and a quoted one is text; both are the pair"
    );
    let first = def.steps()[3].ends_on()[1].lifecycle().expect("a lifecycle end");
    assert_eq!(first.waits(), core_model::Waits::Every, "every instance unless the step says any");
}

#[test]
fn an_end_written_like_a_lifecycle_and_not_one_is_refused_at_its_item() {
    for (written, wants) in [
        ("- pr: Approved", "not a lifecycle end"),
        ("- \"pr:\"", "not a lifecycle end"),
        ("- { pr: approved, ticket: closed }", "one lifecycle and its state"),
    ] {
        let refused = refusals(bug_with(&format!("{REVIEW}    ends_on:\n      {written}\n")));
        let found = refused.iter().find(|r| r.key == "steps[3].ends_on[0]" && matches!(r.fault, Fault::Ends(_)));
        let found = found.unwrap_or_else(|| panic!("{written}: {refused:?}"));
        assert!(found.fault.to_string().contains(wants), "{written}: {}", found.fault);
    }
}

#[test]
fn a_lifecycle_end_on_a_step_something_else_places_is_refused_rather_than_ignored() {
    for key in ["walks", "follows_plan", "records_plan"] {
        let refused = refusals(bug_with(&format!("{REVIEW}    {key}: true\n    ends_on:\n      - pr: approved\n")));
        assert!(
            refused.iter().any(|r| r.key == "steps[3].ends_on" && matches!(r.fault, Fault::Ends(EndFault::Placed { .. }))),
            "{key}: {refused:?}"
        );
    }
}

#[test]
fn a_delivering_step_takes_a_lifecycle_end_as_a_gate_on_its_merge() {
    // The worked example's last step delivers.
    let text = format!("{}\n    ends_on:\n      - pr: approved\n", super::BUG.trim_end());
    let def = super::parse(&text).unwrap_or_else(|refused| panic!("a delivering step gated by a state is read: {refused:?}"));
    let land = def.steps().last().expect("a step");
    assert!(land.delivers());
    assert_eq!(land.ends_on(), [end("pr: approved")]);
}

#[test]
fn a_lifecycle_or_state_the_repository_does_not_have_is_named_in_the_sentence() {
    let def = bug_with(&format!(
        "{REVIEW}    ends_on:\n      - pr: approved\n      - pull: approved\n      - pr: accepted\n"
    ))
    .expect("the file reads; whether the lifecycles exist is asked after");
    let steps = || def.steps().iter().map(|step| (step.id().as_str(), step.ends_on()));
    let said: Vec<String> = names_no_lifecycle(steps(), &Lifecycle::built_in()).iter().map(ToString::to_string).collect();
    assert_eq!(said.len(), 2, "the first names a real state: {said:?}");
    assert!(said[0].contains("step `review` ends on `pull: approved`") && said[0].contains("no lifecycle here is called `pull`"), "{}", said[0]);
    assert!(said[0].contains("ticket, pr, deploy, job"), "it lists what there is: {}", said[0]);
    assert!(said[1].contains("the pr lifecycle has no state `accepted`") && said[1].contains("draft, ready"), "{}", said[1]);
    assert!(names_no_lifecycle(steps(), &[]).len() == 3, "with none known, every one is unnamed");
}
