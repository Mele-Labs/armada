//! What a Job was last asked and is doing now: the person's latest message and the plan task in
//! progress. An attachment's version reads both when it is written, so the graph of how a Job evolved
//! can hang each artifact off the plan or off the message it followed.

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The focus as kept. Either part is absent until it has been seen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptFocus {
    /// The thread row id of the person's latest message.
    pub message_id: Option<String>,
    /// The plan task in progress, the one that began most recently where several are.
    pub task_id: Option<String>,
}

/// The task to keep for `session_id`, from the plan as it now stands. The group and position order
/// break a tie between two that began in the same instant; one with no time is older than any with one.
pub(crate) fn refocus(conn: &Connection, session_id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO session_focus (session_id, task_id)
         VALUES (?1, (
             SELECT t.task_id FROM session_plan_tasks t
             JOIN session_plan_groups g ON g.session_id = t.session_id AND g.group_id = t.group_id
             WHERE t.session_id = ?1 AND t.listed = 'in_progress'
             ORDER BY t.started_at DESC, g.position DESC, t.position DESC
             LIMIT 1))
         ON CONFLICT (session_id) DO UPDATE SET task_id = excluded.task_id",
        [session_id],
    )
    .map(|_| ())
}

impl Store {
    /// The focus, absent until the person has sent a message or a task has been in progress.
    pub fn session_focus(&self, session_id: &str) -> Result<Option<KeptFocus>, WriteError> {
        let doing = "reading a job's focus";
        self.conn
            .query_row(
                "SELECT message_id, task_id FROM session_focus WHERE session_id = ?1",
                [session_id],
                |row| Ok(KeptFocus { message_id: row.get(0)?, task_id: row.get(1)? }),
            )
            .optional()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// The person's latest message is now `message_id`.
    pub fn keep_session_message(&mut self, session_id: &str, message_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_focus (session_id, message_id) VALUES (?1, ?2)
                 ON CONFLICT (session_id) DO UPDATE SET message_id = excluded.message_id",
                params![session_id, message_id],
            )
            .map(|_| ())
            .map_err(fault("keeping a job's focus"))
            .map_err(WriteError::Database)
    }
}
