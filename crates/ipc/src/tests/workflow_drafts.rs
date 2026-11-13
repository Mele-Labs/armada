//! A draft of a workflow or a piece on the wire: each type round-trips through JSON, and what is absent is absent.

use crate::{
    decode, encode, DraftBy, DraftChange, Instant, LifecyclePieceKind, LifecyclePieceSaved, PieceDraft, PieceDraftDiscarded, PiecePublished, PieceScope,
    PublishPieceDraft, PublishWorkflowDraft, PutPieceDraft, PutWorkflowDraft, SessionId, WorkflowDraft, WorkflowDraftDiscarded, WorkflowId, WorkflowPublished,
    WorkflowSaved, WorkflowScope,
};

fn round_trips<T>(value: &T) -> String
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let spelled = encode(value).expect("plain data");
    let back: T = decode("a value", spelled.as_bytes()).expect("reads");
    assert_eq!(&back, value, "{spelled}");
    spelled
}

fn at(text: &str) -> Instant {
    Instant::carried(format!("2026-11-13T{text}.000Z"))
}

fn changes() -> Vec<DraftChange> {
    vec![
        DraftChange { id: 1, by: DraftBy::You, at: at("09:00:00"), text: String::from("{one}"), summary: None, change: None },
        DraftChange { id: 2, by: DraftBy::Agent, at: at("09:05:00"), text: String::from("{two}"), summary: Some(String::from("split Refine in two")), change: Some(7) },
    ]
}

#[test]
fn a_workflow_draft_round_trips_and_names_who_made_each_change_by_a_word() {
    let draft = WorkflowDraft {
        workflow_id: WorkflowId::carried("feature"),
        source: String::from("project"),
        project: Some(String::from("storefront")),
        text: String::from("{two}"),
        base: String::from("{published}"),
        unfit: Some(String::from("step `x` names no Check")),
        changes: changes(),
        started_at: at("09:00:00"),
        updated_at: at("09:05:00"),
    };
    let spelled = round_trips(&draft);
    assert!(spelled.contains(r#""by":"you""#) && spelled.contains(r#""by":"agent""#), "{spelled}");
    assert!(spelled.contains(r#""change":7"#), "{spelled}");

    let fit = WorkflowDraft { project: None, unfit: None, source: String::from("armada"), ..draft };
    let spelled = round_trips(&fit);
    assert!(!spelled.contains("unfit") && !spelled.contains("project"), "a draft that would load and is nobody's project sends neither key: {spelled}");
    assert!(spelled.contains(r#"{"id":1,"by":"you","at":"2026-11-13T09:00:00.000Z","text":"{one}"}"#), "a change the owner made carries no summary or change: {spelled}");
}

#[test]
fn what_a_workflow_draft_is_put_discarded_and_published_with_round_trips_and_a_body_without_the_options_reads() {
    round_trips(&PutWorkflowDraft {
        workflow_id: WorkflowId::carried("feature"),
        source: String::from("kit"),
        project: None,
        text: String::from("{half"),
        summary: Some(String::from("by hand")),
        by: None,
        change: None,
    });
    let plain = round_trips(&PutWorkflowDraft { workflow_id: WorkflowId::carried("feature"), source: String::from("kit"), text: String::from("{}"), ..Default::default() });
    assert!(!plain.contains(r#""by""#) && !plain.contains("change"), "a put by you names neither: {plain}");
    let agent = round_trips(&PutWorkflowDraft {
        workflow_id: WorkflowId::carried("feature"),
        source: String::from("kit"),
        text: String::from("{rebased}"),
        by: Some(DraftBy::Agent),
        change: Some(7),
        ..Default::default()
    });
    assert!(agent.contains(r#""by":"agent""#) && agent.contains(r#""change":7"#), "{agent}");
    round_trips(&WorkflowDraftDiscarded { workflow_id: WorkflowId::carried("feature"), source: String::from("project"), project: Some(String::from("storefront")) });
    for scope in [None, Some(WorkflowScope::Kit), Some(WorkflowScope::Project { id: String::from("storefront") })] {
        for overwrite in [false, true] {
            round_trips(&PublishWorkflowDraft { workflow_id: WorkflowId::carried("feature"), source: String::from("armada"), project: None, scope: scope.clone(), overwrite });
        }
    }
    round_trips(&WorkflowPublished {
        saved: WorkflowSaved {
            workflow_id: WorkflowId::carried("feature"),
            scope: WorkflowScope::Kit,
            file: String::from("/kit/workflows/feature.json"),
            replaced: false,
            runs_from: Some(String::from("kit")),
        },
        version: 2,
        pinned: vec![SessionId::carried("s1")],
    });
    let put: PutWorkflowDraft = decode("a put", br#"{"workflow_id":"feature","source":"kit","text":"{}"}"#).expect("reads without project or summary");
    assert_eq!((put.project, put.summary), (None, None));
    let publish: PublishWorkflowDraft = decode("a publish", br#"{"workflow_id":"feature","source":"armada"}"#).expect("reads without a scope");
    assert_eq!((publish.scope, publish.overwrite), (None, false), "a replace is something a caller says");
}

#[test]
fn a_piece_draft_round_trips_and_a_body_without_a_scope_is_everywhere() {
    round_trips(&PieceDraft {
        kind: LifecyclePieceKind::Rule,
        id: String::from("ticket_to_review"),
        scope: PieceScope::Within { id: String::from("my-store") },
        text: String::from(r#"{"kind":"rule"}"#),
        base: String::new(),
        unfit: Some(String::from("the rule names no state")),
        changes: changes(),
        started_at: at("09:00:00"),
        updated_at: at("09:05:00"),
    });
    let put: PutPieceDraft = decode("a put", br#"{"kind":"lifecycle","id":"ticket","text":"{}"}"#).expect("reads without a scope");
    assert_eq!(put.scope, PieceScope::Everywhere);
    round_trips(&put);
    let publish: PublishPieceDraft = decode("a publish", br#"{"kind":"autonomy","id":"mine"}"#).expect("reads without a scope");
    assert_eq!(publish.scope, PieceScope::Everywhere);
    round_trips(&publish);
    round_trips(&PieceDraftDiscarded { kind: LifecyclePieceKind::Autonomy, id: String::from("mine"), scope: PieceScope::Everywhere });
    round_trips(&PiecePublished {
        saved: LifecyclePieceSaved {
            kind: LifecyclePieceKind::Rule,
            id: String::from("ticket_to_review"),
            scope: PieceScope::Everywhere,
            file: String::from("/kit/rules/ticket_to_review.yml"),
            replaced: true,
        },
    });
}
