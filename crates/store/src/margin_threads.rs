//! His margin threads on a Job's pages, and the rounds of margin talk sent to the Job. `docs/concepts/document.md`,
//! *Margin threads*.
//!
//! **A thread is kept whole**: its row and every message, replaced together, so a reader never sees half of a change.

use rusqlite::{params, OptionalExtension, Transaction};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One message in a thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptMarginMessage {
    /// `you` or `agent`.
    pub voice: String,
    pub text: String,
    pub at: String,
    pub held: bool,
}

/// An edit the agent suggested, and how he decided it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptMarginSuggestion {
    pub struck: String,
    pub added: String,
    pub at: String,
    /// `accepted` or `rejected`, once decided.
    pub decided: Option<String>,
    pub decided_at: Option<String>,
    pub decided_revision: Option<u32>,
}

/// One margin thread on a passage of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptMarginThread {
    pub id: String,
    /// The page's file name.
    pub document: String,
    pub quote: String,
    pub block: Option<String>,
    pub section: Option<String>,
    /// `open`, `answered`, `suggested`, `held` or `settled`.
    pub state: String,
    pub gone: bool,
    pub suggestion: Option<KeptMarginSuggestion>,
    pub outcome: Option<String>,
    pub opened_at: String,
    /// Oldest first.
    pub messages: Vec<KeptMarginMessage>,
}

/// One note of a round.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptMarginNote {
    pub thread: Option<String>,
    pub section: Option<String>,
    pub passage: Option<String>,
    pub note: String,
}

/// One round of margin talk: a message sent to the Job, and the row it wrote where that is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptMarginRound {
    /// Given by [`Store::keep_margin_round`]; ignored when keeping.
    pub seq: u32,
    pub at: String,
    pub document: String,
    pub title: String,
    pub kind: String,
    pub row: Option<String>,
    pub threads: Vec<String>,
    pub notes: Vec<KeptMarginNote>,
}

const THREAD_COLUMNS: &str = "id, document, quote, block, section, state, gone, struck, added, suggested_at, decided, decided_at, \
                              decided_revision, outcome, opened_at";

fn thread_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptMarginThread> {
    let struck: Option<String> = row.get(7)?;
    let suggestion = match struck {
        Some(struck) => Some(KeptMarginSuggestion {
            struck,
            added: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            at: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
            decided: row.get(10)?,
            decided_at: row.get(11)?,
            decided_revision: row.get(12)?,
        }),
        None => None,
    };
    Ok(KeptMarginThread {
        id: row.get(0)?,
        document: row.get(1)?,
        quote: row.get(2)?,
        block: row.get(3)?,
        section: row.get(4)?,
        state: row.get(5)?,
        gone: row.get::<_, i64>(6)? != 0,
        suggestion,
        outcome: row.get(13)?,
        opened_at: row.get(14)?,
        messages: Vec::new(),
    })
}

fn messages_of(transaction: &Transaction<'_>, session: &str, thread: &str) -> rusqlite::Result<Vec<KeptMarginMessage>> {
    let mut statement =
        transaction.prepare("SELECT voice, text, at, held FROM margin_messages WHERE session_id = ?1 AND thread = ?2 ORDER BY seq")?;
    let rows = statement.query_map(params![session, thread], |row| {
        Ok(KeptMarginMessage { voice: row.get(0)?, text: row.get(1)?, at: row.get(2)?, held: row.get::<_, i64>(3)? != 0 })
    })?;
    rows.collect()
}

fn threads_where(transaction: &Transaction<'_>, session: &str, document: Option<&str>) -> rusqlite::Result<Vec<KeptMarginThread>> {
    let mut threads = {
        let mut statement = transaction.prepare(&format!(
            "SELECT {THREAD_COLUMNS} FROM margin_threads WHERE session_id = ?1 AND (?2 IS NULL OR document = ?2) ORDER BY opened_at, rowid"
        ))?;
        let rows = statement.query_map(params![session, document], thread_of)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for thread in &mut threads {
        thread.messages = messages_of(transaction, session, &thread.id)?;
    }
    Ok(threads)
}

impl Store {
    /// Every margin thread of `session`, or only those on page `document`, oldest first.
    pub fn margin_threads(&mut self, session: &str, document: Option<&str>) -> Result<Vec<KeptMarginThread>, WriteError> {
        let doing = "reading a Job's margin threads";
        let read = (|| {
            let transaction = self.conn.transaction()?;
            let threads = threads_where(&transaction, session, document)?;
            transaction.commit()?;
            Ok(threads)
        })();
        read.map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Margin thread `id` of `session`, where there is one.
    pub fn margin_thread(&mut self, session: &str, id: &str) -> Result<Option<KeptMarginThread>, WriteError> {
        let doing = "reading a margin thread";
        let read = (|| {
            let transaction = self.conn.transaction()?;
            let thread = transaction
                .query_row(&format!("SELECT {THREAD_COLUMNS} FROM margin_threads WHERE session_id = ?1 AND id = ?2"), params![session, id], thread_of)
                .optional()?;
            let thread = match thread {
                Some(mut thread) => {
                    thread.messages = messages_of(&transaction, session, id)?;
                    Some(thread)
                }
                None => None,
            };
            transaction.commit()?;
            Ok(thread)
        })();
        read.map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Keep each thread whole, its messages replaced by the ones it holds.
    pub fn keep_margin_threads(&mut self, session: &str, threads: &[KeptMarginThread]) -> Result<(), WriteError> {
        let doing = "keeping margin threads";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            for thread in threads {
                let suggestion = thread.suggestion.as_ref();
                transaction.execute(
                    &format!(
                        "INSERT OR REPLACE INTO margin_threads (session_id, {THREAD_COLUMNS})
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
                    ),
                    params![
                        session,
                        thread.id,
                        thread.document,
                        thread.quote,
                        thread.block,
                        thread.section,
                        thread.state,
                        thread.gone as i64,
                        suggestion.map(|one| &one.struck),
                        suggestion.map(|one| &one.added),
                        suggestion.map(|one| &one.at),
                        suggestion.and_then(|one| one.decided.as_ref()),
                        suggestion.and_then(|one| one.decided_at.as_ref()),
                        suggestion.and_then(|one| one.decided_revision),
                        thread.outcome,
                        thread.opened_at,
                    ],
                )?;
                transaction.execute("DELETE FROM margin_messages WHERE session_id = ?1 AND thread = ?2", params![session, thread.id])?;
                for (seq, message) in thread.messages.iter().enumerate() {
                    transaction.execute(
                        "INSERT INTO margin_messages (session_id, thread, seq, voice, text, at, held) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![session, thread.id, seq as i64, message.voice, message.text, message.at, message.held as i64],
                    )?;
                }
            }
            transaction.commit()
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// How many margin threads `session` has opened, which names the next one.
    pub fn margin_thread_count(&self, session: &str) -> Result<u32, WriteError> {
        let doing = "counting a Job's margin threads";
        self.conn
            .query_row("SELECT COUNT(*) FROM margin_threads WHERE session_id = ?1", [session], |row| row.get(0))
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Keep a round, numbered after the newest. Answers its number.
    pub fn keep_margin_round(&mut self, session: &str, round: &KeptMarginRound) -> Result<u32, WriteError> {
        let doing = "keeping a round of margin talk";
        let threads = serde_json::to_string(&round.threads).unwrap_or_else(|_| String::from("[]"));
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            let seq: u32 = transaction.query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM margin_rounds WHERE session_id = ?1",
                [session],
                |row| row.get(0),
            )?;
            transaction.execute(
                "INSERT INTO margin_rounds (session_id, seq, at, document, title, kind, row_id, threads) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![session, seq, round.at, round.document, round.title, round.kind, round.row, threads],
            )?;
            for (at, note) in round.notes.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO margin_round_notes (session_id, round, seq, thread, section, passage, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![session, seq, at as i64, note.thread, note.section, note.passage, note.note],
                )?;
            }
            transaction.commit()?;
            Ok(seq)
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// The row round `seq` wrote, found after its send.
    pub fn set_margin_round_row(&mut self, session: &str, seq: u32, row: &str) -> Result<(), WriteError> {
        let doing = "keeping the row of a round of margin talk";
        self.conn
            .execute("UPDATE margin_rounds SET row_id = ?3 WHERE session_id = ?1 AND seq = ?2", params![session, seq, row])
            .map(drop)
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Every round of `session`, oldest first.
    pub fn margin_rounds(&mut self, session: &str) -> Result<Vec<KeptMarginRound>, WriteError> {
        let doing = "reading a Job's rounds of margin talk";
        let read = (|| {
            let transaction = self.conn.transaction()?;
            let mut rounds = {
                let mut statement = transaction
                    .prepare("SELECT seq, at, document, title, kind, row_id, threads FROM margin_rounds WHERE session_id = ?1 ORDER BY seq")?;
                let rows = statement.query_map([session], |row| {
                    let threads: String = row.get(6)?;
                    Ok(KeptMarginRound {
                        seq: row.get(0)?,
                        at: row.get(1)?,
                        document: row.get(2)?,
                        title: row.get(3)?,
                        kind: row.get(4)?,
                        row: row.get(5)?,
                        threads: serde_json::from_str(&threads).unwrap_or_default(),
                        notes: Vec::new(),
                    })
                })?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            for round in &mut rounds {
                let mut statement = transaction.prepare(
                    "SELECT thread, section, passage, note FROM margin_round_notes WHERE session_id = ?1 AND round = ?2 ORDER BY seq",
                )?;
                let notes = statement.query_map(params![session, round.seq], |row| {
                    Ok(KeptMarginNote { thread: row.get(0)?, section: row.get(1)?, passage: row.get(2)?, note: row.get(3)? })
                })?;
                round.notes = notes.collect::<rusqlite::Result<Vec<_>>>()?;
            }
            transaction.commit()?;
            Ok(rounds)
        })();
        read.map_err(fault(doing)).map_err(WriteError::Database)
    }
}
