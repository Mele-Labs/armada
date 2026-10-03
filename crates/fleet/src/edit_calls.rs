//! Which task wrote which file, read off its Drone's edit calls. Spike 022,
//! slice 5, answers 6 and 10; `docs/concepts/plan.md`, *Tasks that may run at once*.
//!
//! Each task Drone's edit calls are kept at its first hand-in. At a group's
//! join, two tasks whose Drones ran at the same time and named one path are an
//! overlap: the group does not reach its gate, and the two run again one after
//! the other. **A shell write names no path and goes unseen**, by decision.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::{AgentHarness, Delivery, DroneEvent, Vcs, WorkProduct};
use core_model::{
    Actor, Apart, Component, DroneId, Envelope, FieldValue, GroupId, GroupMove, GroupRuns, JobId,
    Level, PlanChange, StepId, TaskId, TaskState, TaskUpdate, Timestamp, WorkPlan,
};
use store::PlanHand;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::grouping::current_group;
use crate::work_plan::plan_not_kept;
use crate::working::Working;

/// One task's Drone, the stretch it worked before handing in, and the files
/// its edit calls named in that stretch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edited {
    pub task: TaskId,
    pub drone: DroneId,
    /// Its spawn.
    pub from: Timestamp,
    /// Its first hand-in, when its edit calls were kept.
    pub to: Timestamp,
    /// Repository-relative.
    pub paths: BTreeSet<String>,
}

/// Two tasks of one group whose Drones ran at once and named one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overlap {
    /// The lower id first.
    pub tasks: (TaskId, TaskId),
    pub paths: Vec<String>,
}

/// Every pair of `group`'s tasks that ran at once and named one file, each
/// read off **its latest Drone**, so a pair run again is not read twice.
pub fn overlapping(plan: &WorkPlan, group: GroupId, edited: &[Edited]) -> Vec<Overlap> {
    let in_group: Vec<Edited> = edited
        .iter()
        .filter(|one| plan.task(one.task).is_some_and(|t| t.group() == group))
        .cloned()
        .collect();
    let read: Vec<&Edited> = latest(&in_group).into_values().collect();
    let mut found = Vec::new();
    for (n, one) in read.iter().enumerate() {
        for other in &read[n + 1..] {
            let at_once = before(&one.from, &other.to) && before(&other.from, &one.to);
            let paths: Vec<String> = one.paths.intersection(&other.paths).cloned().collect();
            if at_once && !paths.is_empty() {
                found.push(Overlap {
                    tasks: (one.task.min(other.task), one.task.max(other.task)),
                    paths,
                });
            }
        }
    }
    found
}

/// What Fleet appends for overlaps: each task named back to `open`, and each
/// pair run apart for the rest of the plan.
pub fn run_apart(
    group: GroupId,
    overlaps: &[Overlap],
    at: Timestamp,
) -> (Vec<PlanChange>, Vec<GroupMove>) {
    let tasks: BTreeSet<TaskId> = overlaps
        .iter()
        .flat_map(|o| [o.tasks.0, o.tasks.1])
        .collect();
    let reopened = tasks
        .into_iter()
        .map(|task| PlanChange::Updated {
            task,
            to: TaskUpdate::Open,
            shown: None,
        })
        .collect();
    let apart = overlaps
        .iter()
        .map(|o| {
            GroupMove::Apart(Apart {
                group,
                tasks: o.tasks,
                paths: o.paths.clone(),
                at: at.clone(),
            })
        })
        .collect();
    (reopened, apart)
}

/// Every done task a later group's task came back to: their latest Drones'
/// edit calls named one file. Not a fault; the wire's `touched_after_done`.
pub fn touched_after_done(plan: &WorkPlan, edited: &[Edited]) -> BTreeSet<TaskId> {
    let order = |task: TaskId| {
        plan.task(task)
            .and_then(|t| plan.groups().iter().position(|g| *g == t.group()))
    };
    let latest = latest(edited);
    let mut touched = BTreeSet::new();
    for (task, done) in &latest {
        if plan.task(*task).map(|t| t.state()) != Some(TaskState::Done) {
            continue;
        }
        let Some(at) = order(*task) else {
            continue;
        };
        let later = latest.values().any(|other| {
            order(other.task).is_some_and(|o| o > at)
                && other.paths.intersection(&done.paths).next().is_some()
        });
        if later {
            touched.insert(*task);
        }
    }
    touched
}

/// Each task's latest Drone, by spawn.
fn latest(edited: &[Edited]) -> BTreeMap<TaskId, &Edited> {
    let mut latest: BTreeMap<TaskId, &Edited> = BTreeMap::new();
    for one in edited {
        match latest.get(&one.task) {
            Some(kept) if !before(&kept.from, &one.from) => {}
            _ => {
                latest.insert(one.task, one);
            }
        }
    }
    latest
}

/// A path as the repository names it. The harness may spell it absolute or
/// from the home directory.
pub(crate) fn repository_relative(path: &str, worktree: &str, home: &str) -> String {
    let whole = match path.strip_prefix("~/") {
        Some(rest) => format!("{}/{rest}", home.trim_end_matches('/')),
        None => path.to_string(),
    };
    let root = format!("{}/", worktree.trim_end_matches('/'));
    match whole.strip_prefix(&root) {
        Some(inside) => inside.to_string(),
        None => whole,
    }
}

/// Every file these events' edit calls named, repository-relative.
pub(crate) fn named_by<H: AgentHarness>(
    harness: &H,
    events: &[DroneEvent],
    worktree: &str,
    home: &str,
) -> Vec<String> {
    let named: BTreeSet<String> = events
        .iter()
        .filter_map(|event| harness.edited(event))
        .map(|path| repository_relative(&path, worktree, home))
        .collect();
    named.into_iter().collect()
}

/// Whether `a` is strictly before `b`, by the text where the clock will not
/// read: RFC 3339 as Fleet writes it orders as the instant does.
fn before(a: &Timestamp, b: &Timestamp) -> bool {
    match (a.epoch_millis(), b.epoch_millis()) {
        (Some(a), Some(b)) => a < b,
        _ => a.as_str() < b.as_str(),
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Keep the files this task Drone's edit calls named so far, at its hand-in.
    pub(crate) async fn kept_edits(&self, at_work: &Working, at: &Timestamp) -> Result<(), Adrift> {
        let (job, _, drone) = at_work.drone();
        let worktree = at_work.standing().2;
        let paths = named_by(
            self.harness().as_ref(),
            &at_work.heard(),
            worktree.path(),
            &self.host().home,
        );
        self.store()
            .lock()
            .await
            .keep_task_edits(&job, &drone, &paths, at)
            .map_err(Adrift::Writing)?;
        Ok(())
    }

    /// At a group's join: where two of its tasks ran at once and named one
    /// file, run them apart and answer `true`, so the group does not reach its
    /// gate. Answer 6.
    pub(crate) async fn ran_apart(
        &self,
        job: &JobId,
        step: &StepId,
        plan: &WorkPlan,
        runs: &GroupRuns,
        at: &Timestamp,
    ) -> Result<bool, Adrift> {
        let Some(group) = current_group(plan, runs) else {
            return Ok(false);
        };
        let overlaps = overlapping(plan, group, &self.edits_of(job).await?);
        if overlaps.is_empty() {
            return Ok(false);
        }
        let (reopened, apart) = run_apart(group, &overlaps, at.clone());
        let mut store = self.store().lock().await;
        for moved in &apart {
            store
                .record_group_move(job, moved, None)
                .map_err(Adrift::Writing)?;
        }
        let mut moved = Vec::new();
        for change in &reopened {
            let plan = store
                .change_plan(job, change, PlanHand::Step(step), at)
                .map_err(|why| plan_not_kept(job, why))?;
            if let PlanChange::Updated { task, .. } = change {
                moved.push((plan, *task));
            }
        }
        drop(store);
        for (plan, task) in moved {
            self.publish(ipc::Event::JobPlanChanged(ipc::JobPlanChanged::task_moved(
                job,
                &plan,
                task,
                Actor::Fleet,
                at,
            )));
        }
        for overlap in &overlaps {
            self.noted_apart(job, group, overlap);
        }
        Ok(true)
    }

    fn noted_apart(&self, job: &JobId, group: GroupId, overlap: &Overlap) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "two tasks running at once edited one file, so the group did not commit and \
             they run again one after the other",
        )
        .in_job(job.as_ulid().clone())
        .with_field("group", FieldValue::Str(group.to_string()))
        .with_field(
            "tasks",
            FieldValue::Str(format!("{} {}", overlap.tasks.0, overlap.tasks.1)),
        )
        .with_field("paths", FieldValue::Str(overlap.paths.join(" ")));
        self.noted_in_the_log(job, &envelope);
    }

    /// The plan as the wire carries it, each done task a later group's task
    /// came back to marked. A reading that will not load marks nothing.
    pub(crate) async fn served_plan(
        &self,
        job: &JobId,
        plan: &WorkPlan,
        runs: &GroupRuns,
    ) -> ipc::WorkPlan {
        let mut served = ipc::WorkPlan::of(plan, runs);
        let touched = match self.edits_of(job).await {
            Ok(edits) => touched_after_done(plan, &edits),
            Err(_) => BTreeSet::new(),
        };
        for task in &mut served.tasks {
            task.touched_after_done = touched.iter().any(|t| t.to_string() == task.id);
        }
        served
    }

    /// Every task Drone's kept edit calls on this Job.
    pub(crate) async fn edits_of(&self, job: &JobId) -> Result<Vec<Edited>, Adrift> {
        let store = self.store().lock().await;
        let bound = store.task_drones(job).map_err(Adrift::Reading)?;
        let edits = store.task_edits(job).map_err(Adrift::Reading)?;
        Ok(bound
            .into_iter()
            .filter_map(|drone| {
                let to = drone.edits_at?;
                Some(Edited {
                    task: drone.task,
                    paths: edits
                        .iter()
                        .filter(|edit| edit.drone_id == drone.drone_id)
                        .map(|edit| edit.path.clone())
                        .collect(),
                    drone: drone.drone_id,
                    from: drone.spawned_at,
                    to,
                })
            })
            .collect())
    }
}
