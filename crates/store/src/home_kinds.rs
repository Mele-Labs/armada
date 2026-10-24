//! What this Mac syncs with each profile's shared home, one switch a kind, kept on this machine
//! alone.
//!
//! **A kind with no row is off**, so a Mac that never chose syncs nothing. The kind is the wire's
//! word and is not looked up here: Fleet names it.

use std::collections::BTreeMap;

use crate::error::{fault, DatabaseFault, WriteError};
use crate::open::Store;

impl Store {
    /// Each kind somebody switched for `login`'s home, by its word.
    pub fn home_kinds(&self, login: &str) -> Result<BTreeMap<String, bool>, DatabaseFault> {
        let mut statement = self
            .conn
            .prepare("SELECT kind, synced FROM home_kinds WHERE login = ?1")
            .map_err(fault("reading what syncs"))?;
        let rows = statement
            .query_map([login], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0)))
            .map_err(fault("reading what syncs"))?;
        rows.collect::<Result<_, _>>().map_err(fault("reading what syncs"))
    }

    /// Switch one kind on or off for `login`'s home.
    pub fn switch_home_kind(&mut self, login: &str, kind: &str, synced: bool) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO home_kinds (login, kind, synced) VALUES (?1, ?2, ?3)
                 ON CONFLICT (login, kind) DO UPDATE SET synced = excluded.synced",
                (login, kind, i64::from(synced)),
            )
            .map(|_| ())
            .map_err(fault("switching what syncs"))
            .map_err(WriteError::Database)
    }
}
