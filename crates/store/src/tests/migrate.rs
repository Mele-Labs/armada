//! The migrations after the first, run forward over a file an earlier build
//! left behind.
//!
//! A version 1 file is built here the way the world would produce one — by
//! running the first migration and nothing else. The Job tables stay in the
//! schema and nothing reads them, so what is held here is that a file at an
//! old version reaches the current one, and that a table a later migration
//! added works once it has.

use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{at, TempDir};
use crate::{PortClaim, PortClaimant, Store, KNOWN_SCHEMA_VERSION};

/// A file at version 1 and nothing later.
fn version_one(dir: &TempDir) {
    let conn = Connection::open(dir.db()).expect("a file to put version 1 in");
    conn.execute_batch(MIGRATIONS[0].sql)
        .expect("the first migration is the whole of version 1");
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '1')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 1");
}

#[test]
fn a_version_one_file_is_brought_forward_rather_than_refused() {
    let dir = TempDir::new();
    version_one(&dir);

    let store = Store::open(&dir.db()).expect("a version 1 file opens and is migrated");
    assert_eq!(recorded_version(&store), KNOWN_SCHEMA_VERSION.to_string());
}

/// How many names the file has applied, as a string so it compares with
/// [`KNOWN_SCHEMA_VERSION`].
pub(super) fn recorded_version(store: &Store) -> String {
    store
        .conn
        .query_row("SELECT count(*) FROM armada_migrations", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("the applied names")
        .to_string()
}

/// A file at version 44, `main`'s shipped schema before `port_claims`. A file
/// already migrated to 44 by an earlier build must still find its next
/// migration where it left off, not have one inserted underneath it.
fn version_forty_four(dir: &TempDir) {
    let conn = Connection::open(dir.db()).expect("a file to put version 44 in");
    for migration in &MIGRATIONS[..44] {
        conn.execute_batch(migration.sql).expect("a migration");
    }
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '44')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 44");
}

/// A store already at `main`'s shipped schema version migrates forward to
/// include `port_claims`, and the table it adds works once it has.
#[test]
fn a_store_at_version_forty_four_migrates_forward_to_port_claims() {
    let dir = TempDir::new();
    version_forty_four(&dir);

    let mut store = Store::open(&dir.db()).expect("a version 44 file opens and is migrated");
    assert_eq!(
        recorded_version(&store),
        KNOWN_SCHEMA_VERSION.to_string(),
        "migrated all the way, not stopped at 44"
    );

    let claim = PortClaim {
        claimant: PortClaimant::Checkout(String::from("/repos/one")),
        base: 41_000,
        width: 8,
        claimed_at: at("2026-08-26T09:00:00.000Z"),
    };
    store.claim_port_span(&claim).expect("port_claims exists");
    assert_eq!(
        store
            .port_span_for_checkout("/repos/one")
            .expect("the read succeeds"),
        Some(claim)
    );
}
