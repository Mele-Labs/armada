//! A repository's manifests read against one change: which of them gate it,
//! what each owns of it, and what each came to.
//!
//! **Only where the repository has workspaces.** One with none has no
//! [`Gated`], and a Check there runs by its own `when` as it always did. Where
//! workspaces exist the changed paths are read, never `Job::gate_manifests`
//! alone, and an empty diff gates nothing. `docs/concepts/manifest.md`,
//! *Workspace gating*.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use checks_runner::Reach;
use config::Manifest;
use core_model::{
    CheckOutcome, FrozenWorkflow, GateManifest, GateOutcome, ManifestId, NotRunReason,
    ResolvedCheck, ResolvedStep, StepCheck,
};
use verification::Observed;

/// What the manifests say about one change.
#[derive(Debug)]
pub(crate) struct Gated {
    /// Every manifest, the root first: its directory and its id.
    manifests: Vec<(String, ManifestId)>,
    /// The directories of the manifests the change gates.
    gating: Vec<String>,
    /// What each manifest owns of the change, relative to its own directory.
    owned: BTreeMap<String, Vec<String>>,
}

impl Gated {
    pub(crate) fn of(root: &Manifest, workspaces: &[Manifest], touched: &[String]) -> Gated {
        let mut owned: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for path in touched {
            let dir = config::owner(workspaces, path);
            let local = path
                .trim_start_matches('/')
                .strip_prefix(dir)
                .unwrap_or(path);
            owned
                .entry(dir.to_string())
                .or_default()
                .push(local.trim_start_matches('/').to_string());
        }
        Gated {
            manifests: std::iter::once(root)
                .chain(workspaces)
                .map(|manifest| (manifest.dir().to_string(), manifest.id().clone()))
                .collect(),
            gating: config::gating(root, workspaces, touched)
                .into_iter()
                .map(|gate| gate.dir)
                .collect(),
            owned,
        }
    }

    /// The gating manifests, out of `root` and `workspaces`, that declare Checks
    /// and that no step of `workflow` froze a Check for. **Empty where no step
    /// gates on every Check**, as there is then nothing to add them to.
    pub(crate) fn unmet_by<'m>(
        &self,
        workflow: &FrozenWorkflow,
        root: &'m Manifest,
        workspaces: &'m [Manifest],
    ) -> Vec<&'m Manifest> {
        if !workflow
            .steps()
            .iter()
            .any(ResolvedStep::gates_on_every_check)
        {
            return Vec::new();
        }
        let met = |dir: &String| {
            workflow
                .steps()
                .iter()
                .flat_map(ResolvedStep::checks)
                .any(|check| matches!(check, ResolvedCheck::ManifestCheck { manifest_dir, .. } if manifest_dir == dir))
        };
        self.gating
            .iter()
            .filter(|dir| !met(dir))
            .filter_map(|dir| {
                std::iter::once(root)
                    .chain(workspaces)
                    .find(|manifest| manifest.dir() == dir)
            })
            .filter(|manifest| !manifest.checks_as_written().is_empty())
            .collect()
    }

    /// Whether the manifest in `dir` gates the change.
    pub(crate) fn gates(&self, dir: &str) -> bool {
        self.gating.iter().any(|gating| gating == dir)
    }

    /// What `dir` owns of the change, relative to `dir`.
    pub(crate) fn owned_by(&self, dir: &str) -> &[String] {
        self.owned.get(dir).map_or(&[], Vec::as_slice)
    }

    /// The row a Check of a manifest the change does not gate records. Only a
    /// manifest's own Check: a built-in belongs to no manifest.
    pub(crate) fn declines(&self, check: &ResolvedCheck) -> Option<Observed> {
        let ResolvedCheck::ManifestCheck { manifest_dir, .. } = check else {
            return None;
        };
        if self.gates(manifest_dir) {
            return None;
        }
        Some(Observed::Skipped {
            covers: match manifest_dir.is_empty() {
                true => String::from("paths no workspace owns"),
                false => format!("{manifest_dir}/**"),
            },
        })
    }

    /// What each gating manifest came to, off the rows the step recorded.
    ///
    /// `rows` is the step's, one per Check in `checks`' order; any after them
    /// are the scope tier's and not a Check's.
    ///
    /// | Its Checks | Outcome |
    /// |---|---|
    /// | none on this step | did not run, not declared |
    /// | any did not pass | ran and failed |
    /// | any passed | ran and passed |
    /// | all skipped, none covered | did not run, path condition unmet |
    /// | all skipped, one covered | did not run, scope narrowed |
    pub(crate) fn outcomes(
        &self,
        checks: &[ResolvedCheck],
        rows: &[StepCheck],
    ) -> Vec<GateManifest> {
        self.gating
            .iter()
            .filter_map(|dir| {
                let id = self
                    .manifests
                    .iter()
                    .find(|(held, _)| held == dir)?
                    .1
                    .clone();
                let ours = |at: &usize| match &checks[*at] {
                    ResolvedCheck::ManifestCheck { manifest_dir, .. } => manifest_dir == dir,
                    _ => false,
                };
                let each: Vec<(&ResolvedCheck, CheckOutcome)> = (0..checks.len().min(rows.len()))
                    .filter(ours)
                    .map(|at| (&checks[at], rows[at].outcome))
                    .collect();
                let outcome = if each.is_empty() {
                    GateOutcome::DidNotRun(NotRunReason::NotDeclared)
                } else if each.iter().any(|(_, seen)| !seen.advances()) {
                    GateOutcome::RanAndFailed
                } else if each.iter().any(|(_, seen)| seen.passed()) {
                    GateOutcome::RanAndPassed
                } else if each
                    .iter()
                    .all(|(check, _)| !check.covers(self.owned_by(dir)))
                {
                    GateOutcome::DidNotRun(NotRunReason::PathConditionUnmet)
                } else {
                    GateOutcome::DidNotRun(NotRunReason::ScopeNarrowed)
                };
                Some(GateManifest {
                    manifest_id: id,
                    outcome,
                })
            })
            .collect()
    }
}

/// Where a Check in `dir` runs: the worktree for the root, and `dir` under it
/// for a workspace.
pub(crate) fn within(worktree: &Path, dir: &str) -> PathBuf {
    match dir.is_empty() {
        true => worktree.to_path_buf(),
        false => worktree.join(dir),
    }
}

/// The paths under `dir`, relative to it. The root's `dir` is empty and every
/// path is its own.
pub(crate) fn under(dir: &str, paths: &[String]) -> Vec<String> {
    match dir.is_empty() {
        true => paths.to_vec(),
        false => checks_runner::relative_to_dir(dir, paths).0,
    }
}

/// The paths a Check reads its `when` and its narrowing against: what its
/// manifest owns of the change where the manifests are known, and the paths
/// under its directory where they are not.
pub(crate) fn local(
    gated: Option<&Gated>,
    check: &ResolvedCheck,
    touched: &[String],
) -> Vec<String> {
    match gated {
        Some(gated) => gated.owned_by(check.manifest_dir()).to_vec(),
        None => under(check.manifest_dir(), touched),
    }
}

/// What the gate's change reached, as a Check in `dir` reads it.
pub(crate) fn reach_under<'a>(dir: &str, reach: &'a Reach) -> Cow<'a, Reach> {
    match reach {
        Reach::Paths(paths) if !dir.is_empty() => Cow::Owned(Reach::Paths(under(dir, paths))),
        _ => Cow::Borrowed(reach),
    }
}
