//! What is wrong with a Job's worktree or environment, as distinct from what is
//! wrong with its work.

use std::path::Path;

use adapter_traits::Repair;

/// What an output says when a Check failed on an install that never finished.
///
/// **Phrases a tool prints about itself, not about a test.** A list rather than
/// a pattern so a new one is a line here and a case in `tests::healing`. Kept
/// short on purpose: a phrase a failing test could also print would send a
/// repair Drone after work that is the Drone's own.
const BROKEN_INSTALL: &[&str] = &["failed to install correctly"];

/// What a repair Drone is sent to put right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Finding {
    /// Paths the index holds as unmerged whose files carry no conflict marker:
    /// somebody resolved them and nothing staged the resolution, so the diff
    /// does not show it.
    UnmergedIndex { paths: Vec<String> },
    /// A Check printed that something installed in this worktree never
    /// finished installing.
    BrokenInstall { said: String },
}

/// Which finding, without its detail. What the repair budget is counted by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    Index,
    Install,
}

impl Finding {
    pub(crate) fn kind(&self) -> Kind {
        match self {
            Finding::UnmergedIndex { .. } => Kind::Index,
            Finding::BrokenInstall { .. } => Kind::Install,
        }
    }

    /// The first line of `output` naming a broken install.
    pub(crate) fn install_in(output: &str) -> Option<Finding> {
        output
            .lines()
            .find(|line| BROKEN_INSTALL.iter().any(|phrase| line.contains(phrase)))
            .map(|line| Finding::BrokenInstall {
                said: line.trim().to_string(),
            })
    }

    /// The paths among `unmerged` a Drone's resolution left unstaged: not named
    /// in `conflicted`, which a catch-up reported and a Drone is still to
    /// resolve, and holding no marker.
    pub(crate) fn index_in(
        worktree: &Path,
        unmerged: &[String],
        conflicted: &[String],
    ) -> Option<Finding> {
        let paths: Vec<String> = unmerged
            .iter()
            .filter(|path| !conflicted.contains(path))
            .filter(|path| !holds_a_marker(&worktree.join(path)))
            .cloned()
            .collect();
        (!paths.is_empty()).then_some(Finding::UnmergedIndex { paths })
    }

    /// One line for the Job's log.
    pub(crate) fn said(&self) -> String {
        match self {
            Finding::UnmergedIndex { paths } => {
                format!("the index holds {} as unmerged", paths.join(", "))
            }
            Finding::BrokenInstall { said } => format!("a Check printed: {said}"),
        }
    }

    /// The repair Drone's only turn.
    pub(crate) fn told(&self, bootstrap: &[String]) -> String {
        match self {
            Finding::UnmergedIndex { paths } => format!(
                "REPAIR THE INDEX\n\n\
                 This worktree's index holds {} as unmerged, but the files carry no conflict \
                 markers: the conflict was resolved and the resolution never staged, so the \
                 diff does not show it.\n\n\
                 Stage each with `git add`. Where a path was deleted on one side and the file \
                 is gone, `git rm` it. Do not edit any file's content, and do not commit: Fleet \
                 commits. If a file's content does not look resolved, leave that path alone and \
                 say which and why. Then stop.",
                paths
                    .iter()
                    .map(|path| format!("`{path}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Finding::BrokenInstall { said } => format!(
                "REPAIR THE INSTALL\n\n\
                 A Check failed because something installed in this worktree never finished \
                 installing. It printed: {said}\n\n\
                 This worktree was reused, so the repository's bootstrap finished at once \
                 without redoing it. Run the bootstrap again with its flag for a forced \
                 reinstall, so the install steps of every dependency run: {}. Change no source \
                 file. Then stop; Fleet runs the Check again.",
                bootstrap
                    .iter()
                    .map(|run| format!("`{run}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// The one repair this finding is granted, over the bootstrap commands the
    /// Manifest declared.
    pub(crate) fn repairs(&self, bootstrap: &[String]) -> Vec<Repair> {
        match self {
            Finding::UnmergedIndex { .. } => vec![Repair::TheIndex],
            Finding::BrokenInstall { .. } => bootstrap
                .iter()
                .map(|run| Repair::TheInstall(run.clone()))
                .collect(),
        }
    }
}

/// Whether a file still holds a conflict marker. **Unreadable counts as none**:
/// a deleted file and a binary one have no marker to be mid-resolution on.
fn holds_a_marker(file: &Path) -> bool {
    std::fs::read_to_string(file)
        .map(|text| text.lines().any(|line| line.starts_with("<<<<<<< ")))
        .unwrap_or(false)
}
