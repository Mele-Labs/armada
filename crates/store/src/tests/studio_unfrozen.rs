//! No Studio node is `frozen`: V86. Decided with the owner, 1 Oct 2026.
//!
//! Tested from the version before it, `studio_sketches`' way: a V85 file
//! written through raw SQL, so what migrates is a node the owner already has.

use core_model::{StudioId, StudioNodeKind, StudioNodeState, Ulid};
use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{open, TempDir};

const AT: &str = "2026-10-01T09:00:00.000Z";

/// A V85 file holding a Finding its scout ended and an Outline, both frozen.
fn a_v85_file(dir: &TempDir) {
    let conn = Connection::open(dir.db()).expect("a file");
    for migration in &MIGRATIONS[..85] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute_batch(&format!(
        "INSERT INTO armada_meta (key, value) VALUES ('{SCHEMA_VERSION_KEY}', '85');
         INSERT INTO studios VALUES ('01OLD', 'armada', NULL, '{AT}', '{AT}', NULL);
         INSERT INTO studio_nodes VALUES ('01FINDING', '01OLD', 'finding', 'frozen',
             '{{\"asked\":\"Where do the legend colours come from?\",
                \"checkout\":{{\"commit\":\"abc123\",\"uncommitted\":false}},
                \"read\":[\"tokens.css\"],\"learned\":\"From tokens.css\",
                \"ended\":{{\"outcome\":\"answered\",\"cost_micros\":310000}}}}',
             0, 0, '{AT}', 'person');
         INSERT INTO studio_nodes VALUES ('01OUTLINE', '01OLD', 'outline', 'frozen',
             '{{\"body\":\"Capture on Bridge\"}}', 300, 0, '{AT}', 'person');"
    ))
    .expect("a Studio as V85 wrote it");
}

/// **A frozen Finding reads back with no state and a frozen Outline as a
/// draft**, each keeping what it held.
#[test]
fn a_frozen_finding_and_a_frozen_outline_migrate_off_frozen() {
    let dir = TempDir::new();
    a_v85_file(&dir);

    let store = open(&dir);
    let graph = store
        .studio(&StudioId::carried(Ulid::carried("01OLD")))
        .expect("migrates and reads");
    let state = |kind| {
        graph
            .nodes
            .iter()
            .find(|node| node.kind() == kind)
            .expect("the node")
            .state()
    };
    assert_eq!(
        state(StudioNodeKind::Finding),
        None,
        "an ended Finding holds no state"
    );
    assert_eq!(state(StudioNodeKind::Outline), Some(StudioNodeState::Draft));
}

/// **The column refuses `frozen` after V86**, so no write can bring it back.
#[test]
fn the_state_column_refuses_frozen() {
    let dir = TempDir::new();
    a_v85_file(&dir);
    drop(open(&dir));

    let conn = Connection::open(dir.db()).expect("a file");
    let refused = conn.execute(
        "UPDATE studio_nodes SET state = 'frozen' WHERE id = '01OUTLINE'",
        [],
    );
    assert!(refused.is_err(), "the CHECK no longer admits frozen");
}
