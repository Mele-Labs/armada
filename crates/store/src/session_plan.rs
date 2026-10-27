//! A Session's plan and the files it touched, kept. `docs/concepts/session.md`, *The plan*.
//!
//! **Strings, not the wire's types**: this crate does not know `ipc`. A task's `listed` is what the
//! agent's own task list last said of it (`pending`, `in_progress` or `completed`), and the state
//! the wire shows is derived from it on read. The plan is one version: a recording replaces the
//! groups and tasks whole.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::session_focus::refocus;

/// One task of the plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPlanTask {
    pub id: String,
    pub text: String,
    /// What the agent's task list last said of it. Absent while no item of the list matched.
    pub listed: Option<String>,
    /// When Fleet first saw it completed.
    pub completed_at: Option<String>,
    /// An edit was seen under the group's paths while it was worked.
    pub edited: bool,
    /// When the agent's list last put it in progress. Absent while it is not, and for one that was before
    /// this was kept.
    pub started_at: Option<String>,
    /// When its mark last moved or Fleet last saw work under its group's paths, and the plan's `calls` then.
    pub touched_at: Option<String>,
    pub touched_call: u32,
    /// The state Fleet set from proof, `in_progress` or `completed`, which stands over `listed` until the
    /// agent's list changes the task; and that proof, as a person reads it.
    pub marked: Option<String>,
    pub marked_why: Option<String>,
    /// The agent was told of this task, which it is once.
    pub told: bool,
}

/// One group, with its tasks in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPlanGroup {
    pub id: String,
    /// One line of what the group gives the person, as the agent wrote it.
    pub gives: Option<String>,
    pub agent: String,
    pub title: String,
    pub paths: Vec<String>,
    /// The issue the group works: digits for the forge's number, or a tracker's key.
    pub issue: Option<String>,
    /// The last edit seen under one of the paths.
    pub worked_at: Option<String>,
    pub tasks: Vec<KeptPlanTask>,
}

/// One difference from the version before. `kind` is `added`, `renamed`, `moved` or `removed`:
/// `other` is empty, the old text, the group it came from, or the text of what went, and for a
/// removed task `id` is its group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPlanChange {
    pub kind: String,
    pub id: String,
    pub other: String,
}

/// The plan as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPlan {
    pub version: u32,
    /// A few plain lines of what the person gets, as the agent wrote them.
    pub summary: String,
    pub recorded_at: String,
    pub approved_at: Option<String>,
    /// When the person put this version away to keep talking. A new version starts without it.
    pub set_aside_at: Option<String>,
    /// The ledger target and the title of the sketch drawn with it.
    pub sketch: Option<(String, String)>,
    /// The page the plan depends on: its call waits until the person approves that walk.
    pub walk: Option<String>,
    /// What this version changed against the one before. Absent for the first.
    pub changes: Option<Vec<KeptPlanChange>>,
    /// The next number a group and a task are given, so an id is never given twice.
    pub next_group: u32,
    pub next_task: u32,
    /// How many tool calls the Session's own agent has made since the plan was first recorded.
    pub calls: u32,
    pub groups: Vec<KeptPlanGroup>,
}

/// One file touch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptTouch {
    /// The subagent's type and the call that started it. Absent for the Session's own.
    pub by: Option<(String, String)>,
    pub path: String,
    pub act: String,
    pub at: String,
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a job's plan")(why))
}

fn keeping(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("keeping a job's plan")(why))
}

impl Store {
    /// The plan as it stands, absent until the agent records one.
    pub fn session_plan(&self, session_id: &str) -> Result<Option<KeptPlan>, WriteError> {
        let head = self
            .conn
            .query_row(
                "SELECT version, recorded_at, approved_at, sketch_id, sketch_title, changed, next_group, next_task, walk, summary, set_aside_at, calls
                 FROM session_plan WHERE session_id = ?1",
                [session_id],
                |row| {
                    let sketch: Option<String> = row.get(3)?;
                    let title: Option<String> = row.get(4)?;
                    Ok(KeptPlan {
                        version: row.get(0)?,
                        recorded_at: row.get(1)?,
                        approved_at: row.get(2)?,
                        sketch: sketch.zip(title),
                        changes: (row.get::<_, i64>(5)? == 1).then(Vec::new),
                        next_group: row.get(6)?,
                        next_task: row.get(7)?,
                        walk: row.get(8)?,
                        summary: row.get(9)?,
                        set_aside_at: row.get(10)?,
                        calls: row.get(11)?,
                        groups: Vec::new(),
                    })
                },
            )
            .optional()
            .map_err(reading)?;
        let Some(mut plan) = head else { return Ok(None) };
        let mut groups = self
            .conn
            .prepare(
                "SELECT group_id, agent, title, paths, worked_at, issue, issue_reference, gives FROM session_plan_groups
                 WHERE session_id = ?1 ORDER BY position",
            )
            .map_err(reading)?;
        plan.groups = groups
            .query_map([session_id], |row| {
                let paths: String = row.get(3)?;
                Ok(KeptPlanGroup {
                    id: row.get(0)?,
                    gives: row.get(7)?,
                    agent: row.get(1)?,
                    title: row.get(2)?,
                    paths: paths.lines().map(str::to_string).collect(),
                    worked_at: row.get(4)?,
                    // A group kept before keys has its number in `issue` alone.
                    issue: row.get::<_, Option<String>>(6)?.or(row.get::<_, Option<i64>>(5)?.map(|number| number.to_string())),
                    tasks: Vec::new(),
                })
            })
            .map_err(reading)?
            .collect::<Result<_, _>>()
            .map_err(reading)?;
        let mut tasks = self
            .conn
            .prepare(
                "SELECT group_id, task_id, text, listed, completed_at, edited, started_at, touched_at, touched_call, marked,
                        marked_why, told
                 FROM session_plan_tasks WHERE session_id = ?1 ORDER BY position",
            )
            .map_err(reading)?;
        let rows = tasks
            .query_map([session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    KeptPlanTask {
                        id: row.get(1)?,
                        text: row.get(2)?,
                        listed: row.get(3)?,
                        completed_at: row.get(4)?,
                        edited: row.get::<_, i64>(5)? == 1,
                        started_at: row.get(6)?,
                        touched_at: row.get(7)?,
                        touched_call: row.get(8)?,
                        marked: row.get(9)?,
                        marked_why: row.get(10)?,
                        told: row.get::<_, i64>(11)? == 1,
                    },
                ))
            })
            .map_err(reading)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(reading)?;
        for (group, task) in rows {
            if let Some(held) = plan.groups.iter_mut().find(|one| one.id == group) {
                held.tasks.push(task);
            }
        }
        if let Some(changes) = plan.changes.as_mut() {
            let mut read = self
                .conn
                .prepare("SELECT kind, ref_id, other FROM session_plan_changes WHERE session_id = ?1 ORDER BY position")
                .map_err(reading)?;
            *changes = read
                .query_map([session_id], |row| {
                    Ok(KeptPlanChange { kind: row.get(0)?, id: row.get(1)?, other: row.get(2)? })
                })
                .map_err(reading)?
                .collect::<Result<_, _>>()
                .map_err(reading)?;
        }
        Ok(Some(plan))
    }

    /// Keep the plan whole, replacing the one held, in one transaction.
    pub fn keep_session_plan(&mut self, session_id: &str, plan: &KeptPlan) -> Result<(), WriteError> {
        let kept = self.conn.transaction().map_err(keeping)?;
        for statement in [
            "DELETE FROM session_plan WHERE session_id = ?1",
            "DELETE FROM session_plan_groups WHERE session_id = ?1",
            "DELETE FROM session_plan_tasks WHERE session_id = ?1",
            "DELETE FROM session_plan_changes WHERE session_id = ?1",
        ] {
            kept.execute(statement, [session_id]).map_err(keeping)?;
        }
        kept.execute(
            "INSERT INTO session_plan
                 (session_id, version, recorded_at, approved_at, sketch_id, sketch_title, changed, next_group, next_task, walk, summary, set_aside_at, calls)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                session_id,
                plan.version,
                plan.recorded_at,
                plan.approved_at,
                plan.sketch.as_ref().map(|(id, _)| id),
                plan.sketch.as_ref().map(|(_, title)| title),
                plan.changes.is_some(),
                plan.next_group,
                plan.next_task,
                plan.walk,
                plan.summary,
                plan.set_aside_at,
                plan.calls,
            ],
        )
        .map_err(keeping)?;
        for (at, group) in plan.groups.iter().enumerate() {
            kept.execute(
                "INSERT INTO session_plan_groups
                     (session_id, position, group_id, agent, title, paths, worked_at, issue, issue_reference, gives)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    session_id,
                    at as i64,
                    group.id,
                    group.agent,
                    group.title,
                    group.paths.join("\n"),
                    group.worked_at,
                    // A number in the old column too, for a build from before keys that opens this file.
                    group.issue.as_deref().and_then(|issue| issue.parse::<i64>().ok()),
                    group.issue,
                    group.gives,
                ],
            )
            .map_err(keeping)?;
            for (nth, task) in group.tasks.iter().enumerate() {
                kept.execute(
                    "INSERT INTO session_plan_tasks
                         (session_id, group_id, position, task_id, text, listed, completed_at, edited, started_at,
                          touched_at, touched_call, marked, marked_why, told)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        session_id,
                        group.id,
                        nth as i64,
                        task.id,
                        task.text,
                        task.listed,
                        task.completed_at,
                        task.edited,
                        task.started_at,
                        task.touched_at,
                        task.touched_call,
                        task.marked,
                        task.marked_why,
                        task.told
                    ],
                )
                .map_err(keeping)?;
            }
        }
        for (at, change) in plan.changes.iter().flatten().enumerate() {
            kept.execute(
                "INSERT INTO session_plan_changes (session_id, position, kind, ref_id, other) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session_id, at as i64, change.kind, change.id, change.other],
            )
            .map_err(keeping)?;
        }
        refocus(&kept, session_id).map_err(keeping)?;
        kept.commit().map_err(keeping)
    }

    /// Approve `version`, if it is the one held and not yet approved. Whether it was.
    pub fn approve_session_plan(&mut self, session_id: &str, version: u32, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE session_plan SET approved_at = ?3
                 WHERE session_id = ?1 AND version = ?2 AND approved_at IS NULL",
                params![session_id, version, at],
            )
            .map(|changed| changed == 1)
            .map_err(keeping)
    }

    /// Put `version` away, if it is the one held and not already put away. Whether it was.
    pub fn set_aside_session_plan(&mut self, session_id: &str, version: u32, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE session_plan SET set_aside_at = ?3
                 WHERE session_id = ?1 AND version = ?2 AND set_aside_at IS NULL",
                params![session_id, version, at],
            )
            .map(|changed| changed == 1)
            .map_err(keeping)
    }

    /// Keep the progress `plan` holds, a group's last edit and a task's list state, leaving the head,
    /// the shape and the changes as they are.
    pub fn keep_session_plan_progress(&mut self, session_id: &str, plan: &KeptPlan) -> Result<(), WriteError> {
        let kept = self.conn.transaction().map_err(keeping)?;
        for group in &plan.groups {
            kept.execute(
                "UPDATE session_plan_groups SET worked_at = ?3 WHERE session_id = ?1 AND group_id = ?2",
                params![session_id, group.id, group.worked_at],
            )
            .map_err(keeping)?;
            for task in &group.tasks {
                kept.execute(
                    "UPDATE session_plan_tasks SET listed = ?3, completed_at = ?4, edited = ?5, started_at = ?6,
                         touched_at = ?7, touched_call = ?8, marked = ?9, marked_why = ?10, told = ?11
                     WHERE session_id = ?1 AND task_id = ?2",
                    params![
                        session_id,
                        task.id,
                        task.listed,
                        task.completed_at,
                        task.edited,
                        task.started_at,
                        task.touched_at,
                        task.touched_call,
                        task.marked,
                        task.marked_why,
                        task.told
                    ],
                )
                .map_err(keeping)?;
            }
        }
        refocus(&kept, session_id).map_err(keeping)?;
        kept.commit().map_err(keeping)
    }

    /// `calls` more tool calls by the Session's own agent. The count after them, or absent where the
    /// Session has no plan, which counts nothing.
    pub fn count_session_calls(&mut self, session_id: &str, calls: u32) -> Result<Option<u32>, WriteError> {
        self.conn
            .query_row(
                "UPDATE session_plan SET calls = calls + ?2 WHERE session_id = ?1 RETURNING calls",
                params![session_id, calls],
                |row| row.get(0),
            )
            .optional()
            .map_err(keeping)
    }

    /// Keep one file touch, and let go of the Session's oldest beyond `most`. **The same touch as the
    /// latest one, by the same hand, only moves its instant**: a file edited thirty times in a row is
    /// one touch. Whether a new one was kept.
    pub fn keep_session_touch(&mut self, session_id: &str, touch: &KeptTouch, most: usize) -> Result<bool, WriteError> {
        let (agent, agent_id) = (touch.by.as_ref().map(|(agent, _)| agent), touch.by.as_ref().map(|(_, id)| id));
        let kept = self.conn.transaction().map_err(keeping)?;
        let again = kept
            .execute(
                "UPDATE session_touches SET at = ?6
                 WHERE seq = (SELECT MAX(seq) FROM session_touches WHERE session_id = ?1)
                   AND agent IS ?2 AND agent_id IS ?3 AND path = ?4 AND act = ?5",
                params![session_id, agent, agent_id, touch.path, touch.act, touch.at],
            )
            .map_err(keeping)?;
        if again == 0 {
            kept.execute(
                "INSERT INTO session_touches (session_id, agent, agent_id, path, act, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![session_id, agent, agent_id, touch.path, touch.act, touch.at],
            )
            .map_err(keeping)?;
            kept.execute(
                "DELETE FROM session_touches WHERE session_id = ?1 AND seq <= (
                     SELECT seq FROM session_touches WHERE session_id = ?1 ORDER BY seq DESC LIMIT 1 OFFSET ?2)",
                params![session_id, most as i64],
            )
            .map_err(keeping)?;
        }
        kept.commit().map_err(keeping)?;
        Ok(again == 0)
    }

    /// The Session's recent touches, oldest first.
    pub fn session_touches(&self, session_id: &str) -> Result<Vec<KeptTouch>, WriteError> {
        let mut read = self
            .conn
            .prepare("SELECT agent, agent_id, path, act, at FROM session_touches WHERE session_id = ?1 ORDER BY seq")
            .map_err(reading)?;
        let touches = read
            .query_map([session_id], |row| {
                let agent: Option<String> = row.get(0)?;
                let id: Option<String> = row.get(1)?;
                Ok(KeptTouch { by: agent.zip(id), path: row.get(2)?, act: row.get(3)?, at: row.get(4)? })
            })
            .map_err(reading)?
            .collect::<Result<_, _>>()
            .map_err(reading);
        touches
    }
}
