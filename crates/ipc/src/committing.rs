//! `commit_to_repository`: a copy of one piece a person keeps for a repository, committed by a
//! Job of its own. `docs/concepts/kit.md`, *Commit to repository*.
//!
//! **Committing copies.** The person's own piece stays where it is kept; a new Job's branch gains
//! a copy under `.armada/` (an `armada.yml` at the root), and its pull request carries that alone.

use serde::{Deserialize, Serialize};

use crate::ids::WorkflowId;
use crate::lifecycles::PieceScope;
use crate::sessions::SessionId;

fn everywhere() -> PieceScope {
    PieceScope::Everywhere
}

/// `commit_to_repository`'s body. `POST /repositories/commit`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitToRepository {
    /// A root `list_repositories` names.
    pub repository: String,
    pub piece: PieceToCommit,
    /// Replace what the repository's base already holds under the same id. Refused without it.
    #[serde(default)]
    pub overwrite: bool,
    /// Take the piece out of the repository instead of copying it in: the file on the base branch
    /// that declares it is deleted by the same kind of Job, with a pull request. Only the
    /// Workspace, lifecycle, rule and autonomy kinds can be removed.
    #[serde(default)]
    pub remove: bool,
}

/// Which of the person's own pieces is copied, by the id its concept already uses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PieceToCommit {
    /// Tiles of the person's own for this repository, by `id` in each list, copied into
    /// `.armada/flight-controls.toml` beside the tiles the branch already holds.
    FlightControls {
        #[serde(default)]
        job: Vec<String>,
        #[serde(default)]
        deck: Vec<String>,
    },
    /// A workflow from Kit's `~/.armada/workflows/`, copied into `.armada/workflows/`.
    Workflow { workflow_id: WorkflowId },
    /// A review tool from Kit's `~/.armada/review-tools/`, by its dashed id, copied into
    /// `.armada/review-tools/`.
    ReviewTool { id: String },
    /// The person's own `armada.yml` for the repository, from the place their Kit keeps it,
    /// copied to the workspace root.
    Manifest,
    /// Requests' collections of the person's own for this repository by `id`, and environments by
    /// `name`, copied into `.armada/requests.toml` beside what the branch already holds. **A secret
    /// goes as its name and `secret = true`**, never its value.
    Requests {
        #[serde(default)]
        collections: Vec<String>,
        #[serde(default)]
        environments: Vec<String>,
    },
    /// A lifecycle from Kit's `~/.armada/lifecycles/`, found by the id and scope its file declares,
    /// copied into `.armada/lifecycles/`.
    Lifecycle {
        id: String,
        #[serde(default = "everywhere")]
        scope: PieceScope,
    },
    /// A lifecycle rule from Kit's `~/.armada/rules/`, copied into `.armada/rules/`.
    LifecycleRule {
        id: String,
        #[serde(default = "everywhere")]
        scope: PieceScope,
    },
    /// A Workspace (a set of repositories) from Kit's `~/.armada/workspaces/`, copied into
    /// `.armada/workspaces/`. Its id is its identity, so it has no scope.
    Workspace { id: String },
    /// A layer of autonomy defaults from Kit's `~/.armada/autonomy/`, copied into
    /// `.armada/autonomy/`.
    Autonomy {
        id: String,
        #[serde(default = "everywhere")]
        scope: PieceScope,
    },
}

/// `commit_to_repository`'s answer, given once the Job and its branch exist. **The commit and the
/// pull request are not in it**: they finish after it, the pull request arrives on the Job, and a
/// failure arrives as `act.failed` for that Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommittedToRepository {
    pub repository: String,
    /// The Job made for this commit alone.
    pub session_id: SessionId,
    /// The Job's own branch, which will hold the one commit.
    pub branch: String,
    /// Each file written, relative to the repository's root.
    pub paths: Vec<String>,
}
