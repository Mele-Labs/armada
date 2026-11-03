//! A Job's terminals: which numbers it has had, what the person called each, and the shell its
//! agent last typed into or read in each. A shell is never written down and a restart ends it, so
//! the seen run is what says an agent's next shell is a replacement. **A row stays after its shell
//! ends**, which is what keeps `t1` from being given twice.

use core_model::Timestamp;
use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One terminal a Job has had.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptJobTerminal {
    /// The number in `t<number>`.
    pub number: u32,
    /// What the person renamed it to. Blank until they have.
    pub name: String,
    /// The run id of the shell its agent last typed into or read, absent where it never has.
    pub seen_run: Option<String>,
    pub seen_at: Option<String>,
}

impl Store {
    /// `session_id`'s terminals, lowest number first.
    pub fn job_terminals(&self, session_id: &str) -> Result<Vec<KeptJobTerminal>, WriteError> {
        let doing = "reading a job's terminals";
        let mut statement = self
            .conn
            .prepare("SELECT number, name, seen_run_id, seen_at FROM job_terminals WHERE session_id = ?1 ORDER BY number")
            .map_err(fault(doing))?;
        let rows = statement
            .query_map([session_id], |row| {
                Ok(KeptJobTerminal { number: row.get(0)?, name: row.get(1)?, seen_run: row.get(2)?, seen_at: row.get(3)? })
            })
            .map_err(fault(doing))?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// The next terminal of `session_id`: one more than its highest number, `1` where it has had none.
    pub fn mint_job_terminal(&mut self, session_id: &str) -> Result<u32, WriteError> {
        self.conn
            .query_row(
                "INSERT INTO job_terminals (session_id, number)
                 SELECT ?1, COALESCE(MAX(number), 0) + 1 FROM job_terminals WHERE session_id = ?1
                 RETURNING number",
                [session_id],
                |row| row.get(0),
            )
            .map_err(fault("minting a job's terminal"))
            .map_err(WriteError::Database)
    }

    /// Name `session_id`'s terminal `number`. Blank restores the default, which is Fleet's to spell.
    pub fn keep_job_terminal_name(&mut self, session_id: &str, number: u32, name: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE job_terminals SET name = ?3 WHERE session_id = ?1 AND number = ?2", params![session_id, number, name])
            .map(|_| ())
            .map_err(fault("naming a job's terminal"))
            .map_err(WriteError::Database)
    }

    /// `session_id`'s agent has now typed into or read the shell `run_id` of terminal `number`.
    pub fn keep_job_terminal_seen(&mut self, session_id: &str, number: u32, run_id: &str, at: &Timestamp) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_terminals SET seen_run_id = ?3, seen_at = ?4 WHERE session_id = ?1 AND number = ?2",
                params![session_id, number, run_id, at.as_str()],
            )
            .map(|_| ())
            .map_err(fault("keeping the shell a job's agent last saw"))
            .map_err(WriteError::Database)
    }
}
