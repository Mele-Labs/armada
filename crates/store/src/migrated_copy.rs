//! A fresh store for a test, copied from a file migrated once.
//!
//! Applying every migration to an empty file is most of what a store test
//! costs, and the test is about one table, not the migrations. `nextest` runs
//! each test in a process of its own, so the migrated file is kept in the temp
//! dir, named by the migration list, and made once per list rather than once
//! per process. The copy is still a real file opened through
//! [`Store::open`]: WAL, the checks and the reopen are all the same.
//!
//! Not behind a feature: `store` built with a feature `fleet`'s tests turn on
//! would be a second copy of it and of everything above it.
//! `docs/practices/rust.md`, *One build of fleet*.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::error::{fault, OpenError};
use crate::migrations::MIGRATIONS;
use crate::Store;

impl Store {
    /// [`open`](Self::open) for a test: where nothing is at `path` yet, a copy
    /// of a store every migration has already run on is put there first. A
    /// path that holds a file is opened as it is. Tests of migration itself
    /// call `open`, which migrates for real.
    #[doc(hidden)]
    pub fn open_migrated_copy(path: &Path) -> Result<Store, OpenError> {
        if !path.exists() {
            std::fs::copy(template()?, path).expect("the migrated store copies");
        }
        Store::open(path)
    }
}

/// The migrated file for this build's list, made the first time it is asked for.
fn template() -> Result<&'static Path, OpenError> {
    static TEMPLATE: OnceLock<PathBuf> = OnceLock::new();
    if let Some(made) = TEMPLATE.get() {
        return Ok(made);
    }
    let made = make_template()?;
    Ok(TEMPLATE.get_or_init(|| made))
}

fn make_template() -> Result<PathBuf, OpenError> {
    let dir = std::env::temp_dir().join("armada-store-migrated");
    let name = format!("{:016x}", fingerprint());
    let template = dir.join(format!("{name}.sqlite"));
    if template.exists() {
        return Ok(template);
    }
    std::fs::create_dir_all(&dir).expect("a directory for the migrated store");

    // Made under a name of this process's own and renamed into place, so a
    // process that finds the template finds it whole. Two processes making it
    // at once make the same file, and the second rename replaces the first.
    let making = dir.join(format!("{name}-{}.making", std::process::id()));
    let _ = std::fs::remove_file(&making);
    let store = Store::open(&making)?;
    // Out of WAL, which checkpoints and removes the `-wal` file, so the one
    // file copied is all of it.
    let mode: String = store
        .conn
        .query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))
        .map_err(fault("leaving WAL for the copy"))
        .map_err(OpenError::Database)?;
    assert!(mode.eq_ignore_ascii_case("delete"), "the migrated store left WAL, and said {mode}");
    drop(store);
    std::fs::rename(&making, &template).expect("the migrated store moves into place");
    Ok(template)
}

/// Every migration's name, SQL and kind, in order: a list that differs by any
/// of them is a different file.
fn fingerprint() -> u64 {
    let mut hasher = DefaultHasher::new();
    for migration in MIGRATIONS {
        migration.name.hash(&mut hasher);
        migration.sql.hash(&mut hasher);
        migration.breaking.hash(&mut hasher);
    }
    hasher.finish()
}
