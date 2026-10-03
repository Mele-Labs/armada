//! How hard a task is, which model each difficulty runs on, and what a person
//! changes on one task. Spike 022, slice 3.
//!
//! **The planner picks a tier and the Job's map picks the model** (#1530,
//! 22 Sep); **a person may pick a task's model directly, over the map** (the
//! owner, 30 Sep 2026). The order every spawn resolves in is
//! [`Job::model_spawned_for`]'s, spelled once.
//!
//! [`Job::model_spawned_for`]: crate::Job::model_spawned_for

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::job::ids::{ModelName, RepoPath};

/// How hard the planner thought a task was. **A task with none is the
/// planner leaving it to Armada**, which is a tier the map has no key for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskTier {
    Difficult,
    Medium,
    Easy,
}

impl TaskTier {
    pub const ALL: &'static [TaskTier] = &[TaskTier::Difficult, TaskTier::Medium, TaskTier::Easy];

    pub fn as_wire(&self) -> &'static str {
        match self {
            TaskTier::Difficult => "difficult",
            TaskTier::Medium => "medium",
            TaskTier::Easy => "easy",
        }
    }

    pub fn from_wire(value: &str) -> Option<TaskTier> {
        TaskTier::ALL
            .iter()
            .copied()
            .find(|tier| tier.as_wire() == value)
    }
}

/// Which model each tier runs on, for one Job.
///
/// **A tier left out is Armada picking** (the owner's answer 8, 1 Oct 2026):
/// the spawn falls through to the step's model and then the Job's, and the
/// Drone's row says which it ran. There is no value meaning "Auto", so a map
/// cannot say it two ways.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TierModels(BTreeMap<TaskTier, ModelName>);

impl TierModels {
    /// The same map with `tier` running on `model`.
    pub fn with(mut self, tier: TaskTier, model: ModelName) -> TierModels {
        self.0.insert(tier, model);
        self
    }

    /// The model `tier` runs on, or `None` where Armada picks.
    pub fn get(&self, tier: TaskTier) -> Option<&ModelName> {
        self.0.get(&tier)
    }

    /// Every tier the map names, in [`TaskTier::ALL`]'s order.
    pub fn named(&self) -> impl Iterator<Item = (TaskTier, &ModelName)> {
        self.0.iter().map(|(tier, model)| (*tier, model))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// What a person changed on one task: only the fields they changed (#1657).
///
/// **Never empty, and never a blank title**, so a kept edit always changed
/// something a person can read. `note` and `expects` may be set to nothing,
/// and `scope` to no paths: clearing one is a change. **No tier**: a person
/// picks the model directly, over the map (30 Sep 2026).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskEdit {
    title: Option<String>,
    note: Option<String>,
    scope: Option<Vec<RepoPath>>,
    expects: Option<String>,
    model: Option<ModelName>,
}

impl TaskEdit {
    /// `None` where nothing is changed or the title would be blank.
    pub fn new(
        title: Option<&str>,
        note: Option<&str>,
        scope: Option<&[&str]>,
        expects: Option<&str>,
        model: Option<ModelName>,
    ) -> Option<TaskEdit> {
        let title = match title.map(str::trim) {
            Some("") => return None,
            other => other.map(String::from),
        };
        let edit = TaskEdit {
            title,
            note: note.map(|note| String::from(note.trim())),
            scope: scope.map(|paths| {
                paths
                    .iter()
                    .map(|path| path.trim())
                    .filter(|path| !path.is_empty())
                    .map(RepoPath::new)
                    .collect()
            }),
            expects: expects.map(|expects| String::from(expects.trim())),
            model,
        };
        let changes = edit.title.is_some()
            || edit.note.is_some()
            || edit.scope.is_some()
            || edit.expects.is_some()
            || edit.model.is_some();
        changes.then_some(edit)
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn scope(&self) -> Option<&[RepoPath]> {
        self.scope.as_deref()
    }

    pub fn expects(&self) -> Option<&str> {
        self.expects.as_deref()
    }

    /// A person's pick, which beats the Job's map for this task.
    pub fn model(&self) -> Option<&ModelName> {
        self.model.as_ref()
    }
}
