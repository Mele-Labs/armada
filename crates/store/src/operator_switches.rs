//! Each move of the Operator's master switch, for the Log's timeline. Written by `fleet::settings`.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One move of the switch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSwitch {
    pub on: bool,
    /// RFC 3339.
    pub at: String,
}

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Keep one move of the switch.
    pub fn add_operator_switch(&mut self, one: &KeptSwitch) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT INTO operator_switches (turned_on, at) VALUES (?1, ?2)", params![one.on, one.at])
            .map(|_| ())
            .map_err(failed("keeping a move of the Operator's switch"))
    }

    /// The latest moves of the switch, newest first.
    pub fn operator_switches(&self, limit: u32) -> Result<Vec<KeptSwitch>, WriteError> {
        let doing = "reading the moves of the Operator's switch";
        let mut statement = self.conn.prepare("SELECT turned_on, at FROM operator_switches ORDER BY n DESC LIMIT ?1").map_err(failed(doing))?;
        let rows = statement.query_map([limit], |row| Ok(KeptSwitch { on: row.get(0)?, at: row.get(1)? })).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
