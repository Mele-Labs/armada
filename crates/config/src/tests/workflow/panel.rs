//! `panel_models`: one model per member of a Judge panel, so a panel's
//! agreement is more than one model agreeing with itself. #206.

use super::bug_with;
use crate::error::Fault;
use crate::tests::{fault_at, refusals};

/// A fourth step whose one criterion is put to a panel, with `panel` written
/// under the check as given.
fn panelled(panel: &[&str]) -> String {
    let mut lines = vec![
        "  - id: review".to_string(),
        "    label: Review".to_string(),
        "    evidence:".to_string(),
        "      submitted:".to_string(),
        "        type: diff".to_string(),
        "    delivers: false".to_string(),
        "    advance_gate: auto_if_judge_passes".to_string(),
        "    judge_checks:".to_string(),
        "      - model: the-reporting-model".to_string(),
    ];
    lines.extend(panel.iter().map(|line| format!("        {line}")));
    lines.extend([
        "        criteria:".to_string(),
        "          - criterion_id: c1".to_string(),
        "            question: Does the fix address the cause?".to_string(),
        String::new(),
    ]);
    lines.join("\n")
}

#[test]
fn each_member_runs_the_model_named_for_it_and_the_list_sizes_the_panel() {
    let def = bug_with(&panelled(&[
        "panel_models: [the-deciding-model, the-reporting-model]",
    ]))
    .expect("a panel of two models");
    let judge = &def.steps()[3].judge_checks()[0];
    assert_eq!(judge.panel_size(), 2);
    assert_eq!(
        judge.member_model(1).map(|model| model.as_str()),
        Some("the-deciding-model")
    );
    assert_eq!(
        judge.member_model(2).map(|model| model.as_str()),
        Some("the-reporting-model")
    );
}

#[test]
fn a_check_naming_no_members_runs_every_member_on_its_own_model() {
    let def = bug_with(&panelled(&["panel_size: 3"])).expect("a panel of one model");
    let judge = &def.steps()[3].judge_checks()[0];
    assert_eq!(judge.panel_size(), 3);
    assert!(judge.members().is_empty());
    assert_eq!(
        judge.member_model(3).map(|model| model.as_str()),
        Some("the-reporting-model")
    );
}

#[test]
fn a_member_model_the_roster_does_not_offer_is_refused() {
    let refused = refusals(bug_with(&panelled(&[
        "panel_models: [the-deciding-model, a-model-nobody-offers]",
    ])));
    assert!(matches!(
        fault_at(&refused, "steps[3].judge_checks[0].panel_models[1]"),
        Fault::NoSuchModel { .. }
    ));
}

#[test]
fn a_panel_size_the_list_disagrees_with_is_refused() {
    let refused = refusals(bug_with(&panelled(&[
        "panel_size: 3",
        "panel_models: [the-deciding-model, the-reporting-model]",
    ])));
    assert_eq!(
        fault_at(&refused, "steps[3].judge_checks[0].panel_size"),
        &Fault::PanelSizeAndModelsDisagree { size: 3, models: 2 }
    );
}
