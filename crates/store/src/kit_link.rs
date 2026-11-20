//! Kit over the Mac link, kept on this Mac alone: which kinds and items cross, the last-agreed marker for each item
//! with each linked Mac, when this Mac first read each version of an item, and what each linked Mac last offered.
//! `docs/concepts/kit.md`, *Over the Mac link*.
//!
//! **A kind with no row is off**, so a Mac that never chose crosses nothing. Kinds, hashes and offers are Fleet's
//! words and are not looked up here.

use std::collections::BTreeMap;

use rusqlite::params;

use crate::error::{fault, DatabaseFault, WriteError};
use crate::home_kinds::KeptHomeItem;
use crate::open::Store;

/// What Mac `mac` last offered, as the wire carried it, and when it crossed, RFC3339.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptKitOffer {
    pub mac: String,
    pub at: String,
    pub offer: String,
}

fn item_of(kind: String, name: String, repository: String) -> KeptHomeItem {
    KeptHomeItem { kind, name, repository: (!repository.is_empty()).then_some(repository) }
}

impl Store {
    /// Each kind switched on this Mac, by its word.
    pub fn kit_link_kinds(&self) -> Result<BTreeMap<String, bool>, DatabaseFault> {
        let doing = "reading which kinds cross to his other Macs";
        let mut statement = self.conn.prepare("SELECT kind, crosses FROM kit_link_kinds").map_err(fault(doing))?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0)))
            .map_err(fault(doing))?;
        rows.collect::<Result<_, _>>().map_err(fault(doing))
    }

    pub fn switch_kit_link_kind(&mut self, kind: &str, crosses: bool) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO kit_link_kinds (kind, crosses) VALUES (?1, ?2) ON CONFLICT (kind) DO UPDATE SET crosses = excluded.crosses",
                params![kind, i64::from(crosses)],
            )
            .map(|_| ())
            .map_err(fault("switching a kind that crosses to his other Macs"))
            .map_err(WriteError::Database)
    }

    /// Each item switched on by itself.
    pub fn kit_link_items(&self) -> Result<Vec<KeptHomeItem>, DatabaseFault> {
        let doing = "reading the items that cross to his other Macs";
        let mut statement = self
            .conn
            .prepare("SELECT kind, name, repository FROM kit_link_items ORDER BY kind, name, repository")
            .map_err(fault(doing))?;
        let rows = statement.query_map([], |row| Ok(item_of(row.get(0)?, row.get(1)?, row.get(2)?))).map_err(fault(doing))?;
        rows.collect::<Result<_, _>>().map_err(fault(doing))
    }

    /// Switch one item on or off by itself. Off takes its row away.
    pub fn switch_kit_link_item(&mut self, item: &KeptHomeItem, on: bool) -> Result<(), WriteError> {
        let repository = item.repository.as_deref().unwrap_or("");
        let done = match on {
            true => self.conn.execute(
                "INSERT INTO kit_link_items (kind, name, repository) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                params![item.kind, item.name, repository],
            ),
            false => self.conn.execute(
                "DELETE FROM kit_link_items WHERE kind = ?1 AND name = ?2 AND repository = ?3",
                params![item.kind, item.name, repository],
            ),
        };
        done.map(|_| ()).map_err(fault("switching an item that crosses to his other Macs")).map_err(WriteError::Database)
    }

    /// The last-agreed marker of every item this Mac and Mac `mac` agreed on, by item.
    pub fn kit_link_agreed(&self, mac: &str) -> Result<BTreeMap<KeptHomeItem, String>, DatabaseFault> {
        let doing = "reading what this Mac last agreed with another";
        let mut statement =
            self.conn.prepare("SELECT kind, name, repository, hash FROM kit_link_agreed WHERE mac = ?1").map_err(fault(doing))?;
        let rows = statement
            .query_map([mac], |row| Ok((item_of(row.get(0)?, row.get(1)?, row.get(2)?), row.get::<_, String>(3)?)))
            .map_err(fault(doing))?;
        rows.collect::<Result<_, _>>().map_err(fault(doing))
    }

    /// Record that this Mac and Mac `mac` agree on `item` at `hash`, over what was agreed before.
    pub fn agree_kit_link(&mut self, mac: &str, item: &KeptHomeItem, hash: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO kit_link_agreed (mac, kind, name, repository, hash) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (mac, kind, name, repository) DO UPDATE SET hash = excluded.hash",
                params![mac, item.kind, item.name, item.repository.as_deref().unwrap_or(""), hash],
            )
            .map(|_| ())
            .map_err(fault("keeping what this Mac agreed with another"))
            .map_err(WriteError::Database)
    }

    /// Each item's hash on this Mac and when this Mac first read that version.
    pub fn kit_link_seen(&self) -> Result<BTreeMap<KeptHomeItem, (String, String)>, DatabaseFault> {
        let doing = "reading when each Kit item last changed here";
        let mut statement = self.conn.prepare("SELECT kind, name, repository, hash, at FROM kit_link_seen").map_err(fault(doing))?;
        let rows = statement
            .query_map([], |row| Ok((item_of(row.get(0)?, row.get(1)?, row.get(2)?), (row.get::<_, String>(3)?, row.get::<_, String>(4)?))))
            .map_err(fault(doing))?;
        rows.collect::<Result<_, _>>().map_err(fault(doing))
    }

    /// `item` read at `hash` now, `at`. **A version already seen keeps its first time.**
    pub fn see_kit_link(&mut self, item: &KeptHomeItem, hash: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO kit_link_seen (kind, name, repository, hash, at) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (kind, name, repository) DO UPDATE SET hash = excluded.hash, at = excluded.at
                 WHERE kit_link_seen.hash != excluded.hash",
                params![item.kind, item.name, item.repository.as_deref().unwrap_or(""), hash, at],
            )
            .map(|_| ())
            .map_err(fault("keeping when a Kit item changed here"))
            .map_err(WriteError::Database)
    }

    /// What Mac `mac` offered, kept over what it offered before.
    pub fn hold_kit_offer(&mut self, held: &KeptKitOffer) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT OR REPLACE INTO kit_link_held (mac, at, offer) VALUES (?1, ?2, ?3)", params![held.mac, held.at, held.offer])
            .map(|_| ())
            .map_err(fault("keeping what another Mac's Kit offers"))
            .map_err(WriteError::Database)
    }

    /// A clash settled here, queued for Mac `mac` behind every act already queued for it, in the queue project acts
    /// use. Its `seq`.
    pub fn queue_kit_settle(&mut self, mac: &str, at: i64, act: &str) -> Result<i64, WriteError> {
        self.conn
            .execute("INSERT INTO other_mac_project_acts (mac, kind, id, at, act) VALUES (?1, 'kit', '', ?2, ?3)", params![mac, at, act])
            .map_err(fault("queueing a settled Kit clash for another Mac"))
            .map_err(WriteError::Database)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// What each linked Mac last offered, by Mac.
    pub fn kit_offers_held(&self) -> Result<Vec<KeptKitOffer>, DatabaseFault> {
        let doing = "reading what other Macs' Kits offer";
        let mut statement = self.conn.prepare("SELECT mac, at, offer FROM kit_link_held ORDER BY mac").map_err(fault(doing))?;
        let rows = statement
            .query_map([], |row| Ok(KeptKitOffer { mac: row.get(0)?, at: row.get(1)?, offer: row.get(2)? }))
            .map_err(fault(doing))?;
        rows.collect::<Result<_, _>>().map_err(fault(doing))
    }
}
