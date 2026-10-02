//! A Zone kept, and what a frame holds: measured from its corner, moved with
//! it in one write, refused where it cannot hold a kind, and lifted out onto
//! the board when the frame is deleted. `#1620`.

use core_model::{
    ManifestId, Studio, StudioAuthor, StudioEdgeId, StudioId, StudioNode, StudioNodeContent,
    StudioNodeId, StudioNodeKind, StudioPosition, Timestamp, Ulid,
};

use crate::tests::{open, TempDir};
use crate::{Store, StudioError};

fn at(minute: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-10-02T09:{minute:02}:00.000Z"))
}

fn id(id: &str) -> StudioNodeId {
    StudioNodeId::carried(Ulid::carried(id))
}

fn a_studio(store: &mut Store) -> StudioId {
    let studio = Studio {
        id: StudioId::carried(Ulid::carried("01STUDIO")),
        manifest_id: ManifestId::carried(Ulid::carried("armada")),
        name: None,
        named_by: None,
        created_at: at(0),
        touched_at: at(0),
    };
    store.create_studio(&studio).expect("kept");
    studio.id
}

fn node(name: &str, content: StudioNodeContent, x: i64, y: i64) -> StudioNode {
    StudioNode::added(
        id(name),
        content,
        StudioPosition { x, y },
        at(1),
        StudioAuthor::Person,
    )
}

fn note(said: &str) -> StudioNodeContent {
    StudioNodeContent::Note {
        said: said.to_string(),
        capture: None,
    }
}

/// A Zone at (100, 100) holding a Note 40 across and 60 down.
fn a_zone_holding_a_note(store: &mut Store, studio: &StudioId) {
    store
        .add_studio_node(
            studio,
            &node("01AZONE", StudioNodeContent::Zone, 100, 100),
            None,
            &at(1),
        )
        .expect("a Zone is kept");
    let inside = node("01NOTE", note("The chip keeps its count"), 40, 60)
        .placed(Some(id("01AZONE")), StudioPosition { x: 40, y: 60 });
    store
        .add_studio_node(studio, &inside, None, &at(1))
        .expect("a Note is kept inside it");
}

#[test]
fn a_zone_and_what_it_holds_read_back_after_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store);
    a_zone_holding_a_note(&mut store, &studio);
    let before = store.studio(&studio).expect("reads");
    drop(store);

    let after = open(&dir).studio(&studio).expect("reads after a reopen");
    assert_eq!(after, before);
    assert_eq!(after.nodes[0].kind(), StudioNodeKind::Zone);
    assert_eq!(after.nodes[1].within(), Some(&id("01AZONE")));
    assert_eq!(after.nodes[1].position(), StudioPosition { x: 40, y: 60 });
    assert_eq!(
        after.on_the_board(&id("01NOTE")),
        Some(StudioPosition { x: 140, y: 160 })
    );
}

#[test]
fn moving_a_zone_moves_what_it_holds_in_one_write() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store);
    a_zone_holding_a_note(&mut store, &studio);
    store
        .move_studio_node(
            &studio,
            &id("01AZONE"),
            None,
            StudioPosition { x: 500, y: -20 },
            &at(2),
        )
        .expect("moved");

    let graph = store.studio(&studio).expect("reads");
    assert_eq!(graph.nodes[1].position(), StudioPosition { x: 40, y: 60 });
    assert_eq!(
        graph.on_the_board(&id("01NOTE")),
        Some(StudioPosition { x: 540, y: 40 })
    );
}

#[test]
fn a_node_moves_into_a_zone_and_out_of_one() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store);
    a_zone_holding_a_note(&mut store, &studio);
    store
        .move_studio_node(
            &studio,
            &id("01NOTE"),
            None,
            StudioPosition { x: 900, y: 900 },
            &at(2),
        )
        .expect("taken out onto the board");
    let graph = store.studio(&studio).expect("reads");
    assert_eq!(graph.nodes[1].within(), None);

    store
        .move_studio_node(
            &studio,
            &id("01NOTE"),
            Some(&id("01AZONE")),
            StudioPosition { x: 10, y: 10 },
            &at(3),
        )
        .expect("put back in");
    let graph = store.studio(&studio).expect("reads");
    assert_eq!(graph.nodes[1].within(), Some(&id("01AZONE")));
}

#[test]
fn a_zone_holds_no_zone_and_a_note_holds_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store);
    a_zone_holding_a_note(&mut store, &studio);
    let second = node("01AZONE2", StudioNodeContent::Zone, 0, 0)
        .placed(Some(id("01AZONE")), StudioPosition { x: 0, y: 0 });
    let refused = store.add_studio_node(&studio, &second, None, &at(2));
    assert!(
        matches!(refused, Err(StudioError::CannotHold { .. })),
        "{refused:?}"
    );

    store
        .add_studio_node(
            &studio,
            &node("01AZONE2", StudioNodeContent::Zone, 0, 0),
            None,
            &at(2),
        )
        .expect("on the board");
    let refused = store.move_studio_node(
        &studio,
        &id("01AZONE2"),
        Some(&id("01NOTE")),
        StudioPosition { x: 0, y: 0 },
        &at(3),
    );
    assert!(
        matches!(refused, Err(StudioError::CannotHold { .. })),
        "{refused:?}"
    );
}

/// A Cluster made of two Notes in a Zone: drawn around them, inside the Zone.
fn a_cluster_of_two(store: &mut Store, studio: &StudioId) {
    a_zone_holding_a_note(store, studio);
    let second = node("01NOTE2", note("Overview says three"), 40, 260)
        .placed(Some(id("01AZONE")), StudioPosition { x: 40, y: 260 });
    store
        .add_studio_node(studio, &second, None, &at(1))
        .expect("kept");
    let cluster = node(
        "01CLUSTER",
        StudioNodeContent::Cluster {
            title: "Stale counts".to_string(),
        },
        20,
        20,
    )
    .placed(Some(id("01AZONE")), StudioPosition { x: 20, y: 20 });
    let first = id("01NOTE");
    let other = id("01NOTE2");
    store
        .add_studio_frame(
            studio,
            &cluster,
            &[
                (&first, StudioEdgeId::carried(Ulid::carried("01E1"))),
                (&other, StudioEdgeId::carried(Ulid::carried("01E2"))),
            ],
            &[
                (first.clone(), StudioPosition { x: 20, y: 40 }),
                (other.clone(), StudioPosition { x: 20, y: 240 }),
            ],
            &at(2),
        )
        .expect("a Cluster drawn around its Notes");
}

#[test]
fn a_cluster_frames_its_notes_and_keeps_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store);
    a_cluster_of_two(&mut store, &studio);
    let graph = store.studio(&studio).expect("reads");
    let framed = graph
        .nodes
        .iter()
        .find(|n| n.id() == &id("01NOTE"))
        .unwrap();
    assert_eq!(framed.within(), Some(&id("01CLUSTER")));
    assert_eq!(
        graph.on_the_board(&id("01NOTE")),
        Some(StudioPosition { x: 140, y: 160 }),
        "framing it did not move it on the board"
    );

    let refused = store.move_studio_node(
        &studio,
        &id("01NOTE"),
        Some(&id("01AZONE")),
        StudioPosition { x: 0, y: 0 },
        &at(3),
    );
    assert!(
        matches!(refused, Err(StudioError::StaysInItsCluster { .. })),
        "{refused:?}"
    );

    store
        .add_studio_node(&studio, &node("01NOTE3", note("Loose"), 0, 0), None, &at(3))
        .expect("kept");
    let refused = store.move_studio_node(
        &studio,
        &id("01NOTE3"),
        Some(&id("01CLUSTER")),
        StudioPosition { x: 0, y: 0 },
        &at(4),
    );
    assert!(
        matches!(refused, Err(StudioError::NotItsCluster { .. })),
        "{refused:?}"
    );
}

#[test]
fn deleting_frames_lifts_what_they_held_onto_the_board_where_it_was() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store);
    a_cluster_of_two(&mut store, &studio);
    store
        .remove_studio_nodes(&studio, &[id("01AZONE"), id("01CLUSTER")], &at(3))
        .expect("removed");

    let graph = store.studio(&studio).expect("reads");
    assert_eq!(graph.nodes.len(), 2);
    for note in &graph.nodes {
        assert_eq!(note.within(), None);
    }
    assert_eq!(
        graph.on_the_board(&id("01NOTE")),
        Some(StudioPosition { x: 140, y: 160 })
    );
    assert_eq!(
        graph.on_the_board(&id("01NOTE2")),
        Some(StudioPosition { x: 140, y: 360 })
    );
}
