//! A project's glossary, kept on this Mac. `docs/concepts/feature-catalog.md`, *Glossary*.
//!
//! **A term is unique per project without regard to case**: a write of `Take` replaces what `take` held. Rows only; which
//! words are flagged, and on what, is Fleet's to work out.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Who wrote a term the seed kept.
pub const SEED_JOB: &str = "seed";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptTerm {
    pub project: String,
    pub term: String,
    pub meaning: String,
    /// Words that mean the same and should not be used.
    pub not: Vec<String>,
    /// The Job that wrote it, or [`SEED_JOB`].
    pub job: String,
    pub at: String,
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

fn key(term: &str) -> String {
    term.trim().to_lowercase()
}

const TERM: &str = "project, term, meaning, not_words, job, at";

fn term_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptTerm> {
    let not: String = row.get(3)?;
    Ok(KeptTerm {
        project: row.get(0)?,
        term: row.get(1)?,
        meaning: row.get(2)?,
        not: serde_json::from_str(&not).unwrap_or_default(),
        job: row.get(4)?,
        at: row.get(5)?,
    })
}

fn put(conn: &rusqlite::Connection, kept: &KeptTerm, replacing: bool) -> rusqlite::Result<usize> {
    let not = serde_json::to_string(&kept.not).unwrap_or_else(|_| String::from("[]"));
    let on_conflict = if replacing {
        "DO UPDATE SET term = excluded.term, meaning = excluded.meaning, not_words = excluded.not_words, job = excluded.job, at = excluded.at"
    } else {
        "DO NOTHING"
    };
    conn.execute(
        &format!(
            "INSERT INTO glossary_terms (project, term_key, term, meaning, not_words, job, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (project, term_key) {on_conflict}"
        ),
        params![kept.project, key(&kept.term), kept.term.trim(), kept.meaning.trim(), not, kept.job, kept.at],
    )
}

impl Store {
    /// Keep a term, **replacing the one the project holds under the same word in any case**.
    pub fn put_glossary_term(&mut self, kept: &KeptTerm) -> Result<(), WriteError> {
        put(&self.conn, kept, true).map(drop).map_err(failed("keeping a glossary term"))
    }

    /// The project's terms, in alphabetical order of the word.
    pub fn glossary(&self, project: &str) -> Result<Vec<KeptTerm>, WriteError> {
        let doing = "reading a glossary";
        let mut statement = self.conn.prepare(&format!("SELECT {TERM} FROM glossary_terms WHERE project = ?1 ORDER BY term_key")).map_err(failed(doing))?;
        let rows = statement.query_map([project], term_row).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Keep a seed's terms once for the project, **leaving any term a Job already wrote**. Whether the seed was kept now;
    /// a project whose seed was kept before keeps nothing, whatever became of its terms.
    pub fn keep_glossary_seed(&mut self, project: &str, terms: &[KeptTerm], at: &str) -> Result<bool, WriteError> {
        let doing = "keeping a glossary seed";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        let first = tx.execute("INSERT OR IGNORE INTO glossary_seeded (project, at) VALUES (?1, ?2)", params![project, at]).map_err(failed(doing))?;
        if first == 0 {
            return Ok(false);
        }
        for term in terms {
            put(&tx, term, false).map_err(failed(doing))?;
        }
        tx.commit().map_err(failed(doing))?;
        Ok(true)
    }

    /// Whether the project's seed was kept.
    pub fn glossary_seeded(&self, project: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row("SELECT 1 FROM glossary_seeded WHERE project = ?1", [project], |_| Ok(()))
            .optional()
            .map(|found| found.is_some())
            .map_err(failed("reading whether a glossary was seeded"))
    }
}
