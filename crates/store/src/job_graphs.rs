//! A Job's live graphs and the rows of its feeds. `docs/practices/protocol/a-job-shows-a-live-graph.md`.
//!
//! **Text, not decisions.** A graph is kept as the JSON Fleet checked, and a row as the JSON the agent or a recipe gave;
//! what either means is Fleet's to read. **A row the feed holds already is not added again**, compared as text: a row
//! is a JSON object whose keys `serde_json` writes in order, so one row written twice is one text.

use std::collections::HashSet;

use rusqlite::{params, OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One graph of a Job, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptGraph {
    pub session_id: String,
    pub id: String,
    /// The `show_graph` body Fleet checked, as JSON.
    pub shown: String,
    pub shown_at: String,
    /// Its recipe's interval. `None` where the agent feeds it.
    pub every_seconds: Option<u32>,
    /// It still takes rows: not withdrawn, and its Job not ended.
    pub live: bool,
    /// Its recipe's last run, whether it read or not.
    pub tried_at: Option<String>,
    /// Its recipe's last run that gave rows.
    pub read_at: Option<String>,
    /// Why its recipe's last run gave none.
    pub broken: Option<String>,
}

/// One feed's rows, oldest first, and every row it was ever given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptGraphFeed {
    pub name: String,
    pub rows: Vec<String>,
    pub added: u64,
}

/// What adding rows to a feed came to: the rows that were new, and the feed after.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptGraphRowsAdded {
    pub new: Vec<String>,
    pub kept: usize,
    pub added: u64,
}

const COLUMNS: &str = "session_id, id, shown, shown_at, every_seconds, live, tried_at, read_at, broken";

fn kept(row: &Row<'_>) -> rusqlite::Result<KeptGraph> {
    Ok(KeptGraph {
        session_id: row.get(0)?,
        id: row.get(1)?,
        shown: row.get(2)?,
        shown_at: row.get(3)?,
        every_seconds: row.get(4)?,
        live: row.get(5)?,
        tried_at: row.get(6)?,
        read_at: row.get(7)?,
        broken: row.get(8)?,
    })
}

impl Store {
    /// Keep `graph`, or replace the one of its Job and id **where it stands**: it is live again, and its recipe runs
    /// at the next pass, while its last read and the rows of its feeds stay.
    pub fn keep_graph(&mut self, graph: &KeptGraph) -> Result<(), WriteError> {
        let doing = "keeping a Job's graph";
        self.conn
            .execute(
                "INSERT INTO job_graphs (session_id, id, shown, shown_at, every_seconds, live, tried_at, read_at, broken) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, NULL) \
                 ON CONFLICT (session_id, id) DO UPDATE SET shown = ?3, shown_at = ?4, every_seconds = ?5, live = ?6, \
                 tried_at = NULL, broken = NULL",
                params![graph.session_id, graph.id, graph.shown, graph.shown_at, graph.every_seconds, graph.live],
            )
            .map_err(fault(doing))?;
        Ok(())
    }

    /// `session`'s graphs, in the order first shown.
    pub fn graphs_of(&self, session: &str) -> Result<Vec<KeptGraph>, WriteError> {
        let doing = "reading a Job's graphs";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM job_graphs WHERE session_id = ?1 ORDER BY rowid"))
            .map_err(fault(doing))?;
        let rows = statement.query_map([session], kept).map_err(fault(doing))?;
        Ok(rows.collect::<Result<_, _>>().map_err(fault(doing))?)
    }

    /// The graph `id` of `session`.
    pub fn graph(&self, session: &str, id: &str) -> Result<Option<KeptGraph>, WriteError> {
        let doing = "reading a Job's graph";
        Ok(self
            .conn
            .query_row(&format!("SELECT {COLUMNS} FROM job_graphs WHERE session_id = ?1 AND id = ?2"), [session, id], kept)
            .optional()
            .map_err(fault(doing))?)
    }

    /// Every live graph with a recipe, of every Job, for the loop that runs them.
    pub fn live_graph_recipes(&self) -> Result<Vec<KeptGraph>, WriteError> {
        let doing = "reading the graphs whose recipes run";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM job_graphs WHERE live = 1 AND every_seconds IS NOT NULL ORDER BY rowid"))
            .map_err(fault(doing))?;
        let rows = statement.query_map([], kept).map_err(fault(doing))?;
        Ok(rows.collect::<Result<_, _>>().map_err(fault(doing))?)
    }

    /// The graph `id` of `session` takes no more rows. False where it is not kept or was not live.
    pub fn withdraw_graph(&mut self, session: &str, id: &str) -> Result<bool, WriteError> {
        let doing = "withdrawing a Job's graph";
        let moved = self
            .conn
            .execute("UPDATE job_graphs SET live = 0 WHERE session_id = ?1 AND id = ?2 AND live = 1", [session, id])
            .map_err(fault(doing))?;
        Ok(moved > 0)
    }

    /// Every graph of `session` takes no more rows, as its Job ended. False where none was live.
    pub fn end_graphs_of(&mut self, session: &str) -> Result<bool, WriteError> {
        let doing = "ending a Job's graphs";
        let moved = self
            .conn
            .execute("UPDATE job_graphs SET live = 0 WHERE session_id = ?1 AND live = 1", [session])
            .map_err(fault(doing))?;
        Ok(moved > 0)
    }

    /// The recipe of graph `id` ran at `at`: it gave rows where `broken` is `None`, and the rows kept stay where not.
    pub fn graph_tried(&mut self, session: &str, id: &str, at: &str, broken: Option<&str>) -> Result<(), WriteError> {
        let doing = "keeping a graph recipe's run";
        self.conn
            .execute(
                "UPDATE job_graphs SET tried_at = ?3, broken = ?4, read_at = CASE WHEN ?4 IS NULL THEN ?3 ELSE read_at END \
                 WHERE session_id = ?1 AND id = ?2",
                params![session, id, at, broken],
            )
            .map_err(fault(doing))?;
        Ok(())
    }

    /// Add `rows` to `session`'s `feed`, each one the feed does not hold already, and drop its oldest past `cap`.
    pub fn add_graph_rows(&mut self, session: &str, feed: &str, rows: &[String], cap: usize) -> Result<KeptGraphRowsAdded, WriteError> {
        let doing = "adding rows to a Job's feed";
        let tx = self.conn.transaction().map_err(fault(doing))?;
        let (mut held, mut last): (HashSet<String>, i64) = {
            let mut statement = tx.prepare("SELECT n, row FROM job_graph_rows WHERE session_id = ?1 AND feed = ?2").map_err(fault(doing))?;
            let mut held = HashSet::new();
            let mut last = 0;
            let found = statement
                .query_map([session, feed], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
                .map_err(fault(doing))?;
            for one in found {
                let (n, row) = one.map_err(fault(doing))?;
                last = last.max(n);
                held.insert(row);
            }
            (held, last)
        };
        let mut new = Vec::new();
        for row in rows {
            if !held.insert(row.clone()) {
                continue;
            }
            last += 1;
            tx.execute("INSERT INTO job_graph_rows (session_id, feed, n, row) VALUES (?1, ?2, ?3, ?4)", params![session, feed, last, row])
                .map_err(fault(doing))?;
            new.push(row.clone());
        }
        let cap = i64::try_from(cap).unwrap_or(i64::MAX);
        tx.execute("DELETE FROM job_graph_rows WHERE session_id = ?1 AND feed = ?2 AND n <= ?3", params![session, feed, last - cap])
            .map_err(fault(doing))?;
        let kept: i64 = tx
            .query_row("SELECT COUNT(*) FROM job_graph_rows WHERE session_id = ?1 AND feed = ?2", [session, feed], |row| row.get(0))
            .map_err(fault(doing))?;
        tx.commit().map_err(fault(doing))?;
        Ok(KeptGraphRowsAdded { new, kept: usize::try_from(kept).unwrap_or(0), added: u64::try_from(last).unwrap_or(0) })
    }

    /// Every feed of `session`, by name, with the rows it keeps oldest first.
    pub fn graph_feeds(&self, session: &str) -> Result<Vec<KeptGraphFeed>, WriteError> {
        let doing = "reading a Job's feeds";
        let mut statement = self
            .conn
            .prepare("SELECT feed, n, row FROM job_graph_rows WHERE session_id = ?1 ORDER BY feed, n")
            .map_err(fault(doing))?;
        let found = statement
            .query_map([session], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))
            .map_err(fault(doing))?;
        let mut feeds: Vec<KeptGraphFeed> = Vec::new();
        for one in found {
            let (name, n, row) = one.map_err(fault(doing))?;
            let added = u64::try_from(n).unwrap_or(0);
            match feeds.last_mut() {
                Some(feed) if feed.name == name => {
                    feed.rows.push(row);
                    feed.added = added;
                }
                _ => feeds.push(KeptGraphFeed { name, rows: vec![row], added }),
            }
        }
        Ok(feeds)
    }
}
