//! What a migration is, and the table that records which ones a file has had.
//!
//! The list is `migration_list.rs`. A migration has a stable name, not a
//! position, so two branches that each add one never take the same number, and
//! a file written by a build with one more name than this one is read by it
//! when that name was additive. `docs/practices/store-migrations.md` has the
//! rules and why.

use rusqlite::Connection;

pub(crate) use crate::migration_list::MIGRATIONS;

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

/// Every table whose rows belong to one Job, asked of the file rather than
/// listed here.
///
/// [`forget_job`](crate::Store::forget_job) deletes from each. It asks because
/// a list is a thing somebody has to remember to extend, and three times in a
/// row nobody did: `job_step_judgments`, `job_step_gaming_flags` and
/// `job_step_evidence` arrived in `V8`, `V12` and `V10` and none of them
/// reached the delete. Nothing failed while no shipped workflow declared a
/// `judge_check` and the tables stayed empty; the day nine of them went live,
/// every Job that reached a gate became one `armada clean` could not forget,
/// because the `jobs` row it deletes first is the parent those rows point at.
/// A catalog cannot fall behind the schema it is the schema of.
///
/// **The file's catalog, not the constants above it.** `V13` builds four
/// tables under `_wide` names and renames them over the originals, so the
/// `CREATE TABLE` text in that module names four tables no migrated file has
/// and misses the four every one of them does.
///
/// `job_events` is in this set and belongs there — it is append-only by
/// trigger only while its Job row still exists, which by then it does not. See
/// `V4`, and the ordering it forces on `forget_job`.
pub(crate) fn tables_pointing_at_a_job(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    conn.prepare(
        r#"SELECT m.name
           FROM sqlite_master AS m
           JOIN pragma_foreign_key_list(m.name) AS fk
           WHERE m.type = 'table' AND fk."table" = 'jobs'
           GROUP BY m.name
           ORDER BY m.name"#,
    )?
    .query_map([], |row| row.get(0))?
    .collect()
}
