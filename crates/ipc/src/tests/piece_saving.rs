//! Saving, deleting and committing a Workspace, lifecycle, rule or autonomy default: the spellings
//! Bridge sends and reads, and what an older caller that says less still means.

use std::collections::BTreeMap;

use crate::{
    decode, encode, AutonomyToSave, CommitToRepository, DeleteLifecyclePiece, LifecycleDefinition, LifecyclePiece,
    LifecyclePieceDeleted, LifecyclePieceKind, LifecyclePieceSaved, LifecycleStateDefinition, ManifestId,
    PieceScope, PieceToCommit, SaveLifecyclePiece, StateReadFrom, WorkspaceToSave,
};

fn a_lifecycle() -> LifecycleDefinition {
    LifecycleDefinition {
        id: "ticket".to_string(),
        name: "Ticket".to_string(),
        scope: PieceScope::Everywhere,
        states: vec![LifecycleStateDefinition {
            id: "todo".to_string(),
            label: "To do".to_string(),
            read: StateReadFrom::Tracker { words: vec!["Backlog".to_string()] },
        }],
        transitions: Vec::new(),
        ends: vec!["todo".to_string()],
    }
}

#[test]
fn each_kind_of_piece_is_tagged_by_kind_and_carries_what_a_read_answered() {
    let pieces = [
        (LifecyclePiece::Lifecycle { lifecycle: a_lifecycle() }, "lifecycle", "lifecycle"),
        (
            LifecyclePiece::Workspace {
                workspace: WorkspaceToSave {
                    id: "my-store".to_string(),
                    name: "My store".to_string(),
                    repositories: vec![ManifestId::carried("01ABC")],
                },
            },
            "workspace",
            "workspace",
        ),
        (
            LifecyclePiece::Autonomy {
                autonomy: AutonomyToSave {
                    id: "defaults".to_string(),
                    scope: PieceScope::Within { id: "my-store".to_string() },
                    defaults: BTreeMap::from([("forge_write".to_string(), "asks".to_string())]),
                },
            },
            "autonomy",
            "autonomy",
        ),
    ];
    for (piece, kind, key) in pieces {
        let bytes = encode(&piece).expect("encodes");
        assert!(bytes.contains(&format!(r#""kind":"{kind}""#)) && bytes.contains(&format!(r#""{key}":{{"#)), "{bytes}");
        let read: LifecyclePiece = decode("a piece", bytes.as_bytes()).expect("reads back");
        assert_eq!(read, piece);
    }
}

#[test]
fn a_save_that_does_not_say_overwrite_does_not_overwrite() {
    let said = r#"{"piece":{"kind":"workspace","workspace":{"id":"my-store","name":"My store","repositories":["01ABC"]}}}"#;
    let save: SaveLifecyclePiece = decode("a save", said.as_bytes()).expect("reads");
    assert!(!save.overwrite);
}

#[test]
fn a_delete_and_an_autonomy_that_name_no_scope_mean_everywhere() {
    let delete: DeleteLifecyclePiece = decode("a delete", br#"{"kind":"rule","id":"nudge"}"#).expect("reads");
    assert_eq!((delete.kind, delete.scope), (LifecyclePieceKind::Rule, PieceScope::Everywhere));
    let autonomy: AutonomyToSave = decode("autonomy", br#"{"id":"defaults","defaults":{"draft":"acts"}}"#).expect("reads");
    assert_eq!(autonomy.scope, PieceScope::Everywhere);
}

#[test]
fn the_answers_spell_the_kind_and_the_scope() {
    let saved = LifecyclePieceSaved {
        kind: LifecyclePieceKind::Rule,
        id: "nudge".to_string(),
        scope: PieceScope::Within { id: "my-store".to_string() },
        file: "/kit/rules/nudge.my-store.yml".to_string(),
        replaced: true,
    };
    let bytes = encode(&saved).expect("encodes");
    assert!(bytes.contains(r#""kind":"rule""#) && bytes.contains(r#""scope":{"kind":"within","id":"my-store"}"#), "{bytes}");
    let deleted = LifecyclePieceDeleted { kind: LifecyclePieceKind::Workspace, id: "my-store".to_string(), scope: PieceScope::Everywhere, file: "/kit/workspaces/my-store.yml".to_string() };
    let bytes = encode(&deleted).expect("encodes");
    assert!(bytes.contains(r#""kind":"workspace""#), "{bytes}");
    for kind in [LifecyclePieceKind::Lifecycle, LifecyclePieceKind::Rule, LifecyclePieceKind::Workspace, LifecyclePieceKind::Autonomy] {
        assert_eq!(encode(&kind).expect("encodes"), format!("\"{}\"", kind.as_wire()));
    }
}

#[test]
fn a_commit_names_a_piece_by_its_identity_and_says_remove_only_when_it_means_it() {
    let said = |piece: &str| format!(r#"{{"repository":"/work/shop","piece":{piece}}}"#);
    let commit: CommitToRepository = decode("a commit", said(r#"{"kind":"lifecycle","id":"ticket"}"#).as_bytes()).expect("reads");
    assert_eq!(commit.piece, PieceToCommit::Lifecycle { id: "ticket".to_string(), scope: PieceScope::Everywhere });
    assert!(!commit.overwrite && !commit.remove, "an older caller's commit copies and replaces nothing");

    let within = r#"{"kind":"lifecycle_rule","id":"nudge","scope":{"kind":"within","id":"my-store"}}"#;
    let commit: CommitToRepository = decode("a commit", said(within).as_bytes()).expect("reads");
    assert_eq!(commit.piece, PieceToCommit::LifecycleRule { id: "nudge".to_string(), scope: PieceScope::Within { id: "my-store".to_string() } });

    let commit: CommitToRepository = decode("a commit", said(r#"{"kind":"workspace","id":"my-store"}"#).as_bytes()).expect("reads");
    assert_eq!(commit.piece, PieceToCommit::Workspace { id: "my-store".to_string() });
    let commit: CommitToRepository = decode("a commit", said(r#"{"kind":"autonomy","id":"defaults"}"#).as_bytes()).expect("reads");
    assert_eq!(commit.piece, PieceToCommit::Autonomy { id: "defaults".to_string(), scope: PieceScope::Everywhere });

    let removing = r#"{"repository":"/work/shop","piece":{"kind":"workspace","id":"my-store"},"remove":true}"#;
    let commit: CommitToRepository = decode("a commit", removing.as_bytes()).expect("reads");
    assert!(commit.remove);
}
