//! What Fleet observed of a Session's work, and the road it took read back off the timeline. The
//! build record is derived from these on every read (`docs/concepts/session.md`, *The build record*).
//!
//! **Strings, not the wire's types**: this crate does not know `ipc`. A status is `pending`,
//! `in_progress` or `completed`, and an instant is the same-shape UTC RFC3339 every other stamp is,
//! compared as text.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The instants of one build. A Session has one row once it has done anything Fleet observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptBuild {
    /// When this build began: the first work Fleet observed, or the first after the last one landed.
    pub begun_at: String,
    pub first_edit_at: Option<String>,
    pub last_edit_at: Option<String>,
    /// The last read, edit, command or task list.
    pub last_tool_at: Option<String>,
    pub plan_left_at: Option<String>,
    /// When the task list was first written. Cleared with the list.
    pub tasks_at: Option<String>,
}

/// One task of the agent's list as kept. Its place in the list is its position in the vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptTask {
    pub id: String,
    pub text: String,
    pub status: String,
    pub first_seen_at: String,
    /// When Fleet first saw it completed. Absent while it is not.
    pub completed_at: Option<String>,
    /// An edit was seen between the previous completion (or the list's first write) and this one.
    pub edited: bool,
    /// The label of the subagent that wrote it, as `workers` shows it. Absent for the Session's own.
    pub made_by: Option<String>,
}

/// Which stretch of a timeline a road is read from.
#[derive(Clone, Copy, Debug)]
pub struct RoadAsk<'a> {
    pub session: &'a str,
    pub manifest: &'a str,
    /// The pull request's number, where the Session has one.
    pub pull: Option<&'a str>,
    /// Entries from this instant.
    pub from: &'a str,
    /// A walk counts from this instant: the first edit.
    pub walk_from: &'a str,
    /// Entries up to and including this instant. Absent is up to now.
    pub until: Option<&'a str>,
}

/// When each stop was first reached inside the stretch, as the timeline entered it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoadTimes {
    pub walked: Option<String>,
    pub pushed: Option<String>,
    pub opened: Option<String>,
    pub checked: Option<String>,
    pub queued: Option<String>,
    pub merged: Option<String>,
}

/// When a Session's own work was entered on the timeline, inside a stretch. A burst is dated by its
/// first call, so the last edit here is when the last burst began.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkTimes {
    /// The first and the last `session.edited` entry.
    pub edited: Option<(String, String)>,
    pub first_commit: Option<String>,
    pub first_push: Option<String>,
}

impl WorkTimes {
    pub fn is_empty(&self) -> bool {
        self.edited.is_none() && self.first_commit.is_none() && self.first_push.is_none()
    }
}

/// A pull request a Session held that merged: who, which, and when the forge says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LandedPull {
    pub session: String,
    pub number: String,
    pub merged_at: String,
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading what a job built")(why))
}

fn keeping(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("keeping what a job built")(why))
}

impl Store {
    pub fn session_build(&self, session_id: &str) -> Result<Option<KeptBuild>, WriteError> {
        self.conn
            .query_row(
                "SELECT begun_at, first_edit_at, last_edit_at, last_tool_at, plan_left_at, tasks_at
                 FROM session_build WHERE session_id = ?1",
                [session_id],
                |row| {
                    Ok(KeptBuild {
                        begun_at: row.get(0)?,
                        first_edit_at: row.get(1)?,
                        last_edit_at: row.get(2)?,
                        last_tool_at: row.get(3)?,
                        plan_left_at: row.get(4)?,
                        tasks_at: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(reading)
    }

    /// Keep the row whole, replacing the one held.
    pub fn keep_session_build(&mut self, session_id: &str, build: &KeptBuild) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO session_build
                     (session_id, begun_at, first_edit_at, last_edit_at, last_tool_at, plan_left_at, tasks_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session_id,
                    build.begun_at,
                    build.first_edit_at,
                    build.last_edit_at,
                    build.last_tool_at,
                    build.plan_left_at,
                    build.tasks_at,
                ],
            )
            .map(|_| ())
            .map_err(keeping)
    }

    /// The list in the order the agent wrote it.
    pub fn session_build_tasks(&self, session_id: &str) -> Result<Vec<KeptTask>, WriteError> {
        let mut read = self
            .conn
            .prepare(
                "SELECT task_id, text, status, first_seen_at, completed_at, edited, made_by
                 FROM session_build_tasks WHERE session_id = ?1 ORDER BY position",
            )
            .map_err(reading)?;
        let rows = read
            .query_map([session_id], |row| {
                Ok(KeptTask {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    status: row.get(2)?,
                    first_seen_at: row.get(3)?,
                    completed_at: row.get(4)?,
                    edited: row.get::<_, i64>(5)? == 1,
                    made_by: row.get(6)?,
                })
            })
            .map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }

    /// Replace the list whole, in one transaction.
    pub fn keep_session_build_tasks(&mut self, session_id: &str, tasks: &[KeptTask]) -> Result<(), WriteError> {
        let kept = self.conn.transaction().map_err(keeping)?;
        kept.execute("DELETE FROM session_build_tasks WHERE session_id = ?1", [session_id])
            .map_err(keeping)?;
        for (position, task) in tasks.iter().enumerate() {
            kept.execute(
                "INSERT INTO session_build_tasks
                     (session_id, position, task_id, text, status, first_seen_at, completed_at, edited, made_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    session_id,
                    position as i64,
                    task.id,
                    task.text,
                    task.status,
                    task.first_seen_at,
                    task.completed_at,
                    task.edited,
                    task.made_by,
                ],
            )
            .map_err(keeping)?;
        }
        kept.commit().map_err(keeping)
    }

    /// Let a build go, so the next work starts another.
    pub fn forget_session_build(&mut self, session_id: &str) -> Result<(), WriteError> {
        let kept = self.conn.transaction().map_err(keeping)?;
        kept.execute("DELETE FROM session_build WHERE session_id = ?1", [session_id]).map_err(keeping)?;
        kept.execute("DELETE FROM session_build_tasks WHERE session_id = ?1", [session_id]).map_err(keeping)?;
        kept.commit().map_err(keeping)
    }

    /// When each stop of the road was first reached inside `ask`'s stretch. A walk, a push and the
    /// Session's own entries are the Session's; the rest are its pull request's.
    pub fn session_road(&self, ask: &RoadAsk<'_>) -> Result<RoadTimes, WriteError> {
        let session = |kind: &str, from: &str| self.first_entry("session", ask.session, None, kind, from, ask.until);
        let pull = |kind: &str| match ask.pull {
            Some(number) => self.first_entry("pull_request", number, Some(ask.manifest), kind, ask.from, ask.until),
            None => Ok(None),
        };
        Ok(RoadTimes {
            walked: session("session.walked", ask.walk_from)?,
            pushed: session("session.pushed", ask.from)?,
            opened: pull("pr.opened")?,
            checked: pull("pr.checks")?,
            queued: pull("pr.queued")?,
            merged: pull("pr.merged")?,
        })
    }

    /// What the timeline kept of a Session's edits, commits and pushes from `from` on, in one read.
    pub fn session_work(&self, session: &str, from: &str) -> Result<WorkTimes, WriteError> {
        let mut read = self
            .conn
            .prepare(
                "SELECT kind, MIN(at), MAX(at) FROM timeline_events
                 WHERE subject_kind = 'session' AND subject_id = ?1 AND at >= ?2
                   AND kind IN ('session.edited', 'session.committed', 'session.pushed')
                 GROUP BY kind",
            )
            .map_err(reading)?;
        let rows = read
            .query_map(params![session, from], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(reading)?;
        let mut work = WorkTimes::default();
        for row in rows {
            let (kind, first, last) = row.map_err(reading)?;
            match kind.as_str() {
                "session.edited" => work.edited = Some((first, last)),
                "session.committed" => work.first_commit = Some(first),
                _ => work.first_push = Some(first),
            }
        }
        Ok(work)
    }

    fn first_entry(
        &self,
        subject_kind: &str,
        subject_id: &str,
        repo: Option<&str>,
        kind: &str,
        from: &str,
        until: Option<&str>,
    ) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row(
                "SELECT MIN(at) FROM timeline_events
                 WHERE subject_kind = ?1 AND subject_id = ?2 AND kind = ?3
                   AND (?4 IS NULL OR repo = ?4) AND at >= ?5 AND (?6 IS NULL OR at <= ?6)",
                params![subject_kind, subject_id, kind, repo, from, until],
                |row| row.get(0),
            )
            .map_err(reading)
    }

    /// The newest pull requests a Session held that merged in one repository, newest first.
    pub fn landed_pulls(&self, manifest: &str, limit: u32) -> Result<Vec<LandedPull>, WriteError> {
        let mut read = self
            .conn
            .prepare(
                "SELECT a.holder_id, t.subject_id, t.at
                 FROM timeline_events t
                 JOIN ledger_attachments a
                   ON a.kind = 'pr' AND a.holder_kind = 'session' AND a.target = t.subject_id AND a.manifest_id = t.repo
                 WHERE t.kind = 'pr.merged' AND t.subject_kind = 'pull_request' AND t.repo = ?1
                 ORDER BY t.at DESC, t.seq DESC
                 LIMIT ?2",
            )
            .map_err(reading)?;
        let rows = read
            .query_map(params![manifest, i64::from(limit)], |row| {
                Ok(LandedPull { session: row.get(0)?, number: row.get(1)?, merged_at: row.get(2)? })
            })
            .map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }

    /// When the latest pull request this Session held that merged before `before` did. Absent if none.
    pub fn previous_landing(&self, session: &str, manifest: &str, before: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row(
                "SELECT MAX(t.at)
                 FROM timeline_events t
                 JOIN ledger_attachments a
                   ON a.kind = 'pr' AND a.holder_kind = 'session' AND a.target = t.subject_id AND a.manifest_id = t.repo
                 WHERE t.kind = 'pr.merged' AND t.subject_kind = 'pull_request' AND t.repo = ?2
                   AND a.holder_id = ?1 AND t.at < ?3",
                params![session, manifest, before],
                |row| row.get(0),
            )
            .map_err(reading)
    }
}
