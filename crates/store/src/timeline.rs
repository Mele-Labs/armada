//! What Fleet saw happen, one row per thing, for the Cockpit's timeline.
//!
//! **Append-only, and pruned on write** to [`KEPT_DAYS`], as `check_runs` is.
//! The row is the fact as it read when it happened — the title included — so a
//! Job forgotten since still has its history, and nothing here is edited.
//!
//! **Strings, not the wire's types**: this crate does not know `ipc`, and the
//! kind, the subject and the tone are spelt by whoever writes them. Fleet turns
//! a row it cannot spell back into nothing rather than guessing.

use rusqlite::Row;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// How long a row is kept, counted from the one being written.
const KEPT_DAYS: u32 = 30;

/// One thing that happened, before the store numbers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineEntry {
    /// RFC3339, UTC, in Fleet's clock's shape: compared as text.
    pub at: String,
    pub kind: String,
    /// `job`, `session`, `pull_request` or `main`.
    pub subject_kind: String,
    /// The Job's or Session's id, the pull request's number, or empty for main.
    pub subject_id: String,
    /// A pull request's address.
    pub subject_url: Option<String>,
    pub title: Option<String>,
    /// The Manifest id of the repository it belongs to.
    pub repo: Option<String>,
    pub tone: String,
    pub says: String,
    pub detail: Option<String>,
    /// What a burst counted, newest first: the files or commands behind `says`. **Each is one line**, which
    /// is what lets the column hold them joined by a newline.
    pub items: Vec<String>,
    /// The service a call went to, for a `session.called` entry: its line is `detail`.
    pub service: Option<String>,
    /// What that service is called, for the same entry.
    pub service_name: Option<String>,
    pub live: bool,
    pub over: bool,
}

/// An entry as kept: numbered, in the order it was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptTimeline {
    pub seq: i64,
    pub entry: TimelineEntry,
}

/// A window of the timeline, oldest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineWindow {
    pub events: Vec<KeptTimeline>,
    /// Older entries inside the window were left out to keep to the limit.
    pub cut: bool,
}

impl Store {
    /// Keep one entry and answer its number, dropping what is older than
    /// [`KEPT_DAYS`] before it.
    pub fn append_timeline(&mut self, entry: &TimelineEntry) -> Result<i64, WriteError> {
        let failed = |why| WriteError::Database(fault("appending to the timeline")(why));
        let kept = self.conn.transaction().map_err(failed)?;
        kept.execute(
            "INSERT INTO timeline_events (at, kind, subject_kind, subject_id, subject_url, title,
                 repo, tone, says, detail, items, live, over, service, service_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                entry.at,
                entry.kind,
                entry.subject_kind,
                entry.subject_id,
                entry.subject_url,
                entry.title,
                entry.repo,
                entry.tone,
                entry.says,
                entry.detail,
                (!entry.items.is_empty()).then(|| entry.items.join("\n")),
                entry.live,
                entry.over,
                entry.service,
                entry.service_name,
            ],
        )
        .map_err(failed)?;
        let seq = kept.last_insert_rowid();
        // Compared as text: every stamp is the same-shape UTC RFC3339.
        kept.execute(
            "DELETE FROM timeline_events
             WHERE at < strftime('%Y-%m-%dT%H:%M:%SZ', ?1, ?2)",
            rusqlite::params![entry.at, format!("-{KEPT_DAYS} days")],
        )
        .map_err(failed)?;
        kept.commit().map_err(failed)?;
        Ok(seq)
    }

    /// The newest `limit` entries with `since <= at < until`, oldest first. An
    /// absent bound is open on that side. `cut` says older ones were left out.
    pub fn timeline_between(
        &self,
        since: Option<&str>,
        until: Option<&str>,
        limit: u32,
    ) -> Result<TimelineWindow, WriteError> {
        let failed = |why| WriteError::Database(fault("reading the timeline")(why));
        let mut read = self
            .conn
            .prepare(
                "SELECT seq, at, kind, subject_kind, subject_id, subject_url, title, repo, tone, says,
                        detail, items, live, over, service, service_name
                 FROM timeline_events
                 WHERE (?1 IS NULL OR at >= ?1) AND (?2 IS NULL OR at < ?2)
                 ORDER BY at DESC, seq DESC
                 LIMIT ?3",
            )
            .map_err(failed)?;
        let mut events = read
            .query_map(rusqlite::params![since, until, i64::from(limit) + 1], kept)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failed)?;
        let cut = events.len() > limit as usize;
        events.truncate(limit as usize);
        events.reverse();
        Ok(TimelineWindow { events, cut })
    }
}

fn kept(row: &Row<'_>) -> rusqlite::Result<KeptTimeline> {
    Ok(KeptTimeline {
        seq: row.get(0)?,
        entry: TimelineEntry {
            at: row.get(1)?,
            kind: row.get(2)?,
            subject_kind: row.get(3)?,
            subject_id: row.get(4)?,
            subject_url: row.get(5)?,
            title: row.get(6)?,
            repo: row.get(7)?,
            tone: row.get(8)?,
            says: row.get(9)?,
            detail: row.get(10)?,
            items: row.get::<_, Option<String>>(11)?.map_or_else(Vec::new, |joined| joined.lines().map(str::to_string).collect()),
            service: row.get(14)?,
            service_name: row.get(15)?,
            live: row.get(12)?,
            over: row.get(13)?,
        },
    })
}
