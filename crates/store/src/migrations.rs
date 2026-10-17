//! What a migration is, and the table that records which ones a file has had.
//!
//! A migration is a file in `migrations/`, listed after the frozen 113 in
//! `legacy_migrations.rs`. It has a stable name, not a position, so two
//! branches that each add one never take the same number or touch one file, and
//! a file written by a build with one more name than this one is read by it
//! when that name was additive. `docs/practices/store-migrations.md` has the
//! rules and why.

use rusqlite::Connection;

use crate::legacy_migrations::LEGACY;

include!(concat!(env!("OUT_DIR"), "/file_migrations.rs"));

/// The frozen legacy entries, then the files by timestamp. Applying is by
/// missing name, so the order between independent branches does not matter.
pub(crate) const MIGRATIONS: &[Migration] = &ALL;
const ALL: [Migration; LEGACY.len() + FROM_FILES.len()] = {
    let mut all = [Migration::additive("", ""); LEGACY.len() + FROM_FILES.len()];
    let mut at = 0;
    while at < LEGACY.len() {
        all[at] = LEGACY[at];
        at += 1;
    }
    while at < all.len() {
        all[at] = FROM_FILES[at - LEGACY.len()];
        at += 1;
    }
    all
};
const _: () = assert!(LEGACY.len() == LEGACY_COUNT);

/// The key the old numbered list recorded its applied count under. It is read
/// once, to convert a file written before names, and written only for the first
/// [`LEGACY_COUNT`] entries so a build that predates names still reads a fresh
/// file.
pub const SCHEMA_VERSION_KEY: &str = "schema_version";

/// How many entries the numbered list held when names replaced it. A file
/// recording `n` has the first `n` of these applied; nothing counts past it.
pub const LEGACY_COUNT: usize = 113;

/// How many migrations this build knows.
pub const KNOWN_SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

/// One migration. **Never edit an entry's SQL once it is on `main`**: a file
/// that applied it is assumed to hold exactly what it made.
#[derive(Clone, Copy, Debug)]
pub struct Migration {
    /// Unique and permanent: `module.slug`, never a position.
    pub name: &'static str,
    pub sql: &'static str,
    /// Drops, renames or rewrites something an older build reads. A migration
    /// is additive unless it says so, and a test refuses an additive one whose
    /// SQL is not.
    pub breaking: bool,
}

impl Migration {
    /// Creates tables and indexes, or adds columns with defaults, and nothing else.
    pub const fn additive(name: &'static str, sql: &'static str) -> Self {
        Self {
            name,
            sql,
            breaking: false,
        }
    }

    /// Anything else. A file that has applied it cannot be opened by a build
    /// that does not list it.
    pub const fn breaking(name: &'static str, sql: &'static str) -> Self {
        Self {
            name,
            sql,
            breaking: true,
        }
    }
}

/// The table of applied names. Created by [`ensure_applied_table`], never by a
/// migration, since it is what records them.
const APPLIED_TABLE: &str = "CREATE TABLE IF NOT EXISTS armada_migrations (
    name TEXT PRIMARY KEY,
    additive INTEGER NOT NULL CHECK (additive IN (0, 1))
) STRICT";

pub(crate) fn ensure_applied_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(APPLIED_TABLE)
}

