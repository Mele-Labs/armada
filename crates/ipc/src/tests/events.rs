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
