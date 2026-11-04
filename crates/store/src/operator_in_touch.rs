//! The pairs of Jobs the Operator keeps in touch, and the paths each pair shares. Written by
//! `fleet::operator` when it acts on a keep in touch, and read when either Job moves.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Keep `one` and `other` in touch over `shared`. A pair kept again takes the paths named last.
    pub fn keep_operator_in_touch(&mut self, one: &str, other: &str, shared: &[String], since: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO operator_in_touch (one, other, shared, since) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (one, other) DO UPDATE SET shared = excluded.shared",
                params![one, other, shared.join("\n"), since],
            )
            .map(|_| ())
            .map_err(failed("keeping two Jobs in touch"))
    }

    /// Every Job `session` is kept in touch with, either way round, and the paths they share.
    pub fn operator_in_touch(&self, session: &str) -> Result<Vec<(String, Vec<String>)>, WriteError> {
        let doing = "reading the Jobs one is kept in touch with";
        let mut statement = self
            .conn
            .prepare(
                "SELECT other, shared FROM operator_in_touch WHERE one = ?1
                 UNION ALL SELECT one, shared FROM operator_in_touch WHERE other = ?1",
            )
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([session], |row| {
                let shared: String = row.get(1)?;
                Ok((row.get::<_, String>(0)?, shared.lines().filter(|one| !one.is_empty()).map(str::to_string).collect()))
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
