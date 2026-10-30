//! [`Lifecycles`] for Fleet: what applies to a repository, read from the places a piece is kept, and
//! what a Job holds, read from the store.
//!
//! **Read when asked, never cached**, for `piece_places`' reason. **A Job's lifecycles are those
//! `lifecycles_that_apply` hands the keeper**, so the instances, the states they stand in and the
//! definitions sent with them are the same ones; when a Job's copy is frozen that function changes
//! and this follows.

use std::collections::HashMap;
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Lifecycles, Refusal};
use core_model::{InstanceRef, Lifecycle, PieceSource, Standing};
use ipc::{
    AppliedLifecycle, AppliedRule, HeldInstance, InstanceLink, InstanceMove, JobLifecycles, ManifestId,
    RepositoryLifecycleRules, RepositoryLifecycles, RepositoryWorkspaces, RuleActReading, SessionId, WireError,
    WorkspaceOfRepositories,
};
use store::Store;

use super::holding::rebuilt;
use super::keeping::{JOB, PULL_REQUEST};
use super::wire;
use crate::daemon::Fleet;
use crate::piece_places::WORKSPACES;
use crate::session_host::NO_SUCH_SESSION;

const UNPLACED: &str = "fleet.job_lifecycles_unplaced";

/// What a person reads for an instance: the ticket's key, `PR #812`, the Job's title. **A Job with
/// no title, or none Fleet holds, is its id**, and nothing is made up in its place.
pub(super) fn label_of(store: &Store, subject: &InstanceRef) -> String {
    match subject.lifecycle().as_str() {
        PULL_REQUEST => format!("PR #{}", subject.key()),
        JOB => store
            .session(subject.key())
            .ok()
            .flatten()
            .and_then(|kept| kept.title)
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| subject.key().to_string()),
        _ => subject.key().to_string(),
    }
}

/// The rules `session` froze, as the file each was read from says them. **Where it was kept is read from the
/// path**: a file under a repository's `.armada` is the repository's, and any other is Kit's.
fn copy_rules(store: &Store, session: &str) -> Vec<AppliedRule> {
    let Some(copy) = store.job_copy_of(session).ok().flatten() else { return Vec::new() };
    copy.pieces
        .iter()
        .filter(|piece| piece.kind == store::PieceKind::Rule)
        .filter_map(|piece| {
            let rule = config::parse_rule(Path::new(&piece.path), &piece.text).ok()?;
            let source = if piece.path.contains("/.armada/") { "repository" } else { "kit" };
            let reach = match rule.scope() {
                core_model::LifecycleScope::Everywhere => "everywhere",
                core_model::LifecycleScope::Within(_) => "repository",
            };
            Some(AppliedRule {
                rule: wire::rule(&rule),
                kept: ipc::PieceKept {
                    rank: ipc::PieceRank { reach: reach.to_string(), source: source.to_string(), home: false },
                    path: Some(piece.path.clone()),
                    overridden_by: None,
                    hides: None,
                },
            })
        })
        .collect()
}

/// What `session` holds, as the wire spells it. **Empty** for a Job that holds nothing, which is every
/// Job that began before lifecycles existed, and for one in no repository. **`applying` is
/// `Fleet::lifecycles_that_apply`**, passed in because the pieces it reads are Fleet's and not the store's.
pub(crate) fn job_lifecycles_of(store: &Store, session: &str, applying: &dyn Fn(&str) -> Vec<Lifecycle>) -> JobLifecycles {
    let mut held = JobLifecycles {
        session_id: SessionId::carried(session),
        lifecycles: Vec::new(),
        instances: Vec::new(),
        moves: Vec::new(),
        acts: Vec::new(),
        rules: Vec::new(),
        changes: Vec::new(),
    };
    let pieces = store.job_copy_of(session).ok().flatten().map(|kept| kept.pieces).unwrap_or_default();
    held.changes = store.job_copy_marks(session).unwrap_or_default().iter().map(|mark| crate::session_host::job_copying::mark_wire(mark, &pieces)).collect();
    held.rules = copy_rules(store, session);
    let Some(manifest) = store.session(session).ok().flatten().and_then(|kept| kept.manifest_id) else {
        return held;
    };
    let lifecycles = applying(&manifest);
    let kept = store.lifecycle_instances_of(session).unwrap_or_default();
    let ended: HashMap<(&str, &str), &str> =
        kept.iter().filter(|one| !one.ended_at.is_empty()).map(|one| ((one.lifecycle.as_str(), one.key.as_str()), one.ended_at.as_str())).collect();
    let instances = rebuilt(&lifecycles, &kept);
    held.instances = instances
        .all()
        .iter()
        .map(|one| HeldInstance {
            lifecycle: one.subject().lifecycle().as_str().to_string(),
            key: one.subject().key().to_string(),
            label: label_of(store, one.subject()),
            ended_at: ended.get(&(one.subject().lifecycle().as_str(), one.subject().key())).map(|at| ipc::Instant::carried(*at)),
            standing: wire::standing(one.standing()),
            links: one
                .links()
                .iter()
                .map(|link| InstanceLink { lifecycle: link.lifecycle().as_str().to_string(), key: link.key().to_string() })
                .collect(),
        })
        .collect();
    held.lifecycles = lifecycles
        .iter()
        .filter(|lifecycle| held.instances.iter().any(|one| one.lifecycle == lifecycle.id().as_str()))
        .map(wire::lifecycle)
        .collect();
    held.moves = store
        .lifecycle_moves_of(session)
        .unwrap_or_default()
        .into_iter()
        .filter(|one| held.instances.iter().any(|held| held.lifecycle == one.lifecycle && held.key == one.key))
        .filter_map(|one| {
            Some(InstanceMove {
                from: wire::standing(&Standing::from_wire(&one.from)?),
                to: wire::standing(&Standing::from_wire(&one.to)?),
                lifecycle: one.lifecycle,
                key: one.key,
                source: one.seen_source,
                at: ipc::Instant::carried(one.at),
            })
        })
        .collect();
    held.acts = store
        .rule_acts_of(session)
        .unwrap_or_default()
        .into_iter()
        .map(|one| RuleActReading {
            rule: one.rule_id,
            text: one.text,
            course: one.course,
            standing: one.state,
            said: Some(one.said).filter(|said| !said.is_empty()),
            at: ipc::Instant::carried(one.raised_at),
            settled_at: Some(one.settled_at).filter(|at| !at.is_empty()).map(ipc::Instant::carried),
            on: one.on.map(|(lifecycle, key)| InstanceLink { lifecycle, key }),
            entered: one.entered.map(|(lifecycle, key)| InstanceLink { lifecycle, key }),
        })
        .collect();
    held
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
    /// Tell Bridge what `session` holds now. **A Job that holds nothing, and has changed nothing in its copy,
    /// publishes nothing.**
    pub(crate) async fn job_lifecycles_published(&self, session: &str) {
        let held = {
            let store = self.store().lock().await;
            job_lifecycles_of(&store, session, &|manifest| self.lifecycles_that_apply(manifest))
        };
        if !held.instances.is_empty() || !held.changes.is_empty() {
            self.publish(ipc::Event::JobLifecyclesChanged(held));
        }
    }

    /// The Job a call is about: the hosted one holding the connection, else the one named, which
    /// Fleet must hold.
    async fn lifecycles_job(&self, caller: Option<api::Caller>, named: Option<&SessionId>) -> Result<String, Refusal> {
        if let Some(held) = caller.and_then(|caller| self.session_holding(&caller)) {
            return Ok(held);
        }
        let Some(named) = named.map(|named| named.as_str().trim().to_string()).filter(|id| !id.is_empty()) else {
            return Err(Refusal::Unacceptable(WireError::raised(
                UNPLACED,
                "what a Job holds is its own, and nothing places one",
                self.run_id(),
            )));
        };
        match self.store().lock().await.session(&named).map_err(|why| self.ledger_fault(why))? {
            Some(_) => Ok(named),
            None => Err(Refusal::NotFound(WireError::raised(
                NO_SUCH_SESSION,
                format!("No Job Fleet holds is named {named}."),
                self.run_id(),
            ))),
        }
    }

    /// The repository, if any, whose `.armada/workspaces/` is `folder`.
    fn workspaces_kept_in(&self, folder: &Path) -> Option<ManifestId> {
        self.repositories()
            .hosted()
            .into_iter()
            .find(|one| Path::new(one.root()).join(".armada").join(WORKSPACES) == folder)
            .map(|one| ManifestId::from(one.manifest().id()))
    }
}

impl<H, V, W> Lifecycles for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn list_repository_workspaces(&self, manifest_id: Option<ManifestId>) -> Result<RepositoryWorkspaces, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let workspaces = self.workspaces_read();
        let holding: Vec<_> = workspaces.holding(served.manifest().id()).into_iter().map(|one| one.id().clone()).collect();
        let listed = workspaces
            .files()
            .iter()
            .filter(|file| holding.contains(file.workspace().id()))
            .map(|file| WorkspaceOfRepositories {
                id: file.workspace().id().as_str().to_string(),
                name: file.workspace().title().as_str().to_string(),
                repositories: file.workspace().repositories().iter().map(ManifestId::from).collect(),
                home: match file.source() {
                    PieceSource::Repository => file.path().parent().and_then(|folder| self.workspaces_kept_in(folder)),
                    _ => None,
                },
                source: wire::place(file.source()),
                path: file.path().to_string_lossy().into_owned(),
                overridden_by: file.overridden_by().map(wire::place),
                hides: file.hides().map(wire::place),
            })
            .collect();
        Ok(RepositoryWorkspaces {
            manifest_id: ManifestId::from(served.manifest().id()),
            workspaces: listed,
            left_out: wire::left_out(workspaces.left_out()),
        })
    }

    async fn list_lifecycles(&self, manifest_id: Option<ManifestId>) -> Result<RepositoryLifecycles, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let applying = self.lifecycles_applying_to(served.manifest().id())?;
        Ok(RepositoryLifecycles {
            manifest_id: ManifestId::from(served.manifest().id()),
            lifecycles: applying
                .files()
                .iter()
                .map(|file| AppliedLifecycle {
                    lifecycle: wire::lifecycle(file.lifecycle()),
                    kept: wire::kept(file.rank(), file.path(), file.overridden_by(), file.hides()),
                })
                .collect(),
            left_out: wire::left_out(applying.left_out()),
        })
    }

    async fn list_lifecycle_rules(&self, manifest_id: Option<ManifestId>) -> Result<RepositoryLifecycleRules, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let applying = self.rules_applying_to(served.manifest().id())?;
        Ok(RepositoryLifecycleRules {
            manifest_id: ManifestId::from(served.manifest().id()),
            rules: applying
                .files()
                .iter()
                .map(|file| AppliedRule {
                    rule: wire::rule(file.rule()),
                    kept: wire::kept(file.rank(), Some(file.path()), file.overridden_by(), file.hides()),
                })
                .collect(),
            left_out: wire::left_out(applying.left_out()),
        })
    }

    async fn get_job_lifecycles(
        &self,
        caller: Option<api::Caller>,
        session_id: Option<SessionId>,
    ) -> Result<JobLifecycles, Refusal> {
        let job = self.lifecycles_job(caller, session_id.as_ref()).await?;
        let store = self.store().lock().await;
        Ok(job_lifecycles_of(&store, &job, &|manifest| self.lifecycles_that_apply(manifest)))
    }

    async fn save_lifecycle_piece(&self, save: ipc::SaveLifecyclePiece) -> Result<ipc::LifecyclePieceSaved, Refusal> {
        self.lifecycle_piece_saved(save)
    }

    async fn delete_lifecycle_piece(&self, delete: ipc::DeleteLifecyclePiece) -> Result<ipc::LifecyclePieceDeleted, Refusal> {
        self.lifecycle_piece_deleted(delete)
    }
}
