//! A workflow change on the wire: a `cannot` one with what Armada lacks, and either side step by step,
//! each absent rather than empty where Fleet has nothing to say.

use crate::{decode, encode, ChangeOrigin, ChangeState, ChangeStep, Instant, StepId, WorkflowChange, WorkflowId};

fn change(state: ChangeState) -> WorkflowChange {
    WorkflowChange {
        id: 7,
        workflow_id: WorkflowId::carried("feature"),
        source: String::from("repository"),
        project: None,
        step_id: None,
        origin: ChangeOrigin::Asked { text: String::from("end Propose on an approved document") },
        before: String::from("{}"),
        after: None,
        state,
        reason: None,
        missing: Vec::new(),
        before_steps: None,
        after_steps: None,
        after_name: None,
        after_for_requests: None,
        notes: None,
        drafted: false,
        at: Instant::carried("2026-10-28T09:00:00.000Z"),
    }
}

/// A change with nothing missing and no steps read sends no key for either.
#[test]
fn a_change_with_nothing_missing_and_no_steps_carries_no_keys_for_them() {
    let written = encode(&change(ChangeState::Drafting)).expect("a change encodes");
    for key in ["missing", "before_steps", "after_steps"] {
        assert!(!written.contains(key), "{key} in {written}");
    }
}

/// A `cannot` change carries its state, what it lacks and its steps with what ends each, and reads back whole.
#[test]
fn a_cannot_change_carries_what_armada_lacks_and_its_steps_and_reads_back() {
    let step = ChangeStep {
        id: StepId::carried("propose"),
        label: String::from("Propose"),
        instructions: None,
        ends_when: Some(String::from("you reply")),
        ends_any: Vec::new(),
        passed_over_when: None,
        about: None,
        gate: None,
        asks_delivery: false,
        delivers: false,
    };
    let cannot = WorkflowChange {
        missing: vec![String::from("A step can't end when a document is approved.")],
        before_steps: Some(vec![step]),
        ..change(ChangeState::Cannot)
    };
    let written = encode(&cannot).expect("a change encodes");
    assert!(written.contains(r#""state":"cannot""#), "{written}");
    assert!(written.contains(r#""missing":["A step can't end when a document is approved."]"#), "{written}");
    assert!(written.contains(r#""before_steps":[{"id":"propose","label":"Propose","ends_when":"you reply"}]"#), "{written}");
    let read: WorkflowChange = decode("a change", written.as_bytes()).expect("it reads");
    assert_eq!(read, cannot);
}

/// A change that landed in the owner's draft says so, and one that did not sends no key; a project's change names its project.
#[test]
fn a_drafted_change_says_so_and_a_projects_names_its_project_and_each_is_absent_otherwise() {
    let plain = encode(&change(ChangeState::Ready)).expect("a change encodes");
    assert!(!plain.contains("drafted") && !plain.contains("project"), "{plain}");
    let drafted = WorkflowChange { drafted: true, source: String::from("project"), project: Some(String::from("storefront")), ..change(ChangeState::Ready) };
    let written = encode(&drafted).expect("a change encodes");
    assert!(written.contains(r#""drafted":true"#) && written.contains(r#""project":"storefront""#), "{written}");
    let read: WorkflowChange = decode("a change", written.as_bytes()).expect("it reads");
    assert_eq!(read, drafted);
    let before: WorkflowChange = decode("a change", plain.as_bytes()).expect("it reads");
    assert!(!before.drafted && before.project.is_none(), "a Fleet from before sends neither");
}
