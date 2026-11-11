//! Where an Operator rule holds: on the wire, and as the store kept it before a scope was one of three.

use crate::operator_page::{OperatorAnswerPress, OperatorAnswer, OperatorLearned, OperatorRule, RuleScope};
use crate::Instant;

fn pick(scope: RuleScope) -> OperatorRule {
    OperatorRule::PickRule { scope, what: "red_main".into(), pick: "just_file".into() }
}

#[test]
fn a_scope_is_tagged_by_its_kind() {
    let wire = |scope: &RuleScope| serde_json::to_value(scope).expect("written");
    assert_eq!(wire(&RuleScope::Everywhere), serde_json::json!({ "kind": "everywhere" }));
    assert_eq!(wire(&RuleScope::Repository { manifest_id: "01A".into() }), serde_json::json!({ "kind": "repository", "manifest_id": "01A" }));
    assert_eq!(wire(&RuleScope::Project { project: "my-store".into() }), serde_json::json!({ "kind": "project", "project": "my-store" }));
    for scope in [RuleScope::Everywhere, RuleScope::Repository { manifest_id: "01A".into() }, RuleScope::Project { project: "my-store".into() }] {
        assert_eq!(serde_json::from_value::<RuleScope>(wire(&scope)).expect("read"), scope);
    }
}

#[test]
fn a_rule_round_trips_through_the_store_with_its_scope() {
    for scope in [RuleScope::Everywhere, RuleScope::Repository { manifest_id: "01A".into() }, RuleScope::Project { project: "my-store".into() }] {
        let rule = pick(scope);
        assert_eq!(OperatorRule::read(&rule.kept()), Some(rule));
    }
}

#[test]
fn a_rule_kept_with_a_manifest_id_reads_as_that_repository_and_one_without_as_everywhere() {
    let here = r#"{"kind":"pick_rule","manifest_id":"01A","what":"red_main","pick":"just_file"}"#;
    let everywhere = r#"{"kind":"pick_rule","what":"red_main","pick":"just_file"}"#;
    assert_eq!(OperatorRule::read(here), Some(pick(RuleScope::Repository { manifest_id: "01A".into() })));
    assert_eq!(OperatorRule::read(everywhere), Some(pick(RuleScope::Everywhere)));
}

#[test]
fn what_is_not_a_rule_does_not_read() {
    assert_eq!(OperatorRule::read(r#"{"kind":"hold_paths","manifest_id":"01A"}"#), None);
}

#[test]
fn a_learned_rule_carries_its_scope_and_a_name_to_read() {
    let learned = OperatorLearned {
        id: 3,
        scope: RuleScope::Project { project: "my-store".into() },
        repository: Some("My store".into()),
        said: "Start a fix when main is red".into(),
        at: Instant::carried("2026-11-10T09:00:00.000Z"),
    };
    let wire = serde_json::to_value(&learned).expect("written");
    assert_eq!(wire["scope"], serde_json::json!({ "kind": "project", "project": "my-store" }));
    assert!(wire.get("manifest_id").is_none());
    assert_eq!(serde_json::from_value::<OperatorLearned>(wire).expect("read"), learned);
}

#[test]
fn a_press_names_a_project_only_when_it_has_one() {
    let plain: OperatorAnswerPress = serde_json::from_str(r#"{"id":"turn:1","answer":"always_here"}"#).expect("read");
    assert_eq!(plain.project, None);
    let held = OperatorAnswerPress { id: "turn:1".into(), answer: OperatorAnswer::AlwaysHere, then: None, option: None, project: Some("my-store".into()) };
    assert_eq!(serde_json::to_value(&held).expect("written")["project"], "my-store");
    assert!(serde_json::to_value(&plain).expect("written").get("project").is_none());
}
