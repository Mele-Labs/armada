//! `scripts/convert-db-once`, which carries the one database at count 114 (a
//! preview applied step-baseline's V115 before names existed) to names. Run
//! against copies built the old way; skipped where `sqlite3` is not installed.

use std::path::PathBuf;
use std::process::Command;

use rusqlite::Connection;

use crate::migrations::{Migration, LEGACY_COUNT, MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::TempDir;
use crate::Store;

const NAME: &str = "step_baseline.survives_restart";
/// What #1831's `step_baseline::V115` creates.
const V115: &str = "CREATE TABLE job_step_baselines (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    entries TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id)
) STRICT;";

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/convert-db-once")
}

fn has_sqlite3() -> bool {
    Command::new("sqlite3").arg("--version").output().is_ok()
}

/// A file as the old code left it after a preview of #1831: the 113 numbered
/// entries, V115, and the count 114.
fn numbered_114(dir: &TempDir) {
    let conn = Connection::open(dir.db()).expect("a file");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("wal");
    for migration in &MIGRATIONS[..LEGACY_COUNT] {
        conn.execute_batch(migration.sql)
            .expect("a numbered migration");
    }
    conn.execute_batch(V115).expect("V115");
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '114')
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("count");
}

fn convert(dir: &TempDir) -> (bool, String) {
    let out = Command::new("bash")
        .arg(script())
        .arg(dir.db())
        .output()
        .expect("bash");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn with_the_file() -> Vec<Migration> {
    MIGRATIONS
        .iter()
        .copied()
        .chain([Migration::additive(NAME, V115)])
        .collect()
}

#[test]
fn the_recipe_opens_with_its_file_and_applies_nothing_again() {
    if !has_sqlite3() {
        return;
    }
    let dir = TempDir::new();
    numbered_114(&dir);
    let (ok, said) = convert(&dir);
    assert!(ok, "{said}");

    // CREATE TABLE with no IF NOT EXISTS would fail if V115 ran again.
    let store = Store::open_with(&dir.db(), &with_the_file()).expect("opens with the file");
    assert!(store.unknown_migrations().is_empty());
    let applied: i64 = store
        .conn
        .query_row("SELECT count(*) FROM armada_migrations", [], |r| r.get(0))
        .expect("count");
    assert_eq!(applied as usize, LEGACY_COUNT + 1);
}

#[test]
fn the_converted_file_opens_with_every_name_known() {
    if !has_sqlite3() {
        return;
    }
    let dir = TempDir::new();
    numbered_114(&dir);
    assert!(convert(&dir).0);
    let store = Store::open(&dir.db()).expect("the step-baseline file is on main now");
    assert!(store.unknown_migrations().is_empty());
}

#[test]
fn the_recipe_refuses_what_is_not_the_one_file_and_copies_before_it_writes() {
    if !has_sqlite3() {
        return;
    }
    let dir = TempDir::new();
    numbered_114(&dir);
    assert!(convert(&dir).0);
    assert!(PathBuf::from(format!("{}.before-names", dir.db().display())).exists());
    let (ok, said) = convert(&dir);
    assert!(
        !ok && said.contains("not a numbered database at exactly version 114"),
        "{said}"
    );

    let at_113 = TempDir::new();
    numbered_114(&at_113);
    Connection::open(at_113.db())
        .expect("open")
        .execute("UPDATE armada_meta SET value = '113'", [])
        .expect("set");
    let (ok, said) = convert(&at_113);
    assert!(!ok && said.contains("exactly version 114"), "{said}");

    let no_table = TempDir::new();
    numbered_114(&no_table);
    Connection::open(no_table.db())
        .expect("open")
        .execute_batch("DROP TABLE job_step_baselines")
        .expect("drop");
    let (ok, said) = convert(&no_table);
    assert!(!ok && said.contains("job_step_baselines"), "{said}");
}

/// The rows the script records for the numbered entries are the frozen list's.
#[test]
fn the_scripts_rows_are_the_legacy_list() {
    let text = std::fs::read_to_string(script()).expect("the script");
    let rows: Vec<(String, bool)> = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("('"))
        .filter_map(|l| l.split_once("',"))
        .map(|(n, rest)| (n.to_string(), rest.starts_with('1')))
        .collect();
    let mut want: Vec<(String, bool)> = MIGRATIONS[..LEGACY_COUNT]
        .iter()
        .map(|m| (m.name.to_string(), !m.breaking))
        .collect();
    want.push((NAME.to_string(), true));
    assert_eq!(rows, want);
}

/// V115 passes the additive rule, so the script flags it additive.
#[test]
fn step_baselines_sql_is_additive() {
    assert_eq!(super::named_migrations::not_additive(V115), None);
}
