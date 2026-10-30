//! What a Job holds, standing in the states of its lifecycles, and every move between them.
//!
//! **Text in, text out.** An instance is a lifecycle id and a key, because the lifecycles are
//! Fleet's to resolve and a row here must not need one. What it last saw is kept in the words its
//! source used, and the standing is worked out from that again by the lifecycle, so a standing in
//! the store is never one a read did not produce.
//!
//! **Only a Job in `lifecycle_jobs` holds any.** A Job started before the table existed has no row
//! and is never given one.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One thing a Job holds, with what a reader last saw of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptInstance {
    pub session_id: String,
    pub lifecycle: String,
    pub key: String,
    /// What it came from, `(lifecycle, key)`, in the order the links were made.
    pub links: Vec<(String, String)>,
    /// Which source said it, as `StateSource::as_wire` spells it. Empty until it is read.
    pub seen_source: String,
    /// What the source said, as `Observed::spelled` wrote it down.
    pub seen: String,
    /// The watch that follows it. Empty for one no watch follows.
    pub watch_id: String,
    pub taken_on_at: String,
    /// When ending the Job ended it, standing where it was. Empty for one that is live, and for one
    /// that reached an end state of its own.
    pub ended_at: String,
}

/// One change of standing, as it was recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptMove {
    pub id: i64,
    pub session_id: String,
    pub lifecycle: String,
    pub key: String,
    /// `Standing::as_wire`.
    pub from: String,
    pub to: String,
    pub seen_source: String,
    pub seen: String,
    pub at: String,
}

/// What a read saw, and the change of standing it made where it made one.
pub struct KeptRead<'a> {
    pub session_id: &'a str,
    pub lifecycle: &'a str,
    pub key: &'a str,
    pub seen_source: &'a str,
    pub seen: &'a str,
    /// `(from, to)` as `Standing::as_wire` spells them. `None` where the read changed nothing.
    pub moved: Option<(&'a str, &'a str)>,
    pub at: &'a str,
}

fn failing(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// `session_id` takes instances on from now. True where it did not already.
    pub fn mark_lifecycle_job(&mut self, session_id: &str, since: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("INSERT OR IGNORE INTO lifecycle_jobs (session_id, since) VALUES (?1, ?2)", params![session_id, since])
            .map(|changed| changed > 0)
            .map_err(failing("marking a job as holding lifecycle instances"))
    }

    /// The agent was refused for what the Job holds, at `at`. **The first refusal stays**, so the
    /// instant is when it was first told to wait.
    pub fn hold_lifecycle_job(&mut self, session_id: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE lifecycle_jobs SET held_at = ?2 WHERE session_id = ?1 AND held_at = ''", params![session_id, at])
            .map(|_| ())
            .map_err(failing("keeping that a job's agent was told to wait on its lifecycle instances"))
    }

    /// Whether the Job's agent was told to wait on its instances and has not yet been told they are over.
    pub fn lifecycle_job_held(&self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row("SELECT 1 FROM lifecycle_jobs WHERE session_id = ?1 AND held_at <> ''", params![session_id], |_| Ok(()))
            .optional()
            .map(|found| found.is_some())
            .map_err(failing("asking whether a job's agent was told to wait on its lifecycle instances"))
    }

    /// The agent was told they are over. **True where it had been told to wait**, and only once.
    pub fn release_lifecycle_job(&mut self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE lifecycle_jobs SET held_at = '' WHERE session_id = ?1 AND held_at <> ''", params![session_id])
            .map(|changed| changed > 0)
            .map_err(failing("clearing that a job's agent was told to wait on its lifecycle instances"))
    }

    pub fn is_lifecycle_job(&self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row("SELECT 1 FROM lifecycle_jobs WHERE session_id = ?1", params![session_id], |_| Ok(()))
            .optional()
            .map(|found| found.is_some())
            .map_err(failing("asking whether a job holds lifecycle instances"))
    }

    /// The Job takes `lifecycle` `key` on, linked to `links`. **A thing is held once**: taken on
    /// again it is not changed, but a link it was not yet given is added. True where it is new.
    pub fn take_on_lifecycle_instance(
        &mut self,
        session_id: &str,
        lifecycle: &str,
        key: &str,
        links: &[(&str, &str)],
        at: &str,
    ) -> Result<bool, WriteError> {
        let doing = "taking a lifecycle instance on";
        let tx = self.conn.transaction().map_err(failing(doing))?;
        let new = tx
            .execute(
                "INSERT OR IGNORE INTO lifecycle_instances (session_id, lifecycle, key, taken_on_at) VALUES (?1, ?2, ?3, ?4)",
                params![session_id, lifecycle, key, at],
            )
            .map_err(failing(doing))?
            > 0;
        for (to_lifecycle, to_key) in links {
            tx.execute(
                "INSERT OR IGNORE INTO lifecycle_links (session_id, lifecycle, key, to_lifecycle, to_key)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session_id, lifecycle, key, to_lifecycle, to_key],
            )
            .map_err(failing(doing))?;
        }
        tx.commit().map_err(failing(doing))?;
        Ok(new)
    }

    /// A watch follows the instance: what it reads is the instance's to be handed.
    pub fn follow_lifecycle_instance_with_watch(
        &mut self,
        session_id: &str,
        lifecycle: &str,
        key: &str,
        watch_id: &str,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE lifecycle_instances SET watch_id = ?4 WHERE session_id = ?1 AND lifecycle = ?2 AND key = ?3",
                params![session_id, lifecycle, key, watch_id],
            )
            .map(|_| ())
            .map_err(failing("following a lifecycle instance with a watch"))
    }

    /// Ending the Job ends `held`, each a `(lifecycle, key)` of its instances, at `at`. **One that
    /// was ended already keeps its first instant**, and a standing is left as it was. The number
    /// ended now.
    pub fn end_lifecycle_instances(&mut self, session_id: &str, held: &[(&str, &str)], at: &str) -> Result<usize, WriteError> {
        let doing = "ending the lifecycle instances a job holds";
        let tx = self.conn.transaction().map_err(failing(doing))?;
        let mut ended = 0;
        for (lifecycle, key) in held {
            ended += tx
                .execute(
                    "UPDATE lifecycle_instances SET ended_at = ?4
                     WHERE session_id = ?1 AND lifecycle = ?2 AND key = ?3 AND ended_at = ''",
                    params![session_id, lifecycle, key, at],
                )
                .map_err(failing(doing))?;
        }
        tx.commit().map_err(failing(doing))?;
        Ok(ended)
    }

    /// Keep what a read saw, and with it the move it made. **One write**, so the instance never
    /// reads as one thing while the log says another. The id of the move, where it made one.
    pub fn keep_lifecycle_read(&mut self, read: &KeptRead<'_>) -> Result<Option<i64>, WriteError> {
        let doing = "keeping what was read of a lifecycle instance";
        let tx = self.conn.transaction().map_err(failing(doing))?;
        tx.execute(
            "UPDATE lifecycle_instances SET seen_source = ?4, seen = ?5
             WHERE session_id = ?1 AND lifecycle = ?2 AND key = ?3",
            params![read.session_id, read.lifecycle, read.key, read.seen_source, read.seen],
        )
        .map_err(failing(doing))?;
        let mut moved = None;
        if let Some((from, to)) = read.moved {
            tx.execute(
                "INSERT INTO lifecycle_moves (session_id, lifecycle, key, from_standing, to_standing, seen_source, seen, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![read.session_id, read.lifecycle, read.key, from, to, read.seen_source, read.seen, read.at],
            )
            .map_err(failing(doing))?;
            moved = Some(tx.last_insert_rowid());
        }
        tx.commit().map_err(failing(doing))?;
        Ok(moved)
    }

    /// Everything the Job holds, in the order it was taken on.
    pub fn lifecycle_instances_of(&self, session_id: &str) -> Result<Vec<KeptInstance>, WriteError> {
        let doing = "reading the lifecycle instances a job holds";
        let mut statement = self
            .conn
            .prepare(
                "SELECT lifecycle, key, seen_source, seen, watch_id, taken_on_at, ended_at FROM lifecycle_instances
                 WHERE session_id = ?1 ORDER BY rowid",
            )
            .map_err(failing(doing))?;
        let mut held = statement
            .query_map(params![session_id], |row| {
                Ok(KeptInstance {
                    session_id: session_id.to_string(),
                    lifecycle: row.get(0)?,
                    key: row.get(1)?,
                    links: Vec::new(),
                    seen_source: row.get(2)?,
                    seen: row.get(3)?,
                    watch_id: row.get(4)?,
                    taken_on_at: row.get(5)?,
                    ended_at: row.get(6)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))?;
        let mut links = self
            .conn
            .prepare(
                "SELECT lifecycle, key, to_lifecycle, to_key FROM lifecycle_links WHERE session_id = ?1 ORDER BY rowid",
            )
            .map_err(failing(doing))?;
        let links = links
            .query_map(params![session_id], |row| {
                Ok(((row.get::<_, String>(0)?, row.get::<_, String>(1)?), (row.get::<_, String>(2)?, row.get::<_, String>(3)?)))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))?;
        for (of, to) in links {
            if let Some(one) = held.iter_mut().find(|one| (&one.lifecycle, &one.key) == (&of.0, &of.1)) {
                one.links.push(to);
            }
        }
        Ok(held)
    }

    /// The live Jobs that hold something of `lifecycle`: **the ones a reader has a reason to
    /// ask after**, since a Job that ended is not read again.
    pub fn live_lifecycle_holders(&self, lifecycle: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading the live jobs that hold a lifecycle";
        let mut statement = self
            .conn
            .prepare(
                "SELECT DISTINCT i.session_id FROM lifecycle_instances i JOIN sessions s ON s.id = i.session_id
                 WHERE i.lifecycle = ?1 AND s.state = 'live' ORDER BY i.session_id",
            )
            .map_err(failing(doing))?;
        let rows = statement
            .query_map(params![lifecycle], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<String>, _>>())
            .map_err(failing(doing))?;
        Ok(rows)
    }

    /// The instances a watch follows, across Jobs.
    pub fn lifecycle_instances_followed_by(&self, watch_id: &str) -> Result<Vec<KeptInstance>, WriteError> {
        let doing = "reading the lifecycle instances a watch follows";
        let sessions: Vec<String> = self
            .conn
            .prepare("SELECT DISTINCT session_id FROM lifecycle_instances WHERE watch_id = ?1")
            .map_err(failing(doing))?
            .query_map(params![watch_id], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))?;
        let mut followed = Vec::new();
        for session in sessions {
            followed.extend(self.lifecycle_instances_of(&session)?.into_iter().filter(|one| one.watch_id == watch_id));
        }
        Ok(followed)
    }

    /// Every move the Job's instances made, oldest first. **Append-only**: nothing here is edited
    /// or forgotten, so a rule can ask for what came after the last one it took.
    pub fn lifecycle_moves_of(&self, session_id: &str) -> Result<Vec<KeptMove>, WriteError> {
        let doing = "reading the moves a job's lifecycle instances made";
        let mut statement = self
            .conn
            .prepare(
                "SELECT id, lifecycle, key, from_standing, to_standing, seen_source, seen, at FROM lifecycle_moves
                 WHERE session_id = ?1 ORDER BY id",
            )
            .map_err(failing(doing))?;
        let rows = statement
            .query_map(params![session_id], |row| {
                Ok(KeptMove {
                    id: row.get(0)?,
                    session_id: session_id.to_string(),
                    lifecycle: row.get(1)?,
                    key: row.get(2)?,
                    from: row.get(3)?,
                    to: row.get(4)?,
                    seen_source: row.get(5)?,
                    seen: row.get(6)?,
                    at: row.get(7)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))?;
        Ok(rows)
    }
}
