//! A prototype part rides on an artifact's part list: `PartContent` is tagged by `kind`, and the prototype's own
//! fields sit beside the tag, as native and Pocket read them.

use crate::{decode, encode, ArtifactPart, Instant, PartContent, PartKind, PartState, PrototypePart, SceneFixture};

fn part(prototype: PrototypePart) -> ArtifactPart {
    ArtifactPart {
        id: String::from("ready"),
        content: PartContent::Prototype(prototype),
        asks: true,
        state: PartState::Open,
        revision: 2,
        drawn: String::from("row-3"),
        added: Instant::carried("2026-11-23T09:00:00.000Z"),
        answer: None,
    }
}

#[test]
fn a_prototype_part_reads_back_whole_with_its_fields_beside_the_kind() {
    let held = part(PrototypePart {
        scene: String::from("ready.html"),
        markup: String::from("<scene><fixture name=\"jobs\" src=\"jobs.json\"/></scene>"),
        fixtures: vec![SceneFixture { name: String::from("jobs.json"), text: String::from("[1,2]") }],
        scene_state: Some(String::from("empty")),
        trouble: None,
    });
    let sent = encode(&held).expect("a part encodes");
    assert!(sent.contains("\"kind\":\"prototype\"") && sent.contains("\"scene\":\"ready.html\""), "the fields sit beside the kind: {sent}");
    assert!(sent.contains("\"state\":\"open\"") && sent.contains("\"scene_state\":\"empty\""), "the part's state and the scene's keep their own keys: {sent}");
    assert!(!sent.contains("trouble"), "an absent trouble is left off: {sent}");
    let read: ArtifactPart = decode("a part", sent.as_bytes()).expect("a prototype part reads back");
    assert_eq!(read, held);
    assert_eq!(read.content.kind(), PartKind::Prototype);
    assert_eq!(PartKind::Prototype.word(), "prototype");
}

#[test]
fn a_prototype_without_a_state_or_trouble_reads_as_none() {
    let sent = r#"{"id":"ready","kind":"prototype","scene":"a.html","markup":"","fixtures":[],"asks":true,"state":"open","revision":1,"drawn":"d","added":"2026-11-23T09:00:00.000Z"}"#;
    let read: ArtifactPart = decode("a part", sent.as_bytes()).expect("a part with no state reads");
    let PartContent::Prototype(prototype) = read.content else { panic!("a prototype part") };
    assert_eq!((prototype.scene_state, prototype.trouble), (None, None));
}
