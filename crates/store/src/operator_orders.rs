//! The standing orders the person confirmed, and his short conversation with the Operator. Written
//! by `fleet::operator`, whose words the text columns hold.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One standing order, as it was confirmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptOrder {
    /// The row's number, once kept. Ignored as it is kept.
    pub n: i64,
    /// `ipc::operator_talk::OperatorOrderForm`, as JSON.
    pub form: String,
    pub said: String,
    /// When an order for today stops standing.
    pub until: Option<String>,
    pub at: String,
}

/// One message of the conversation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptTalk {
    /// The row's number, once kept. Ignored as it is kept.
    pub n: i64,
    pub from_person: bool,
    pub text: String,
    /// The call that answered it, on the Operator's own message.
    pub call: Option<String>,
    pub at: String,
    /// The board item or Needs you item it was about.
    pub about: Option<String>,
    /// On the Operator's raise of a Needs you item: `ipc::operator_talk::OperatorRaise`, as JSON.
    pub raise: Option<String>,
    /// The Job and the item a raise is of. The Job is empty for a suggestion no Job holds.
    pub raise_session: Option<String>,
    pub raise_item: Option<String>,
    /// When a raise's item left the list.
    pub settled_at: Option<String>,
    /// The Operator's briefing of the day.
    pub briefing: bool,
    /// What the briefing held: `ipc::operator_talk::OperatorBriefing`, as JSON.
    pub briefed: Option<String>,
}

const TALK_COLUMNS: &str = "n, from_person, text, call, at, about, raised, raise_session, raise_item, settled_at, briefing, briefed";

fn talk_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptTalk> {
    Ok(KeptTalk {
        n: row.get(0)?,
        from_person: row.get(1)?,
        text: row.get(2)?,
        call: row.get(3)?,
        at: row.get(4)?,
        about: row.get(5)?,
        raise: row.get(6)?,
        raise_session: row.get(7)?,
        raise_item: row.get(8)?,
        settled_at: row.get(9)?,
        briefing: row.get(10)?,
        briefed: row.get(11)?,
    })
}

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Keep one order, answering its number.
    pub fn add_operator_order(&mut self, one: &KeptOrder) -> Result<i64, WriteError> {
        self.conn
            .execute(
                "INSERT INTO operator_orders (form, said, until, at) VALUES (?1, ?2, ?3, ?4)",
                params![one.form, one.said, one.until, one.at],
            )
            .map_err(failed("keeping a standing order"))?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The orders not removed, oldest first. One whose `until` has passed is still here: whether it
    /// stands is read against the clock.
    pub fn operator_orders(&self) -> Result<Vec<KeptOrder>, WriteError> {
        let doing = "reading the standing orders";
        let mut statement = self
            .conn
            .prepare("SELECT n, form, said, until, at FROM operator_orders WHERE removed_at IS NULL ORDER BY n")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([], |row| {
                Ok(KeptOrder { n: row.get(0)?, form: row.get(1)?, said: row.get(2)?, until: row.get(3)?, at: row.get(4)? })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Order `n` stops standing. `false` where it was not standing.
    pub fn remove_operator_order(&mut self, n: i64, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE operator_orders SET removed_at = ?2 WHERE n = ?1 AND removed_at IS NULL", params![n, at])
            .map(|changed| changed > 0)
            .map_err(failed("removing a standing order"))
    }

    /// Keep one message of the conversation.
    pub fn add_operator_talk(&mut self, one: &KeptTalk) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO operator_talk (from_person, text, call, at, about, raised, raise_session, raise_item, settled_at, briefing, briefed) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![one.from_person, one.text, one.call, one.at, one.about, one.raise, one.raise_session, one.raise_item, one.settled_at, one.briefing, one.briefed],
            )
            .map(|_| ())
            .map_err(failed("keeping a message to the Operator"))
    }

    /// The last `limit` messages of the conversation, raises and briefings among them, oldest first.
    pub fn operator_talk(&self, limit: u32) -> Result<Vec<KeptTalk>, WriteError> {
        self.talk_read(&format!("SELECT {TALK_COLUMNS} FROM (SELECT * FROM operator_talk ORDER BY n DESC LIMIT ?1) ORDER BY n"), params![limit])
    }

    /// The last `limit` messages that are not raises, oldest first: what he and the Operator said.
    pub fn operator_talk_said(&self, limit: u32) -> Result<Vec<KeptTalk>, WriteError> {
        self.talk_read(&format!("SELECT {TALK_COLUMNS} FROM (SELECT * FROM operator_talk WHERE raised IS NULL ORDER BY n DESC LIMIT ?1) ORDER BY n"), params![limit])
    }

    /// Every raise whose item still waits, oldest first.
    pub fn operator_raises_open(&self) -> Result<Vec<KeptTalk>, WriteError> {
        self.talk_read(&format!("SELECT {TALK_COLUMNS} FROM operator_talk WHERE raised IS NOT NULL AND settled_at IS NULL ORDER BY n"), params![])
    }

    /// Raise `n`'s item left the list at `at`. `false` where it was settled already.
    pub fn settle_operator_raise(&mut self, n: i64, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE operator_talk SET settled_at = ?2 WHERE n = ?1 AND raised IS NOT NULL AND settled_at IS NULL", params![n, at])
            .map(|changed| changed > 0)
            .map_err(failed("settling a raise of the Operator's"))
    }

    /// When the Operator last briefed the person, where it ever has.
    pub fn operator_briefed_at(&self) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT max(at) FROM operator_talk WHERE briefing = 1", [], |row| row.get(0))
            .map_err(failed("reading when the Operator last briefed"))
    }

    fn talk_read(&self, sql: &str, args: impl rusqlite::Params) -> Result<Vec<KeptTalk>, WriteError> {
        let doing = "reading the conversation with the Operator";
        let mut statement = self.conn.prepare(sql).map_err(failed(doing))?;
        let rows = statement.query_map(args, talk_row).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
