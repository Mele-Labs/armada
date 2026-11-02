//! What a hosted Job's agent said it came up with, kept for the debug report.
//! `docs/concepts/session/a-job-that-came-up-without-its-tools.md`.
//!
//! **Names only, and only the harness's and the mod's**: the caller drops another server's tools and
//! another plugin's skills before they reach here, and a test holds that at the line that reads them.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One Job's opening line, as far as it is kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptInit {
    pub harness_version: String,
    pub mod_loaded: bool,
    pub door: String,
    pub task_tools: bool,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
}

fn lines(names: &[String]) -> String {
    names.join("\n")
}

fn names(text: String) -> Vec<String> {
    text.lines().filter(|name| !name.is_empty()).map(str::to_string).collect()
}

impl Store {
    /// Keep what `session_id`'s agent came up with, replacing the last line. `at` is RFC3339.
    pub fn keep_init(&mut self, session_id: &str, init: &KeptInit, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_init (session_id, harness_version, mod_loaded, door, task_tools, tools, skills, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(session_id) DO UPDATE SET harness_version = excluded.harness_version,
                   mod_loaded = excluded.mod_loaded, door = excluded.door, task_tools = excluded.task_tools,
                   tools = excluded.tools, skills = excluded.skills, at = excluded.at",
                params![session_id, init.harness_version, init.mod_loaded, init.door, init.task_tools, lines(&init.tools), lines(&init.skills), at],
            )
            .map(|_| ())
            .map_err(fault("keeping what a job came up with"))
            .map_err(WriteError::Database)
    }

    /// What `session_id`'s agent last came up with, if its opening line has been read.
    pub fn init_of(&self, session_id: &str) -> Result<Option<KeptInit>, WriteError> {
        self.conn
            .query_row(
                "SELECT harness_version, mod_loaded, door, task_tools, tools, skills FROM session_init WHERE session_id = ?1",
                [session_id],
                |row| {
                    Ok(KeptInit {
                        harness_version: row.get(0)?,
                        mod_loaded: row.get(1)?,
                        door: row.get(2)?,
                        task_tools: row.get(3)?,
                        tools: names(row.get(4)?),
                        skills: names(row.get(5)?),
                    })
                },
            )
            .optional()
            .map_err(fault("reading what a job came up with"))
            .map_err(WriteError::Database)
    }
}
