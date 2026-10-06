//! Which manifests gate a Job, as `Job::gate_manifests` records them.
//!
//! **A repository with no workspaces records none.** The root gates every Job
//! there and always has, so an empty list means "the root alone", and a Job
//! written before workspaces gated reads the same. Where the repository has
//! workspaces the list names every manifest that gates, the root included.
//!
//! **An empty set of paths gates nothing**, decided 6 Oct 2026: a Job that
//! changed no files runs no Checks. `config::gating` answers that; nothing here
//! hands it to `Covers::reach`, which reads no paths as always.
//! `docs/concepts/manifest.md`, *Workspace gating*.

use config::Manifest;
use core_model::{GateManifest, GateOutcome, NotRunReason};

use crate::repositories::Served;

/// The manifests the paths gate. `None` is paths not yet known, which gates
/// every manifest the repository has: a Job that may write anywhere may reach
/// any of them.
///
/// **The outcome is a placeholder**, as no Check has run. The gate writes the
/// real one.
pub(crate) fn gate_manifests(served: &Served, paths: Option<&[String]>) -> Vec<GateManifest> {
    let workspaces = served.workspaces();
    if workspaces.is_empty() {
        return Vec::new();
    }
    let root = served.manifest();
    let gating: Vec<&Manifest> = match paths {
        None => std::iter::once(root).chain(workspaces).collect(),
        Some(paths) => config::gating(root, workspaces, paths)
            .iter()
            .filter_map(|gate| {
                std::iter::once(root)
                    .chain(workspaces)
                    .find(|manifest| manifest.dir() == gate.dir)
            })
            .collect(),
    };
    gating
        .into_iter()
        .map(|manifest| GateManifest {
            manifest_id: manifest.id().clone(),
            outcome: GateOutcome::DidNotRun(NotRunReason::PathConditionUnmet),
        })
        .collect()
}
