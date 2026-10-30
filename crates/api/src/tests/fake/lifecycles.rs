//! Workspaces, lifecycles and rules, as the fake answers them: nothing is read from a file, a
//! repository holds none of any, and one Job holds a pull request in review.

use ipc::{
    AllWorkspaces, DeleteLifecyclePiece, LifecyclePiece, LifecyclePieceDeleted, LifecyclePieceKind, LifecyclePieceSaved, SaveLifecyclePiece,
    HeldInstance, InstanceLink, InstanceMove, InstanceStanding, Instant, JobLifecycles, LifecycleDefinition,
    LifecycleStateDefinition, LifecycleTransition, ManifestId, PieceScope, RepositoryAutonomy, RepositoryLifecycleRules, RepositoryLifecycles,
    RepositoryWorkspaces, SessionId, StateReadFrom,
};

use super::FakeDaemon;
use crate::{Caller, Lifecycles, Refusal};

/// The one Job the fake says holds something; any other holds nothing.
pub const THE_LIFECYCLE_JOB: &str = "job-with-a-pull-request";

/// The one piece id the fake says Kit already holds: a save of it needs `overwrite`, and a delete of
/// any other finds nothing.
pub const THE_KEPT_PIECE: &str = "kept";

fn refused(code: &str, said: String) -> Refusal {
    Refusal::Unacceptable(ipc::WireError::raised(code, said, super::run_id()))
}

fn state(id: &str, label: &str, fact: &str) -> LifecycleStateDefinition {
    LifecycleStateDefinition {
        id: id.to_string(),
        label: label.to_string(),
        read: StateReadFrom::Forge { fact: fact.to_string() },
    }
}

fn the_pull_request_lifecycle() -> LifecycleDefinition {
    LifecycleDefinition {
        id: "pr".to_string(),
        name: "Pull request".to_string(),
        scope: PieceScope::Everywhere,
        states: vec![state("ready", "Ready", "ready"), state("in_review", "In review", "in_review"), state("merged", "Merged", "merged")],
        transitions: vec![
            LifecycleTransition { from: "ready".to_string(), to: "in_review".to_string() },
            LifecycleTransition { from: "in_review".to_string(), to: "merged".to_string() },
        ],
        ends: vec!["merged".to_string()],
    }
}

fn the_job() -> JobLifecycles {
    JobLifecycles {
        session_id: SessionId::carried(THE_LIFECYCLE_JOB),
        lifecycles: vec![the_pull_request_lifecycle()],
        instances: vec![HeldInstance {
            lifecycle: "pr".to_string(),
            key: "812".to_string(),
            label: "PR #812".to_string(),
            standing: InstanceStanding::In { state: "in_review".to_string() },
            links: vec![InstanceLink { lifecycle: "ticket".to_string(), key: "ARM-41".to_string() }],
            ended_at: None,
        }],
        moves: vec![InstanceMove {
            lifecycle: "pr".to_string(),
            key: "812".to_string(),
            from: InstanceStanding::Unread,
            to: InstanceStanding::In { state: "in_review".to_string() },
            source: "forge".to_string(),
            at: Instant::carried("2026-10-30T09:00:00.000Z"),
        }],
        acts: Vec::new(),
        rules: Vec::new(),
        changes: Vec::new(),
    }
}

impl FakeDaemon {
    async fn repository(&self, named: Option<ManifestId>) -> Result<ManifestId, Refusal> {
        <FakeDaemon as crate::Queries>::scope(self, named).await
    }
}

impl Lifecycles for FakeDaemon {
    async fn list_repository_workspaces(&self, manifest_id: Option<ManifestId>) -> Result<RepositoryWorkspaces, Refusal> {
        Ok(RepositoryWorkspaces { manifest_id: self.repository(manifest_id).await?, workspaces: Vec::new(), left_out: Vec::new() })
    }

    async fn list_workspaces(&self) -> Result<AllWorkspaces, Refusal> {
        Ok(AllWorkspaces { workspaces: Vec::new(), left_out: Vec::new() })
    }

    async fn list_lifecycles(&self, manifest_id: Option<ManifestId>) -> Result<RepositoryLifecycles, Refusal> {
        Ok(RepositoryLifecycles { manifest_id: self.repository(manifest_id).await?, lifecycles: Vec::new(), left_out: Vec::new() })
    }

    async fn list_lifecycle_rules(&self, manifest_id: Option<ManifestId>) -> Result<RepositoryLifecycleRules, Refusal> {
        Ok(RepositoryLifecycleRules { manifest_id: self.repository(manifest_id).await?, rules: Vec::new(), left_out: Vec::new() })
    }

    async fn list_autonomy_defaults(&self, manifest_id: Option<ManifestId>) -> Result<RepositoryAutonomy, Refusal> {
        Ok(RepositoryAutonomy { manifest_id: self.repository(manifest_id).await?, armada: Vec::new(), layers: Vec::new(), left_out: Vec::new() })
    }

    async fn get_job_lifecycles(&self, _caller: Option<Caller>, session_id: Option<SessionId>) -> Result<JobLifecycles, Refusal> {
        let Some(job) = session_id else {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.job_lifecycles_unplaced",
                "what a Job holds is its own, and nothing places one",
                super::run_id(),
            )));
        };
        match job.as_str() == THE_LIFECYCLE_JOB {
            true => Ok(the_job()),
            false => Ok(JobLifecycles { session_id: job, lifecycles: Vec::new(), instances: Vec::new(), moves: Vec::new(), acts: Vec::new(), rules: Vec::new(), changes: Vec::new() }),
        }
    }

    /// Writes nothing: answers the piece it was given, in a file named for it, and refuses
    /// [`THE_KEPT_PIECE`] without `overwrite`, so a route test can tell the body arrived whole.
    async fn save_lifecycle_piece(&self, save: SaveLifecyclePiece) -> Result<LifecyclePieceSaved, Refusal> {
        let (kind, id, scope) = match save.piece {
            LifecyclePiece::Lifecycle { lifecycle } => (LifecyclePieceKind::Lifecycle, lifecycle.id, lifecycle.scope),
            LifecyclePiece::Rule { rule } => (LifecyclePieceKind::Rule, rule.id, rule.scope),
            LifecyclePiece::Workspace { workspace } => (LifecyclePieceKind::Workspace, workspace.id, PieceScope::Everywhere),
            LifecyclePiece::Autonomy { autonomy } => (LifecyclePieceKind::Autonomy, autonomy.id, autonomy.scope),
        };
        let kept = id == THE_KEPT_PIECE;
        if kept && !save.overwrite {
            return Err(refused("fleet.piece_exists", format!("/kit/{id}.yml already declares `{id}` and nothing was written.")));
        }
        Ok(LifecyclePieceSaved { kind, file: format!("/kit/{id}.yml"), id, scope, replaced: kept })
    }

    async fn delete_lifecycle_piece(&self, delete: DeleteLifecyclePiece) -> Result<LifecyclePieceDeleted, Refusal> {
        if delete.id != THE_KEPT_PIECE {
            return Err(refused("fleet.piece_not_kept", format!("Kit holds no `{}` to delete.", delete.id)));
        }
        Ok(LifecyclePieceDeleted { file: format!("/kit/{}.yml", delete.id), kind: delete.kind, id: delete.id, scope: delete.scope })
    }
}
