//! Which of the four places a Job's workflow was read from. #425.
//!
//! **On the record because a person asks it of a Job**, not of a directory: a
//! workflow from somebody's Kit explains a Job that behaves differently for one
//! person than another, and the files may have changed since it was frozen.

use core::fmt;

/// Which of the four places a workflow was read from.
///
/// **Declared in the order they lose, and the order is the rule**: `Ord` is
/// what `config::Catalogue` compares, so a fifth source is placed by where its
/// variant is written rather than by a precedence table beside it. **Yours
/// first, then narrower**: Kit's beats a teammate's commit, and a copy kept for
/// one project beats Kit's every-project one (`docs/concepts/workflow.md`,
/// *Where a definition comes from*). Which project a `Project` file belongs to is
/// not here; it is `config::Written::project`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkflowSource {
    /// Compiled into the binary. What a repository with no workflows of its
    /// own runs on.
    Armada,
    /// The repository's own `.armada/workflows/`.
    Repository,
    /// Kit's Workflows, which a person keeps for every repository they work in.
    Kit,
    /// A copy a person keeps for one project, in Kit's folder. Beats Kit's
    /// every-project copy.
    Project,
}

impl WorkflowSource {
    pub const ALL: &'static [WorkflowSource] = &[
        WorkflowSource::Armada,
        WorkflowSource::Repository,
        WorkflowSource::Kit,
        WorkflowSource::Project,
    ];

    pub fn as_wire(self) -> &'static str {
        match self {
            WorkflowSource::Armada => "armada",
            WorkflowSource::Kit => "kit",
            WorkflowSource::Repository => "repository",
            WorkflowSource::Project => "project",
        }
    }

    /// Read a stored value back. `None` where it is not one of the set.
    pub fn from_wire(value: &str) -> Option<WorkflowSource> {
        WorkflowSource::ALL
            .iter()
            .copied()
            .find(|source| source.as_wire() == value)
    }
}

impl fmt::Display for WorkflowSource {
    /// Where it came from, as a person would say it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            WorkflowSource::Armada => "carried by Armada",
            WorkflowSource::Kit => "from Kit",
            WorkflowSource::Repository => "from the repository",
            WorkflowSource::Project => "from a project",
        })
    }
}
