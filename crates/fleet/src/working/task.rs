//! The task a Drone was put on, and what crosses from one task's Drone to the
//! next. Spike 022, slice 1b; `crate::tasking` is the Fleet half.
//!
//! **A Drone is bound to its task in the slot, at the spawn**, so a hand-in
//! names no task and cannot name the wrong one. The binding is also on the
//! record (`store::TaskDrone`), which is what a Fleet that adopts the Drone
//! after a restart reads it back from.

use std::time::Duration;

use adapter_traits::Footprint;
use core_model::{DeclaredPaths, TaskId, Timestamp};

use crate::converging::elapsed;
use crate::working::Working;

/// The task, and when its Drone handed it in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OnTask {
    task: TaskId,
    handed_in: Option<Timestamp>,
}

/// What one task's Drone leaves the next on the same step: the baseline the
/// step entered with, and every path declared so far.
///
/// **The baseline is the step's, never a task's.** `diff_nonempty` and the
/// scope check measure the step's work, and a baseline read at each task's
/// spawn would credit the step with the last task alone.
pub(crate) struct Carried {
    entered_with: Option<Footprint>,
    declared: Option<DeclaredPaths>,
}

impl Working {
    /// Bind this Drone to the task it was spawned for.
    pub(crate) fn on_task(&mut self, task: TaskId) {
        self.task = Some(OnTask {
            task,
            handed_in: None,
        });
    }

    /// The task this Drone was put on, or `None` on a Drone working its step.
    pub(crate) fn task(&self) -> Option<TaskId> {
        self.task.as_ref().map(|on| on.task)
    }

    /// Its task is handed in, at `at`. A turn once it has come to rest ends
    /// it, where a task is left: [`settled`](Working::settled).
    pub(crate) fn task_handed_in(&mut self, at: Timestamp) {
        if let Some(on) = self.task.as_mut() {
            on.handed_in = Some(at);
        }
    }

    pub(crate) fn has_handed_in(&self) -> bool {
        self.task.as_ref().is_some_and(|on| on.handed_in.is_some())
    }

    /// Whether a Drone that handed its task in is done with its run: at rest,
    /// its pipe closed, or `grace` gone since the hand-in.
    ///
    /// **Waited for because the terminating line is the run's only figure.**
    /// The harness names turns and cost on it and nowhere else, and a Drone
    /// still writing its last message after `submit_evidence` answered is a
    /// Drone whose line has not arrived: ended there, its spend row is empty.
    /// Job 3 on 3 Oct lost three task Drones' spend that way. The grace is
    /// `StepNorms::report_grace`, the time a Drone is given to come to rest
    /// once it has been asked to report, which a hand-in is.
    pub(crate) fn settled(&self, now: &Timestamp, grace: Duration) -> bool {
        let Some(at) = self.task.as_ref().and_then(|on| on.handed_in.as_ref()) else {
            return false;
        };
        self.at_rest() || self.transcript_ended() || elapsed(at, now) >= grace
    }

    /// What the next task's Drone on this step starts from.
    pub(crate) fn carried(&self) -> Carried {
        Carried {
            entered_with: self.entered_with.clone(),
            declared: self.declared.clone(),
        }
    }

    /// Start from what the Drone before this one left. **Over the baseline
    /// `crate::dispatch::marked` just read**, which is a task's and not the
    /// step's.
    pub(crate) fn carrying(&mut self, carried: Carried) {
        if let Some(entered_with) = carried.entered_with {
            self.entered_with = Some(entered_with);
        }
        self.declared.clone_from(&carried.declared);
        self.inherited = carried.declared;
    }

    /// A declaration, widened by what earlier Drones on the step declared.
    pub(crate) fn widened(&self, paths: DeclaredPaths) -> DeclaredPaths {
        let Some(before) = &self.inherited else {
            return paths;
        };
        let mut all = before.paths().to_vec();
        for path in paths.paths() {
            if !all.contains(path) {
                all.push(path.clone());
            }
        }
        DeclaredPaths::of(all)
    }
}
