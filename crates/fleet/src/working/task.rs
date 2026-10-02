//! The task a Drone was put on, and what crosses from one task's Drone to the
//! next. Spike 022, slice 1b; `crate::tasking` is the Fleet half.
//!
//! **A Drone is bound to its task in the slot, at the spawn**, so a hand-in
//! names no task and cannot name the wrong one. The binding is also on the
//! record (`store::TaskDrone`), which is what a Fleet that adopts the Drone
//! after a restart reads it back from.

use adapter_traits::Footprint;
use core_model::{DeclaredPaths, TaskId};

use crate::working::Working;

/// The task, and whether its Drone has handed it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OnTask {
    task: TaskId,
    handed_in: bool,
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
            handed_in: false,
        });
    }

    /// The task this Drone was put on, or `None` on a Drone working its step.
    pub(crate) fn task(&self) -> Option<TaskId> {
        self.task.map(|on| on.task)
    }

    /// Its task is handed in. The next turn ends it, where a task is left.
    pub(crate) fn task_handed_in(&mut self) {
        if let Some(on) = self.task.as_mut() {
            on.handed_in = true;
        }
    }

    pub(crate) fn has_handed_in(&self) -> bool {
        self.task.is_some_and(|on| on.handed_in)
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
