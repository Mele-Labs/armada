//! A File on a Studio: V84. Decided with the owner, 1 Oct 2026.
//!
//! Tested from the version before it, `studio_forge`'s way: a V83 file with a
//! Studio on it, written through raw SQL, so what is measured is a file the
//! owner already has rather than one this build made.

use core_model::{StudioId, StudioNodeContent, Ulid};
use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{open, TempDir};

const AT: &str = "2026-10-01T09:00:00.000Z";

/// A V83 file holding one Studio, a Link and an Issue, and a `produced` edge
/// joining them — the shape V84 rebuilds both tables under.
fn a_v83_file(dir: &TempDir) {
    let conn = Connection::open(dir.db()).expect("a file");
    for migration in &MIGRATIONS[..83] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute_batch(&format!(
        "INSERT INTO armada_meta (key, value) VALUES ('{SCHEMA_VERSION_KEY}', '83');
         INSERT INTO studios VALUES ('01OLD', 'armada', 'Named then', '{AT}', '{AT}', 'person');
         INSERT INTO studio_nodes VALUES ('01LINK', '01OLD', 'link', NULL,
             '{{\"address\":\"https://example.invalid/a-board\"}}', 40, 80, '{AT}', 'person');
         INSERT INTO studio_nodes VALUES ('01ISSUE', '01OLD', 'issue', NULL,
             '{{\"address\":\"https://example.invalid/o/r/issues/7\",\"number\":\"7\"}}',
             9, 0, '{AT}', NULL);
         INSERT INTO studio_edges VALUES ('01E', '01OLD', '01LINK', '01ISSUE', 'produced',
             'accepted', '{AT}', 'person');"
    ))
    .expect("a Studio as V83 wrote it");
}

fn old() -> StudioId {
    StudioId::carried(Ulid::carried("01OLD"))
}

/// **V84 rebuilds both tables and loses nothing**, for the reason V79's own
/// test gives: the nodes are dropped under their edges, and the edges cascade.
/// And a File, which V83's `CHECK` refused, writes and reads back as its path.
#[test]
fn a_studio_written_before_the_file_kind_keeps_everything_and_takes_a_file() {
    let dir = TempDir::new();
    a_v83_file(&dir);

    let mut store = open(&dir);
    let graph = store.studio(&old()).expect("migrates and reads");
    assert_eq!(graph.nodes.len(), 2, "both nodes survive the rebuild");
    assert_eq!(graph.edges.len(), 1, "and the edge between them does");
    assert_eq!(graph.edges[0].id().as_str(), "01E");
    assert_eq!(
        graph.studio.touched_at.as_str(),
        AT,
        "a migration is nobody's write, so the list does not reorder"
    );

    let content = StudioNodeContent::file("crates/fleet/src/briefing.rs");
    let node = core_model::StudioNode::added(
        core_model::StudioNodeId::carried(Ulid::carried("01FILE")),
        content.clone(),
        core_model::StudioPosition { x: 0, y: 0 },
        core_model::Timestamp::from_rfc3339(AT),
        core_model::StudioAuthor::Person,
    );
    store
        .add_studio_node(
            &old(),
            &node,
            None,
            &core_model::Timestamp::from_rfc3339(AT),
        )
        .expect("a File is a kind V84 admits");
    let read = store.studio(&old()).expect("reads");
    let file = read
        .nodes
        .iter()
        .find(|one| one.id().as_str() == "01FILE")
        .expect("the File");
    assert_eq!(file.content(), &content);
    assert_eq!(file.state(), None, "a File holds no state");

    let conn = Connection::open(dir.db()).expect("the same file");
    let stored: (String, String) = conn
        .query_row(
            "SELECT kind, content FROM studio_nodes WHERE id = '01FILE'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the row");
    assert_eq!(
        stored,
        (
            String::from("file"),
            String::from(r#"{"path":"crates/fleet/src/briefing.rs"}"#)
        ),
        "the path and nothing else"
    );
}
