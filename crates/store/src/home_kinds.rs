//! What this Mac syncs with each profile's shared home, one switch a kind and one an item switched
//! on by itself, kept on this machine alone.
//!
//! **A kind with no row is off**, so a Mac that never chose syncs nothing. The kind is the wire's
//! word and is not looked up here: Fleet names it.

use std::collections::BTreeMap;

use crate::error::{fault, DatabaseFault, WriteError};
use crate::open::Store;

/// One item switched on by itself, as the store keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptHomeItem {
    /// The wire's word for its kind.
    pub kind: String,
    pub name: String,
    /// A memory note's repository, and only there.
    pub repository: Option<String>,
}

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

    /// Each item switched on by itself for `login`'s home: its kind's word, its name and a memory
    /// note's repository.
    pub fn home_items(&self, login: &str) -> Result<Vec<KeptHomeItem>, DatabaseFault> {
        let mut statement = self
            .conn
            .prepare("SELECT kind, name, repository FROM home_items WHERE login = ?1 ORDER BY kind, name, repository")
            .map_err(fault("reading the items that sync"))?;
        let rows = statement
            .query_map([login], |row| {
                let repository: String = row.get(2)?;
                Ok(KeptHomeItem { kind: row.get(0)?, name: row.get(1)?, repository: (!repository.is_empty()).then_some(repository) })
            })
            .map_err(fault("reading the items that sync"))?;
        rows.collect::<Result<_, _>>().map_err(fault("reading the items that sync"))
    }

    /// Switch one item on or off by itself for `login`'s home. Off takes its row away.
    pub fn switch_home_item(&mut self, login: &str, item: &KeptHomeItem, on: bool) -> Result<(), WriteError> {
        let repository = item.repository.as_deref().unwrap_or("");
        let done = match on {
            true => self.conn.execute(
                "INSERT INTO home_items (login, kind, name, repository) VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
                (login, &item.kind, &item.name, repository),
            ),
            false => self.conn.execute(
                "DELETE FROM home_items WHERE login = ?1 AND kind = ?2 AND name = ?3 AND repository = ?4",
                (login, &item.kind, &item.name, repository),
            ),
        };
        done.map(|_| ()).map_err(fault("switching an item that syncs")).map_err(WriteError::Database)
    }
}
