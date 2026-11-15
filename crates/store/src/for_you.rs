//! What was said on top of a Job's For you: the order, a line, a withdrawal, and the rows an agent added.
//! `docs/practices/protocol/for-you.md`. Everything else on the tab is read off what Fleet already keeps.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One row id's marks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptForYouMark {
    pub item: String,
    /// Its place in the order set, from 0. `None` where not placed.
    pub place: Option<u32>,
    pub line: Option<String>,
    pub withdrawn: bool,
}

/// One row an agent added, `added:<n>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptForYouAdded {
    pub n: u32,
    pub title: String,
    /// `walk`, `document`, `plan`, `draft`, `sketch`, `question`, `run` or `approve`.
    pub kind: String,
    /// `artifact`, `walk`, `waiting` or `plan`.
    pub opens: String,
    /// The artifact's id, the address or the item's id. `None` for `plan`.
    pub target: Option<String>,
    pub since: String,
}

/// A row to add.
#[derive(Clone, Copy, Debug)]
pub struct NewForYouAdded<'a> {
    pub title: &'a str,
    pub kind: &'a str,
    pub opens: &'a str,
    pub target: Option<&'a str>,
    pub since: &'a str,
}

impl Store {
    /// Every mark kept for `session`'s For you.
    pub fn for_you_marks(&self, session: &str) -> Result<Vec<KeptForYouMark>, WriteError> {
        let doing = "reading the marks on a Job's For you";
        let mut statement = self
            .conn
            .prepare("SELECT item, place, line, withdrawn FROM for_you_marks WHERE session_id = ?1")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session], |row| {
                Ok(KeptForYouMark { item: row.get(0)?, place: row.get(1)?, line: row.get(2)?, withdrawn: row.get::<_, i64>(3)? != 0 })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Every row added to `session`'s For you, oldest first.
    pub fn for_you_added(&self, session: &str) -> Result<Vec<KeptForYouAdded>, WriteError> {
        let doing = "reading the rows added to a Job's For you";
        let mut statement = self
            .conn
            .prepare("SELECT n, title, kind, opens, target, since FROM for_you_added WHERE session_id = ?1 ORDER BY n")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session], |row| {
                Ok(KeptForYouAdded {
                    n: row.get(0)?,
                    title: row.get(1)?,
                    kind: row.get(2)?,
                    opens: row.get(3)?,
                    target: row.get(4)?,
                    since: row.get(5)?,
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Keep `item` withdrawn for good.
    pub fn withdraw_for_you(&mut self, session: &str, item: &str) -> Result<(), WriteError> {
        let doing = "withdrawing a row of a Job's For you";
        self.conn
            .execute(
                "INSERT INTO for_you_marks (session_id, item, withdrawn) VALUES (?1, ?2, 1)
                 ON CONFLICT (session_id, item) DO UPDATE SET withdrawn = 1",
                params![session, item],
            )
            .map(drop)
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Place `items` in this order, and no other row.
    pub fn order_for_you(&mut self, session: &str, items: &[String]) -> Result<(), WriteError> {
        let doing = "ordering a Job's For you";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            transaction.execute("UPDATE for_you_marks SET place = NULL WHERE session_id = ?1", [session])?;
            for (place, item) in items.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO for_you_marks (session_id, item, place) VALUES (?1, ?2, ?3)
                     ON CONFLICT (session_id, item) DO UPDATE SET place = excluded.place",
                    params![session, item, place as i64],
                )?;
            }
            transaction.commit()
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Put `line` under `item`, or clear it with `None`.
    pub fn line_for_you(&mut self, session: &str, item: &str, line: Option<&str>) -> Result<(), WriteError> {
        let doing = "putting a line on a row of a Job's For you";
        self.conn
            .execute(
                "INSERT INTO for_you_marks (session_id, item, line) VALUES (?1, ?2, ?3)
                 ON CONFLICT (session_id, item) DO UPDATE SET line = excluded.line",
                params![session, item, line],
            )
            .map(drop)
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Keep a row added to `session`'s For you. Answers its `n`.
    pub fn add_for_you(&mut self, session: &str, row: NewForYouAdded<'_>) -> Result<u32, WriteError> {
        let doing = "adding a row to a Job's For you";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            let last: Option<u32> = transaction
                .query_row("SELECT MAX(n) FROM for_you_added WHERE session_id = ?1", [session], |one| one.get(0))
                .optional()?
                .flatten();
            let n = last.map_or(1, |last| last + 1);
            transaction.execute(
                "INSERT INTO for_you_added (session_id, n, title, kind, opens, target, since) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![session, n, row.title, row.kind, row.opens, row.target, row.since],
            )?;
            transaction.commit()?;
            Ok(n)
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }
}
