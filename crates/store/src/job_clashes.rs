//! Two Jobs that write the same files, and the order they settled. `docs/concepts/fleet.md`, *Claims a Job holds*.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// How a clash stands, as the table names it.
pub mod standing {
    /// The two agreed, with `settle_clash`.
    pub const AGREED: &str = "agreed";
    /// The person chose which lands first.
    pub const RULED: &str = "ruled";
    /// Raised to the person and not chosen yet.
    pub const WAITING: &str = "waiting";
}

/// One pair's clash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptClash {
    /// The two session ids, sorted.
    pub pair: [String; 2],
    /// The Job that settled or raised it last.
    pub recorded_by: String,
    /// Each Job's title, in the order of `pair`.
    pub titles: [String; 2],
    pub files: Vec<String>,
    pub standing: String,
    /// The session that lands first. `None` where both go on, or while it waits.
    pub first: Option<String>,
    pub why: String,
    pub at: String,
}

impl KeptClash {
    /// The two of `one` and `other`, sorted as the table keys them.
    pub fn pair_of(one: &str, other: &str) -> [String; 2] {
        let mut pair = [one.to_string(), other.to_string()];
        pair.sort();
        pair
    }

    /// Whether the pair settled `path`: agreed or ruled on it, or on a folder that holds it.
    pub fn settles(&self, path: &str) -> bool {
        self.standing != standing::WAITING && self.files.iter().any(|file| path == file || path.strip_prefix(file.as_str()).is_some_and(|rest| rest.starts_with('/')))
    }
}

const COLUMNS: &str = "pair_a, pair_b, recorded_by, a_title, b_title, files, standing, first, why, at";

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptClash> {
    Ok(KeptClash {
        pair: [row.get(0)?, row.get(1)?],
        recorded_by: row.get(2)?,
        titles: [row.get(3)?, row.get(4)?],
        files: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
        standing: row.get(6)?,
        first: row.get(7)?,
        why: row.get(8)?,
        at: row.get(9)?,
    })
}

impl Store {
    /// Keep `clash`, in place of what the pair held before.
    pub fn put_job_clash(&mut self, clash: &KeptClash) -> Result<(), WriteError> {
        let files = serde_json::to_string(&clash.files).unwrap_or_else(|_| String::from("[]"));
        self.conn
            .execute(
                &format!("INSERT OR REPLACE INTO job_clashes ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"),
                params![clash.pair[0], clash.pair[1], clash.recorded_by, clash.titles[0], clash.titles[1], files, clash.standing, clash.first, clash.why, clash.at],
            )
            .map(|_| ())
            .map_err(failed("keeping the order two Jobs settled"))
    }

    /// The clash between `one` and `other`, either way round.
    pub fn job_clash(&self, one: &str, other: &str) -> Result<Option<KeptClash>, WriteError> {
        let [a, b] = KeptClash::pair_of(one, other);
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM job_clashes WHERE pair_a = ?1 AND pair_b = ?2"), params![a, b], kept)
            .optional()
            .map_err(failed("reading the order two Jobs settled"))
    }

    /// Every clash `session` is one of two in, oldest first.
    pub fn job_clashes_of(&self, session: &str) -> Result<Vec<KeptClash>, WriteError> {
        let doing = "reading the order two Jobs settled";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM job_clashes WHERE pair_a = ?1 OR pair_b = ?1 ORDER BY at, pair_a, pair_b"))
            .map_err(failed(doing))?;
        let rows = statement.query_map([session], kept).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
