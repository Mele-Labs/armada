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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptTalk {
    pub from_person: bool,
    pub text: String,
    /// The call that answered it, on the Operator's own message.
    pub call: Option<String>,
    pub at: String,
    /// The board item or Needs you item it was about.
    pub about: Option<String>,
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
                "INSERT INTO operator_talk (from_person, text, call, at, about) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![one.from_person, one.text, one.call, one.at, one.about],
            )
            .map(|_| ())
            .map_err(failed("keeping a message to the Operator"))
    }

    /// The last `limit` messages of the conversation, oldest first.
    pub fn operator_talk(&self, limit: u32) -> Result<Vec<KeptTalk>, WriteError> {
        let doing = "reading the conversation with the Operator";
        let mut statement = self
            .conn
            .prepare("SELECT from_person, text, call, at, about FROM (SELECT * FROM operator_talk ORDER BY n DESC LIMIT ?1) ORDER BY n")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([limit], |row| Ok(KeptTalk { from_person: row.get(0)?, text: row.get(1)?, call: row.get(2)?, at: row.get(3)?, about: row.get(4)? }))
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
