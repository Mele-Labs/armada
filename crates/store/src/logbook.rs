//! The Logbook's working record: entries per repository, one habits section and a machine section,
//! each with the evidence behind it, its earlier texts, and a deny list of what was forgotten.
//! `docs/concepts/logbook.md`.
//!
//! **Nothing here decides.** Standing, rewriting and fading are Fleet's Curate; this keeps what it
//! decided and hands it back whole.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Which part of the Logbook an entry is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogSection {
    /// One repository's, under its key.
    Repo,
    /// This Mac's.
    Machine,
    /// The person's habits, shared by every repository.
    Habits,
}

/// What an entry says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogKind {
    Command,
    Gotcha,
    Convention,
    Environment,
    Habit,
}

/// Where an entry stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogState {
    /// Seen once, not served.
    Candidate,
    /// Served.
    Standing,
    /// Unused too long; kept, not served.
    Faded,
}

/// What a Session's evidence did to an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogHow {
    First,
    Confirmed,
    Contradicted,
    Tripped,
}

/// Why an entry's text was replaced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogRewrite {
    Contradicted,
    Tripped,
    Person,
    /// Tidy folded a near-duplicate into it.
    Merged,
}

macro_rules! words {
    ($kind:ident { $($variant:ident => $word:literal),* $(,)? }) => {
        impl $kind {
            pub fn as_text(self) -> &'static str {
                match self { $($kind::$variant => $word),* }
            }
            pub fn of_text(word: &str) -> Option<Self> {
                match word { $($word => Some($kind::$variant),)* _ => None }
            }
        }
    };
}

words!(LogSection { Repo => "repo", Machine => "machine", Habits => "habits" });
words!(LogKind { Command => "command", Gotcha => "gotcha", Convention => "convention", Environment => "environment", Habit => "habit" });
words!(LogState { Candidate => "candidate", Standing => "standing", Faded => "faded" });
words!(LogHow { First => "first", Confirmed => "confirmed", Contradicted => "contradicted", Tripped => "tripped" });
words!(LogRewrite { Contradicted => "contradicted", Tripped => "tripped", Person => "person", Merged => "merged" });

/// One Session's evidence for a claim about an entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEvidence {
    pub session_id: String,
    /// The record rows it cites, `kind:n`.
    pub cites: Vec<String>,
    pub how: LogHow,
    /// A command failed and a different form then worked, in this Session.
    pub hard: bool,
    pub at: u64,
}

/// A text an entry had before it was rewritten.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogHistory {
    pub text: String,
    pub replaced_at: u64,
    pub why: LogRewrite,
}

/// One Logbook entry. `id` is `None` until it is first saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub id: Option<i64>,
    pub section: LogSection,
    /// The repository's key for [`LogSection::Repo`], empty otherwise.
    pub repository: String,
    pub kind: LogKind,
    pub text: String,
    pub paths: Vec<String>,
    pub evidence: Vec<LogEvidence>,
    pub first_seen: u64,
    pub last_confirmed: u64,
    pub last_served: Option<u64>,
    pub state: LogState,
    pub locked: bool,
    pub edited_by_person: bool,
    /// How many times a Session tripped on what it already said.
    pub tripped: u32,
    pub history: Vec<LogHistory>,
}

impl LogEntry {
    /// The distinct Sessions that bore it out: every one that did not contradict it.
    pub fn confirmations(&self) -> usize {
        let mut sessions: Vec<&str> = self
            .evidence
            .iter()
            .filter(|one| one.how != LogHow::Contradicted)
            .map(|one| one.session_id.as_str())
            .collect();
        sessions.sort_unstable();
        sessions.dedup();
        sessions.len()
    }
}

/// What the deny list keys a text by: its words, lowercased, one space apart. Two texts that differ
/// only in case, spacing or punctuation are one.
pub fn logbook_words(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

const COLUMNS: &str = "id, section, repository, kind, text, paths, first_seen, last_confirmed, last_served, \
                       state, locked, edited_by_person, tripped";

fn unreadable(column: usize, word: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column, rusqlite::types::Type::Text, word.into())
}

fn entry_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<LogEntry> {
    let word = |at: usize| row.get::<_, String>(at);
    let paths: String = row.get(5)?;
    Ok(LogEntry {
        id: Some(row.get(0)?),
        section: LogSection::of_text(&word(1)?).ok_or_else(|| unreadable(1, word(1).unwrap_or_default()))?,
        repository: row.get(2)?,
        kind: LogKind::of_text(&word(3)?).ok_or_else(|| unreadable(3, word(3).unwrap_or_default()))?,
        text: row.get(4)?,
        paths: paths.lines().filter(|line| !line.is_empty()).map(str::to_string).collect(),
        evidence: Vec::new(),
        first_seen: row.get::<_, i64>(6)?.max(0) as u64,
        last_confirmed: row.get::<_, i64>(7)?.max(0) as u64,
        last_served: row.get::<_, Option<i64>>(8)?.map(|at| at.max(0) as u64),
        state: LogState::of_text(&word(9)?).ok_or_else(|| unreadable(9, word(9).unwrap_or_default()))?,
        locked: row.get::<_, i64>(10)? != 0,
        edited_by_person: row.get::<_, i64>(11)? != 0,
        tripped: row.get::<_, i64>(12)?.max(0) as u32,
        history: Vec::new(),
    })
}

fn database(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// A repository's entries, then the habits and this Mac's, each with its evidence and history.
    /// Faded entries are in it; forgotten ones are gone.
    pub fn logbook_of(&self, repository: &str) -> Result<Vec<LogEntry>, WriteError> {
        const DOING: &str = "reading the Logbook";
        let mut asked = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM logbook_entries \
                 WHERE (section = 'repo' AND repository = ?1) OR section IN ('habits', 'machine') ORDER BY id"
            ))
            .map_err(database(DOING))?;
        let mut entries: Vec<LogEntry> = asked
            .query_map([repository], entry_of)
            .map_err(database(DOING))?
            .collect::<Result<_, _>>()
            .map_err(database(DOING))?;
        for entry in &mut entries {
            let id = entry.id.unwrap_or_default();
            entry.evidence = self.logbook_evidence(id)?;
            entry.history = self.logbook_history(id)?;
        }
        Ok(entries)
    }

    /// One entry by its id, with its evidence and history. `None` where there is none, as after it
    /// was forgotten.
    pub fn logbook_entry(&self, id: i64) -> Result<Option<LogEntry>, WriteError> {
        let found = self
            .conn
            .query_row(&format!("SELECT {COLUMNS} FROM logbook_entries WHERE id = ?1"), [id], entry_of)
            .optional()
            .map_err(database("reading a Logbook entry"))?;
        let Some(mut entry) = found else { return Ok(None) };
        entry.evidence = self.logbook_evidence(id)?;
        entry.history = self.logbook_history(id)?;
        Ok(Some(entry))
    }

    fn logbook_evidence(&self, id: i64) -> Result<Vec<LogEvidence>, WriteError> {
        const DOING: &str = "reading an entry's evidence";
        let mut asked = self
            .conn
            .prepare("SELECT session_id, cites, how, hard, at FROM logbook_evidence WHERE entry_id = ?1 ORDER BY n")
            .map_err(database(DOING))?;
        let rows = asked
            .query_map([id], |row| {
                let how: String = row.get(2)?;
                let cites: String = row.get(1)?;
                Ok(LogEvidence {
                    session_id: row.get(0)?,
                    cites: cites.split_whitespace().map(str::to_string).collect(),
                    how: LogHow::of_text(&how).ok_or_else(|| unreadable(2, how))?,
                    hard: row.get::<_, i64>(3)? != 0,
                    at: row.get::<_, i64>(4)?.max(0) as u64,
                })
            })
            .map_err(database(DOING))?;
        rows.collect::<Result<_, _>>().map_err(database(DOING))
    }

    fn logbook_history(&self, id: i64) -> Result<Vec<LogHistory>, WriteError> {
        const DOING: &str = "reading an entry's earlier texts";
        let mut asked = self
            .conn
            .prepare("SELECT text, replaced_at, why FROM logbook_history WHERE entry_id = ?1 ORDER BY n")
            .map_err(database(DOING))?;
        let rows = asked
            .query_map([id], |row| {
                let why: String = row.get(2)?;
                Ok(LogHistory {
                    text: row.get(0)?,
                    replaced_at: row.get::<_, i64>(1)?.max(0) as u64,
                    why: LogRewrite::of_text(&why).ok_or_else(|| unreadable(2, why))?,
                })
            })
            .map_err(database(DOING))?;
        rows.collect::<Result<_, _>>().map_err(database(DOING))
    }

    /// Keep an entry whole, its evidence and history with it, and answer its id. **Refused for a
    /// text on the deny list**, so nothing brings a forgotten entry back.
    pub fn save_logbook_entry(&mut self, entry: &LogEntry) -> Result<Option<i64>, WriteError> {
        const DOING: &str = "keeping a Logbook entry";
        if self.logbook_denied(entry.section, &entry.repository, &entry.text)? {
            return Ok(None);
        }
        let tx = self.conn.transaction().map_err(database(DOING))?;
        tx.execute(
            &format!("INSERT OR REPLACE INTO logbook_entries ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"),
            params![
                entry.id,
                entry.section.as_text(),
                entry.repository,
                entry.kind.as_text(),
                entry.text,
                entry.paths.join("\n"),
                entry.first_seen as i64,
                entry.last_confirmed as i64,
                entry.last_served.map(|at| at as i64),
                entry.state.as_text(),
                i64::from(entry.locked),
                i64::from(entry.edited_by_person),
                i64::from(entry.tripped),
            ],
        )
        .map_err(database(DOING))?;
        let id = entry.id.unwrap_or_else(|| tx.last_insert_rowid());
        tx.execute("DELETE FROM logbook_evidence WHERE entry_id = ?1", [id]).map_err(database(DOING))?;
        tx.execute("DELETE FROM logbook_history WHERE entry_id = ?1", [id]).map_err(database(DOING))?;
        for (n, one) in entry.evidence.iter().enumerate() {
            tx.execute(
                "INSERT INTO logbook_evidence (entry_id, n, session_id, cites, how, hard, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, n as i64, one.session_id, one.cites.join(" "), one.how.as_text(), i64::from(one.hard), one.at as i64],
            )
            .map_err(database(DOING))?;
        }
        for (n, one) in entry.history.iter().enumerate() {
            tx.execute(
                "INSERT INTO logbook_history (entry_id, n, text, replaced_at, why) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, n as i64, one.text, one.replaced_at as i64, one.why.as_text()],
            )
            .map_err(database(DOING))?;
        }
        tx.commit().map_err(database(DOING))?;
        Ok(Some(id))
    }

    /// Whether a text is on the deny list for its section and repository.
    pub fn logbook_denied(&self, section: LogSection, repository: &str, text: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                "SELECT 1 FROM logbook_forgotten WHERE section = ?1 AND repository = ?2 AND words = ?3",
                params![section.as_text(), repository, logbook_words(text)],
                |_| Ok(()),
            )
            .optional()
            .map(|found| found.is_some())
            .map_err(database("reading what the Logbook forgot"))
    }

    /// Forget an entry: its text goes on the deny list, every text it had with it, and the entry
    /// goes. `false` where there was no such entry.
    pub fn forget_logbook_entry(&mut self, id: i64, at: u64) -> Result<bool, WriteError> {
        const DOING: &str = "forgetting a Logbook entry";
        let entry = self
            .conn
            .query_row(&format!("SELECT {COLUMNS} FROM logbook_entries WHERE id = ?1"), [id], entry_of)
            .optional()
            .map_err(database(DOING))?;
        let Some(entry) = entry else { return Ok(false) };
        let history = self.logbook_history(id)?;
        let tx = self.conn.transaction().map_err(database(DOING))?;
        for text in std::iter::once(&entry.text).chain(history.iter().map(|one| &one.text)) {
            tx.execute(
                "INSERT OR IGNORE INTO logbook_forgotten (section, repository, words, forgotten_at) VALUES (?1, ?2, ?3, ?4)",
                params![entry.section.as_text(), entry.repository, logbook_words(text), at as i64],
            )
            .map_err(database(DOING))?;
        }
        for table in ["logbook_entries WHERE id", "logbook_evidence WHERE entry_id", "logbook_history WHERE entry_id"] {
            tx.execute(&format!("DELETE FROM {table} = ?1"), [id]).map_err(database(DOING))?;
        }
        tx.commit().map_err(database(DOING))?;
        Ok(true)
    }

    /// An entry a person wrote: it stands at once, marked theirs, with no Session behind it unless `evidence` names the one it was taught from. **A
    /// person's own act lifts the deny list** for that text, since teaching it is asking it back.
    pub fn teach_logbook_entry(
        &mut self,
        section: LogSection,
        repository: &str,
        kind: LogKind,
        text: &str,
        paths: &[String],
        evidence: Option<LogEvidence>,
        at: u64,
    ) -> Result<i64, WriteError> {
        let repository = if section == LogSection::Repo { repository } else { "" };
        self.conn
            .execute(
                "DELETE FROM logbook_forgotten WHERE section = ?1 AND repository = ?2 AND words = ?3",
                params![section.as_text(), repository, logbook_words(text)],
            )
            .map_err(database("lifting a forgotten Logbook entry"))?;
        let entry = LogEntry {
            id: None,
            section,
            repository: repository.to_string(),
            kind,
            text: text.trim().to_string(),
            paths: paths.to_vec(),
            evidence: evidence.into_iter().collect(),
            first_seen: at,
            last_confirmed: at,
            last_served: None,
            state: LogState::Standing,
            locked: false,
            edited_by_person: true,
            tripped: 0,
            history: Vec::new(),
        };
        self.save_logbook_entry(&entry)?.ok_or_else(|| WriteError::Database(fault("teaching a Logbook entry")(rusqlite::Error::QueryReturnedNoRows)))
    }

    /// Lock an entry, or let it go: a locked entry never fades and is never rewritten.
    pub fn lock_logbook_entry(&mut self, id: i64, locked: bool) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE logbook_entries SET locked = ?2 WHERE id = ?1", params![id, i64::from(locked)])
            .map(|_| ())
            .map_err(database("locking a Logbook entry"))
    }

    /// Say an entry was served at `at`, which keeps it from fading.
    pub fn logbook_served(&mut self, ids: &[i64], at: u64) -> Result<(), WriteError> {
        for id in ids {
            self.conn
                .execute("UPDATE logbook_entries SET last_served = ?2 WHERE id = ?1", params![id, at as i64])
                .map_err(database("saying a Logbook entry was served"))?;
        }
        Ok(())
    }

    /// Whether the pass that moved the open lessons' repository facts into the Logbook finished.
    pub fn logbook_backlog_moved(&self) -> Result<bool, WriteError> {
        self.conn
            .query_row("SELECT 1 FROM logbook_backlog_moved WHERE id = 1", [], |_| Ok(()))
            .optional()
            .map(|found| found.is_some())
            .map_err(database("reading whether the backlog was moved"))
    }

    /// Keep that the pass finished, so it never runs again.
    pub fn logbook_backlog_finished(&mut self, at: u64) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT OR IGNORE INTO logbook_backlog_moved (id, at) VALUES (1, ?1)", [at as i64])
            .map(|_| ())
            .map_err(database("keeping that the backlog was moved"))
    }

    /// When a repository's Logbook was last tidied. `None` before its first.
    pub fn logbook_last_tidied(&self, repository: &str) -> Result<Option<u64>, WriteError> {
        self.conn
            .query_row("SELECT at FROM logbook_tidied WHERE repository = ?1", [repository], |row| row.get::<_, i64>(0))
            .optional()
            .map(|at| at.map(|at| at.max(0) as u64))
            .map_err(database("reading when the Logbook was last tidied"))
    }

    /// Keep that a repository's Logbook was tidied at `at`.
    pub fn logbook_tidy_finished(&mut self, repository: &str, at: u64) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT OR REPLACE INTO logbook_tidied (repository, at) VALUES (?1, ?2)", params![repository, at as i64])
            .map(|_| ())
            .map_err(database("keeping that the Logbook was tidied"))
    }

    /// Keep that a Session was distilled up to `covers_to`, for its repository's key.
    pub fn logbook_distilled(&mut self, session: &str, repository: &str, covers_to: &str, at: u64) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO logbook_distils (session_id, repository, covers_to, at) VALUES (?1, ?2, ?3, ?4)",
                params![session, repository, covers_to, at as i64],
            )
            .map(|_| ())
            .map_err(database("keeping a distil"))
    }

    /// Where a Session's last distil ended. `None` before its first.
    pub fn logbook_last_distil(&self, session: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row(
                "SELECT covers_to FROM logbook_distils WHERE session_id = ?1 ORDER BY at DESC, rowid DESC LIMIT 1",
                [session],
                |row| row.get(0),
            )
            .optional()
            .map_err(database("reading where a Session's last distil ended"))
    }

    /// How many Sessions of a repository were distilled after `since`; every repository's where
    /// `repository` is empty.
    pub fn logbook_sessions_since(&self, repository: &str, since: u64) -> Result<usize, WriteError> {
        self.conn
            .query_row(
                "SELECT COUNT(DISTINCT session_id) FROM logbook_distils WHERE at > ?2 AND (?1 = '' OR repository = ?1)",
                params![repository, since as i64],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count.max(0) as usize)
            .map_err(database("counting a repository's Sessions"))
    }
}
