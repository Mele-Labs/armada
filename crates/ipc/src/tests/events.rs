//! What the stream Bridge draws from carries.
//!
//! **The kinds are the dotted names `crates/ipc/operations/` keys them
//! under**, so a rule can compare the two without a mapping in between. And
//! nothing generates the DTO types from this crate yet, so these cases are
//! what pin the spellings a TypeScript union mirrors by hand — a variant
//! renamed here fails in Rust rather than as a branch that never runs.

use crate::{decode, encode, StreamMessage};

#[test]
fn an_event_travels_under_the_name_the_inventory_declares() {
    let message = StreamMessage::Event(crate::Delivered {
        cursor: crate::Cursor::at(7),
        event: crate::Event::StudioDeleted(crate::StudioDeleted {
            id: crate::StudioId::carried("01STUDIO"),
            manifest_id: crate::ManifestId::carried("01MF"),
        }),
    });
    let json = encode(&message).expect("plain data");
    assert!(json.contains("\"message\":\"event\""), "{json}");
    assert!(json.contains("\"kind\":\"studio.deleted\""), "{json}");
    assert_eq!(
        decode::<StreamMessage>("stream message", json.as_bytes()).expect("it round-trips"),
        message
    );
}

/// A kind that was retired with the Jobs does not read: a peer sending one is
/// refused rather than guessed at.
#[test]
fn a_retired_kind_is_refused() {
    let body = br#"{"message":"event","cursor":0,"event":{"kind":"job.state_changed"}}"#;
    assert!(decode::<StreamMessage>("stream message", body).is_err());
}

/// A background act's failure travels as `act.failed`, the act under its snake_case name, the refusal under
/// `error`, and an absent repository or Job leaves no key.
#[test]
fn a_failed_background_act_travels_as_act_failed() {
    let message = StreamMessage::Event(crate::Delivered {
        cursor: crate::Cursor::at(3),
        event: crate::Event::ActFailed(crate::ActFailed {
            act: crate::BackgroundAct::PullRequestPress,
            manifest_id: Some(crate::ManifestId::carried("01MF")),
            session_id: None,
            error: crate::WireError::raised("fleet.merge_refused", "The merge was refused", crate::RunId::carried("run")),
        }),
    });
    let json = encode(&message).expect("plain data");
    assert!(json.contains("\"kind\":\"act.failed\""), "{json}");
    assert!(json.contains("\"act\":\"pull_request_press\""), "{json}");
    assert!(json.contains("\"manifest_id\":\"01MF\""), "{json}");
    assert!(!json.contains("session_id"), "{json}");
    assert!(json.contains("\"code\":\"fleet.merge_refused\""), "{json}");
    assert_eq!(decode::<StreamMessage>("stream message", json.as_bytes()).expect("it round-trips"), message);
    let body = br#"{"message":"event","cursor":0,"event":{"kind":"act.failed","act":"made_up","error":{"code":"c","message":"m","run_id":"r","fields":{},"chain":[]}}}"#;
    assert!(decode::<StreamMessage>("stream message", body).is_err(), "an act Bridge does not know is refused");
}
