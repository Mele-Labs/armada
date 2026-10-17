//! An empty store and a damaged one are different events, and this is where
//! that is proved.
//!
//! Every case below asserts the same shape: **the open is refused, by name.**
//! None of them asserts a fallback, because there is none. Starting empty over
//! a database that exists is how a person loses work and is told nothing.
//!
use rusqlite::Connection;

use crate::migrations::LEGACY_COUNT;
use crate::migrations::SCHEMA_VERSION_KEY;
use crate::tests::{open, TempDir};
use crate::{OpenError, Store, KNOWN_SCHEMA_VERSION};

// ------------------------------------------------------------ the empty case

#[test]
fn nothing_at_the_path_is_a_first_run() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert!(store
        .remembered_repositories()
        .expect("an empty store")
        .is_empty());
    assert_eq!(version(&dir), KNOWN_SCHEMA_VERSION.to_string());
}

#[test]
fn opening_twice_migrates_once() {
    let dir = TempDir::new();
    drop(open(&dir));
    drop(open(&dir));
    assert_eq!(version(&dir), KNOWN_SCHEMA_VERSION.to_string());
}

#[test]
fn the_store_is_in_wal_mode() {
    let dir = TempDir::new();
    let store = open(&dir);
    let mode: String = store
        .conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("a journal mode");
    assert_eq!(mode.to_lowercase(), "wal");
}

// ------------------------------------------------------- the file-level cases

#[test]
fn a_database_that_is_not_an_armada_store_is_refused() {
    let dir = TempDir::new();
    let conn = Connection::open(dir.db()).expect("some other database");
    conn.execute_batch("CREATE TABLE somebody_elses (id INTEGER PRIMARY KEY);")
        .expect("written");
    drop(conn);

    match Store::open(&dir.db()) {
        Err(OpenError::NotAnArmadaStore { tables, .. }) => assert_eq!(tables, 1),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn a_numbered_file_past_the_old_list_is_refused() {
    let dir = TempDir::new();
    drop(open(&dir));
    as_a_numbered_file(&dir);
    set_version(&dir, &(LEGACY_COUNT + 7).to_string());

    match Store::open(&dir.db()) {
        Err(OpenError::SchemaVersionFromTheFuture { found, known, .. }) => {
            assert_eq!(found as usize, LEGACY_COUNT + 7);
            assert_eq!(known as usize, LEGACY_COUNT);
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn a_version_that_is_not_a_number_is_refused() {
    let dir = TempDir::new();
    drop(open(&dir));
    as_a_numbered_file(&dir);
    set_version(&dir, "whenever");

    match Store::open(&dir.db()) {
        Err(OpenError::SchemaVersionUnreadable { found, .. }) => {
            assert_eq!(found.as_deref(), Some("whenever"))
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn a_marker_table_with_no_version_is_refused_rather_than_assumed_fresh() {
    let dir = TempDir::new();
    drop(open(&dir));
    as_a_numbered_file(&dir);
    let conn = Connection::open(dir.db()).expect("open");
    conn.execute("DELETE FROM armada_meta", [])
        .expect("emptied");
    drop(conn);

    match Store::open(&dir.db()) {
        Err(OpenError::SchemaVersionUnreadable { found: None, .. }) => {}
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// Scribbling over a page. Which check catches it depends on where the damage
/// lands, so the assertion is the one that matters: **it does not open, and it
/// does not open empty.**
#[test]
fn a_damaged_file_does_not_open_empty() {
    let dir = TempDir::new();
    let store = open(&dir);
    store
        .conn
        .pragma_update(None, "wal_checkpoint", "TRUNCATE")
        .expect("flushed to the main file");
    drop(store);

    let mut bytes = std::fs::read(dir.db()).expect("the file");
    let page = 4096;
    assert!(
        bytes.len() > page * 2,
        "there is more than one page to damage"
    );
    for byte in bytes.iter_mut().skip(page).take(page) {
        *byte = 0x5A;
    }
    std::fs::write(dir.db(), &bytes).expect("scribbled on");

    // `integrity_check` is what catches this one in practice; a scribble
    // elsewhere in the file would come back as a SQLite fault instead. Either
    // way it is refused, which is the property, so the assertion is that and
    // not the variant.
    assert!(
        Store::open(&dir.db()).is_err(),
        "a damaged file is refused, not opened as though it were new"
    );
}

// ------------------------------------------------------------------- fixtures

fn version(dir: &TempDir) -> String {
    let conn = Connection::open(dir.db()).expect("open");
    conn.query_row("SELECT count(*) FROM armada_migrations", [], |row| {
        row.get::<_, i64>(0)
    })
    .expect("the applied names")
    .to_string()
}

/// The file as it was before names: no table of applied names, only the count.
fn as_a_numbered_file(dir: &TempDir) {
    let conn = Connection::open(dir.db()).expect("open");
    conn.execute_batch("DROP TABLE armada_migrations")
        .expect("dropped");
}

fn set_version(dir: &TempDir, value: &str) {
    let conn = Connection::open(dir.db()).expect("open");
    conn.execute(
        "UPDATE armada_meta SET value = ?2 WHERE key = ?1",
        (SCHEMA_VERSION_KEY, value),
    )
    .expect("set");
}
