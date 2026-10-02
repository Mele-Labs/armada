//! A Zone and what sits in a frame, on the wire: `{"kind":"zone"}` and
//! nothing else, `within` naming the frame a node sits in, and a move naming
//! the frame it is put down in. `#1620`, protocol 22.0.

use core_model::{ManifestId, StudioGraph, StudioId, StudioNodeId, Timestamp, Ulid};

use crate::{decode, encode, MoveStudioNode, Studio};

/// A node inside one names it in `within`, with its position measured from the
/// Zone's corner, and a node on the board sends no `within` at all.
#[test]
fn a_zone_and_a_node_within_it_round_trip() {
    let at = Timestamp::from_rfc3339("2026-10-02T09:00:00.000Z");
    let zone = core_model::StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01ZONE")),
        core_model::StudioNodeContent::Zone,
        core_model::StudioPosition { x: 100, y: 100 },
        at.clone(),
        core_model::StudioAuthor::Person,
    );
    let inside = core_model::StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01NOTE")),
        core_model::StudioNodeContent::Note {
            said: "The chip keeps its count".to_string(),
            capture: None,
        },
        core_model::StudioPosition { x: 0, y: 0 },
        at.clone(),
        core_model::StudioAuthor::Person,
    )
    .placed(
        Some(zone.id().clone()),
        core_model::StudioPosition { x: 40, y: 60 },
    );
    let graph = StudioGraph {
        studio: core_model::Studio {
            id: StudioId::carried(Ulid::carried("01STUDIO")),
            manifest_id: ManifestId::carried(Ulid::carried("armada")),
            name: None,
            named_by: None,
            created_at: at.clone(),
            touched_at: at,
        },
        nodes: vec![zone, inside],
        edges: vec![],
    };
    let studio = Studio::of(&graph);
    let json = encode(&studio).expect("plain data");
    assert!(
        json.contains(r#"{"id":"01ZONE","kind":"zone","position":{"x":100,"y":100}"#),
        "{json}"
    );
    assert!(
        json.contains(r#""within":"01ZONE","position":{"x":40,"y":60}"#),
        "{json}"
    );
    assert_eq!(json.matches("\"within\"").count(), 1, "{json}");
    assert_eq!(
        decode::<Studio>("a Studio", json.as_bytes()).expect("round-trips"),
        studio
    );
}

/// A move names the frame it is put down in, and one naming none is a move
/// onto the board.
#[test]
fn a_move_names_its_frame_or_none() {
    let into: MoveStudioNode = decode(
        "a move",
        br#"{"node_id":"01NOTE","within":"01ZONE","position":{"x":1,"y":2}}"#,
    )
    .expect("decodes");
    assert_eq!(
        into.within.map(|zone| zone.to_domain()),
        Some(StudioNodeId::carried(Ulid::carried("01ZONE")))
    );
    let onto: MoveStudioNode = decode(
        "a move",
        br#"{"node_id":"01NOTE","position":{"x":1,"y":2}}"#,
    )
    .expect("decodes");
    assert_eq!(onto.within, None);
}
