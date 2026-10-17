//! What a table rebuild has to carry, checked on the one migration that does
//! one.
//!
//! **V50 rebuilds `port_claims` rather than altering it.** SQLite cannot widen
//! a `CHECK` in place, and [`V45`](crate::ports::V45)'s spells "exactly one of
//! a Job or the main checkout" — so admitting Fleet's own listener as a third
//! claimant means building the table beside the old one, carrying every row
//! across and dropping the original.
//!
//! **A rebuild that quietly lost rows would be a Job losing the port a Command
//! is already bound to**, and the main checkout losing the span a server is
//! running on. That is a different question from the ones
//! `crate::tests::migrate` asks — which are about what a *column* means to a
//! row written before it — so it is asked here.

use rusqlite::Connection;

use super::migrate::recorded_version;
use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{at, TempDir};
use crate::{PortClaim, PortClaimant, Store, KNOWN_SCHEMA_VERSION};

/// A file at version 49 — the schema before Fleet's own listener became a
/// claimant — carrying one of each claim `port_claims` could then hold. The
/// inserts are raw SQL because the column the current build writes does not
/// exist on this file yet.
fn version_forty_nine(dir: &TempDir, job: &str) {
    let conn = Connection::open(dir.db()).expect("a file to put version 49 in");
    for migration in &MIGRATIONS[..49] {
        conn.execute_batch(migration.sql).expect("a migration");
    }
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '49')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 49");
    conn.execute(
        "INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
             atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES (?1, 'a job holding a port span', 'running', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                   '', '[]', 0, '2026-08-26T09:00:00.000Z')",
        (job,),
    )
    .expect("a Job as version 49 wrote them");
    conn.execute(
        "INSERT INTO port_claims (job_id, main_checkout, base, width, claimed_at)
         VALUES (?1, NULL, 41000, 8, '2026-08-26T09:00:00.000Z')",
        (job,),
    )
    .expect("the Job's span, claimed before the rebuild");
    conn.execute(
        "INSERT INTO port_claims (job_id, main_checkout, base, width, claimed_at)
         VALUES (NULL, 1, 41008, 8, '2026-08-26T09:00:00.000Z')",
        (),
    )
    .expect("the main checkout's span, claimed before the rebuild");
}

/// **The rebuild carries every row across**, including the Job's, which stays
/// in the table and is not read.
#[test]
fn a_store_at_version_forty_nine_keeps_every_claim_through_the_rebuild() {
    let dir = TempDir::new();
    version_forty_nine(&dir, "01PORTREBUILD00000000001");

    let mut store = Store::open(&dir.db()).expect("a version 49 file opens and is migrated");
    assert_eq!(
        recorded_version(&store),
        KNOWN_SCHEMA_VERSION.to_string(),
        "migrated all the way, not stopped at 49"
    );

    // The Job's claim carried across with its `job_id`, and nothing reads it. V57
    // then releases the main checkout's claim, which named no repository.
    let carried: i64 = store
        .conn
        .query_row("SELECT COUNT(*) FROM port_claims WHERE job_id IS NOT NULL", [], |row| row.get(0))
        .expect("counts");
    assert_eq!(carried, 1, "the Job's claim survived the rebuild");
    let every = store.every_port_claim().expect("read");
    assert!(every.is_empty(), "a claim a Job held is not read: {every:?}");

    // And the third claimant the rebuild exists for, which the old `CHECK`
    // could not have held.
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::FleetListener,
            base: 41_016,
            width: 1,
            claimed_at: at("2026-08-26T09:00:00.000Z"),
        })
        .expect("Fleet's own listener can claim once the rebuild has run");
    assert_eq!(store.every_port_claim().expect("read").len(), 1);
}
