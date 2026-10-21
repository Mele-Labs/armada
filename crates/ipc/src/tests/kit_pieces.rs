//! Kit's Add and Remove on the wire, in the bytes `packages/protocol/src/kit.ts` reads: tagged by
//! `found` and `state`, and a plugin's `trashed` sent as `null` rather than left out.

use crate::{InstallKitPiece, KitInventory, KitPieceInstalled, KitPieceKind, KitPieceRemoved, KitSourceFound};

fn empty() -> KitInventory {
    KitInventory { harness: String::from("h"), home: String::from("/home"), present: true, kinds: Vec::new() }
}

#[test]
fn a_plugin_found_is_tagged_by_found_with_every_field_the_mock_reads() {
    let found = KitSourceFound::Plugin {
        name: String::from("impeccable"),
        marketplace: String::from("impeccable"),
        repository: String::from("pbakaus/impeccable"),
        skills: vec![String::from("impeccable")],
        commands: vec![String::from("one"), String::from("two")],
    };
    assert_eq!(
        crate::encode(&found).unwrap(),
        r#"{"found":"plugin","name":"impeccable","marketplace":"impeccable","repository":"pbakaus/impeccable","skills":["impeccable"],"commands":["one","two"]}"#
    );
    let unknown = crate::encode(&KitSourceFound::Unknown { why: String::from("no") }).unwrap();
    assert_eq!(unknown, r#"{"found":"unknown","why":"no"}"#);
}

#[test]
fn a_failed_install_carries_what_was_taken_back() {
    let failed = KitPieceInstalled::Failed {
        command: String::from("c"),
        exit_code: 1,
        error: String::from("e"),
        taken_back: vec![String::from("the broken-plugin marketplace")],
    };
    assert_eq!(
        crate::encode(&failed).unwrap(),
        r#"{"state":"failed","command":"c","exit_code":1,"error":"e","taken_back":["the broken-plugin marketplace"]}"#
    );
}

#[test]
fn a_removed_plugin_says_trashed_null_and_a_body_must_say_it_too() {
    let removed = KitPieceRemoved { setup: empty(), trashed: None };
    let bytes = crate::encode(&removed).unwrap();
    assert!(bytes.ends_with(r#""trashed":null}"#), "{bytes}");
    assert!(crate::decode::<KitPieceRemoved>("removed", br#"{"setup":{"harness":"h","home":"/","present":true,"kinds":[]}}"#).is_err());
}

#[test]
fn an_install_body_reads_as_the_mock_sends_it() {
    let asked: InstallKitPiece =
        crate::decode("an install", br#"{"source":"pbakaus/impeccable","kind":"plugin","manifest":false}"#).unwrap();
    assert_eq!((asked.kind, asked.manifest), (KitPieceKind::Plugin, false));
}
