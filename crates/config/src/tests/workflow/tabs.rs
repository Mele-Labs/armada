//! A step's `tabs`: the right-column tabs it opens, front first. Absent is none, an id outside
//! `STEP_TABS` is refused naming them, and the list reaches the resolved step.

use core_model::STEP_TABS;

use super::{bug_with, manifest};
use crate::error::Fault;
use crate::resolve::ResolvedWorkflow;
use crate::tests::refusals;

const REVIEW: &str = "  - id: review\n    label: Review\n    delivers: false\n    advance_gate: auto\n";

#[test]
fn a_step_lists_its_tabs_front_first_and_they_reach_the_resolved_step() {
    let def = bug_with(&format!("{REVIEW}    tabs: [walks, changes, now]\n")).expect("tabs load");
    assert_eq!(def.steps()[3].tabs(), ["walks", "changes", "now"]);
    let resolved = ResolvedWorkflow::resolve(&def, &manifest()).expect("every name declared");
    assert_eq!(resolved.steps()[3].tabs(), ["walks", "changes", "now"]);
    assert!(resolved.steps()[0].tabs().is_empty(), "a step that lists none opens none");
}

#[test]
fn a_repeated_tab_is_kept_once_in_the_place_it_first_stands() {
    let def = bug_with(&format!("{REVIEW}    tabs: [plan, now, plan]\n")).expect("tabs load");
    assert_eq!(def.steps()[3].tabs(), ["plan", "now"]);
}

#[test]
fn an_id_that_is_no_tab_is_refused_at_its_item_naming_the_set() {
    let refused = refusals(bug_with(&format!("{REVIEW}    tabs: [plan, pull_requests]\n")));
    let found = refused.iter().find(|r| r.key == "steps[3].tabs[1]").unwrap_or_else(|| panic!("{refused:?}"));
    assert_eq!(found.fault, Fault::NotInTheSchema { value: "pull_requests".to_string(), legal: STEP_TABS });
}

#[test]
fn tabs_that_are_not_a_list_of_words_are_refused() {
    assert!(refusals(bug_with(&format!("{REVIEW}    tabs: plan\n"))).iter().any(|r| r.key == "steps[3].tabs"));
    assert!(refusals(bug_with(&format!("{REVIEW}    tabs: []\n"))).iter().any(|r| r.key == "steps[3].tabs" && r.fault == Fault::Empty));
}
